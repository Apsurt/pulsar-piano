//! MIDI input. midir can't report hot-plugging, so a housekeeping thread polls the port list every second
//! and connects to every input it finds. It also frees instruments the audio thread has swapped out.

use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use midir::{Ignore, MidiInput, MidiInputConnection};
use rtrb::{Consumer, Producer};
use serde::Serialize;

use crate::{EngineEvent, Instrument, MidiEvent, Source};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MidiDevice {
    pub name: String,
    /// False when the port exists but couldn't be opened, e.g. another app (a DAW) holds it on Windows.
    pub connected: bool,
}

/// The single entry point for every event, whatever its source: audio first, then the UI.
pub(crate) fn dispatch(
    input: &Mutex<Producer<MidiEvent>>,
    events: &mpsc::Sender<EngineEvent>,
    event: MidiEvent,
    source: Source,
) {
    if let Ok(mut queue) = input.lock() {
        let _ = queue.push(event);
    }
    let _ = events.send(EngineEvent::Midi(event, source));
}

pub(crate) fn spawn_housekeeping(
    input: Arc<Mutex<Producer<MidiEvent>>>,
    events: mpsc::Sender<EngineEvent>,
    mut garbage: Consumer<Box<dyn Instrument>>,
    devices: Arc<Mutex<Vec<MidiDevice>>>,
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
                        && let Some(conn) = connect(id, input.clone(), events.clone())
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
    input: Arc<Mutex<Producer<MidiEvent>>>,
    events: mpsc::Sender<EngineEvent>,
) -> Option<MidiInputConnection<()>> {
    let mut midi = MidiInput::new("Pulsar").ok()?;
    midi.ignore(Ignore::All);
    let port = midi.find_port_by_id(id)?;
    midi.connect(
        &port,
        "pulsar-in",
        move |_, bytes, _| {
            if let Some(event) = MidiEvent::parse(bytes) {
                dispatch(&input, &events, event, Source::Device);
            }
        },
        (),
    )
    .ok()
}
