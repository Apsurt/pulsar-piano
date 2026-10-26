//! MIDI input. midir can't report hot-plugging, so a housekeeping thread polls the port list every second
//! and connects to every input it finds. It also frees instruments the audio thread has swapped out.

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use midir::{Ignore, MidiInput, MidiInputConnection};
use rtrb::{Consumer, Producer};
use serde::Serialize;

use crate::{EngineEvent, Instrument, MidiEvent, Source, Touch};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MidiDevice {
    pub name: String,
    /// False when the port exists but couldn't be opened, e.g. another app (a DAW) holds it on Windows.
    pub connected: bool,
}

/// The single entry point for every event, whatever its source: audio first, then the UI.
/// Control-port events (transport pads and the like) never make sound.
pub(crate) fn dispatch(
    input: &Mutex<Producer<MidiEvent>>,
    events: &mpsc::Sender<EngineEvent>,
    event: MidiEvent,
    source: Source,
) {
    if source != Source::Control
        && let Ok(mut queue) = input.lock()
    {
        let _ = queue.push(event);
    }
    let _ = events.send(EngineEvent::Midi(event, source));
}

/// Controllers expose a separate "DAW" port for transport pads and host integration;
/// its messages are commands for the app, not notes to play.
fn is_control_port(name: &str) -> bool {
    name.to_lowercase().contains("daw")
}

pub(crate) fn spawn_housekeeping(
    input: Arc<Mutex<Producer<MidiEvent>>>,
    events: mpsc::Sender<EngineEvent>,
    mut garbage: Consumer<Box<dyn Instrument>>,
    devices: Arc<Mutex<Vec<MidiDevice>>>,
    touch: Arc<AtomicU8>,
    stop: mpsc::Receiver<()>,
) {
    let _ = thread::Builder::new()
        .name("pulsar-housekeeping".into())
        .spawn(move || {
            let watcher = MidiInput::new("Pulsar watcher").ok();
            let mut connections: Vec<(String, MidiInputConnection<()>)> = Vec::new();
            loop {
                while garbage.pop().is_ok() {}

                let ports: Vec<(String, String)> = watcher
                    .as_ref()
                    .map(|w| {
                        w.ports()
                            .iter()
                            .filter_map(|p| Some((w.port_name(p).ok()?, p.id())))
                            .collect()
                    })
                    .unwrap_or_default();
                connections.retain(|(name, _)| ports.iter().any(|(n, _)| n == name));
                for (name, id) in &ports {
                    if !connections.iter().any(|(n, _)| n == name)
                        && let Some(conn) = connect(
                            id,
                            is_control_port(name),
                            input.clone(),
                            events.clone(),
                            touch.clone(),
                        )
                    {
                        connections.push((name.clone(), conn));
                    }
                }

                let status: Vec<MidiDevice> = ports
                    .iter()
                    .map(|(name, _)| MidiDevice {
                        name: name.clone(),
                        connected: connections.iter().any(|(n, _)| n == name),
                    })
                    .collect();
                if let Ok(mut current) = devices.lock()
                    && *current != status
                {
                    *current = status.clone();
                    let _ = events.send(EngineEvent::Devices(status));
                }

                match stop.recv_timeout(Duration::from_secs(1)) {
                    Err(RecvTimeoutError::Timeout) => {}
                    _ => break,
                }
            }
        });
}

fn connect(
    id: &str,
    control: bool,
    input: Arc<Mutex<Producer<MidiEvent>>>,
    events: mpsc::Sender<EngineEvent>,
    touch: Arc<AtomicU8>,
) -> Option<MidiInputConnection<()>> {
    let mut midi = MidiInput::new("Pulsar").ok()?;
    midi.ignore(Ignore::All);
    let port = midi.find_port_by_id(id)?;
    let source = if control {
        Source::Control
    } else {
        Source::Device
    };
    midi.connect(
        &port,
        "pulsar-in",
        move |_, bytes, _| {
            if let Some(mut event) = MidiEvent::parse(bytes) {
                if let MidiEvent::NoteOn { velocity, .. } = &mut event
                    && source == Source::Device
                {
                    *velocity = Touch::from_u8(touch.load(Ordering::Relaxed)).apply(*velocity);
                }
                dispatch(&input, &events, event, source);
            }
        },
        (),
    )
    .ok()
}
