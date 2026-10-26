//! Pulsar engine: MIDI in -> instruments -> audio out, entirely off the UI thread.
//!
//! This crate must never depend on Tauri, so it can be run and tested as a plain CLI
//! (`cargo run -p pulsar-engine --example play -- <sounds dir>`).

mod audio;
mod event;
mod instrument;
pub mod instruments;
mod library;
mod midi_in;

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, mpsc};

use rtrb::{Producer, RingBuffer};
use serde::Serialize;

use audio::{AudioCommand, AudioHandle, AudioStats, Renderer};
pub use event::{CC_SUSTAIN, MidiEvent};
pub use instrument::Instrument;
pub use library::InstrumentInfo;
pub use midi_in::MidiDevice;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Where an event came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// A MIDI keyboard's playing port.
    Device,
    /// A controller's control port (e.g. Arturia "DAW" port): transport pads; never played.
    Control,
    /// The app itself: on-screen or computer keys.
    App,
}

/// What the engine reports back, for the UI to visualise.
#[derive(Clone, Debug)]
pub enum EngineEvent {
    Midi(MidiEvent, Source),
    Devices(Vec<MidiDevice>),
    InstrumentLoading(InstrumentInfo),
    InstrumentChanged(InstrumentInfo),
    InstrumentFailed {
        instrument: InstrumentInfo,
        error: String,
    },
}

#[derive(Clone, Debug, Serialize)]
pub struct Status {
    pub output_device: String,
    pub sample_rate: u32,
    pub latency_ms: f32,
    pub volume_db: f32,
    pub devices: Vec<MidiDevice>,
    pub instrument: Option<InstrumentInfo>,
    pub sounds_dir: PathBuf,
}

pub struct Engine {
    input: Arc<Mutex<Producer<MidiEvent>>>,
    commands: Mutex<Producer<AudioCommand>>,
    events: mpsc::Sender<EngineEvent>,
    sounds_dir: PathBuf,
    audio: AudioHandle,
    stats: Arc<AudioStats>,
    devices: Arc<Mutex<Vec<MidiDevice>>>,
    current: Mutex<Option<InstrumentInfo>>,
    volume_db: AtomicU32,
    loading: Mutex<()>,
    _housekeeping: mpsc::Sender<()>,
}

impl Engine {
    /// Opens the default audio output and starts watching for MIDI keyboards.
    /// No instrument is loaded yet: call [`Engine::load_instrument`].
    pub fn start(sounds_dir: PathBuf) -> Result<(Self, mpsc::Receiver<EngineEvent>), String> {
        let (input, input_rx) = RingBuffer::new(1024);
        let (commands, commands_rx) = RingBuffer::new(16);
        let (garbage, garbage_rx) = RingBuffer::new(16);
        let stats = Arc::new(AudioStats::default());
        let audio = audio::start(Renderer::new(input_rx, commands_rx, garbage, stats.clone()))?;

        let (events, events_rx) = mpsc::channel();
        let input = Arc::new(Mutex::new(input));
        let devices = Arc::new(Mutex::new(Vec::new()));
        let (housekeeping, housekeeping_stop) = mpsc::channel();
        midi_in::spawn_housekeeping(
            input.clone(),
            events.clone(),
            garbage_rx,
            devices.clone(),
            housekeeping_stop,
        );

        let engine = Self {
            input,
            commands: Mutex::new(commands),
            events,
            sounds_dir,
            audio,
            stats,
            devices,
            current: Mutex::new(None),
            volume_db: AtomicU32::new(0f32.to_bits()),
            loading: Mutex::new(()),
            _housekeeping: housekeeping,
        };
        Ok((engine, events_rx))
    }

    /// Play an event from a non-hardware source (on-screen or computer keyboard).
    pub fn send(&self, event: MidiEvent) {
        midi_in::dispatch(&self.input, &self.events, event, Source::App);
    }

    pub fn instruments(&self) -> Vec<InstrumentInfo> {
        library::scan(&self.sounds_dir)
    }

    /// Blocks while the instrument loads (a large SoundFont can take seconds), then swaps it in.
    pub fn load_instrument(&self, id: &str) -> Result<InstrumentInfo, String> {
        let info = self
            .instruments()
            .into_iter()
            .find(|i| i.id == id)
            .ok_or_else(|| format!("no instrument {id}"))?;
        let _one_at_a_time = self.loading.lock().map_err(|e| e.to_string())?;
        let _ = self
            .events
            .send(EngineEvent::InstrumentLoading(info.clone()));

        let result =
            library::load(&self.sounds_dir, id, self.audio.sample_rate).and_then(|instrument| {
                self.commands
                    .lock()
                    .map_err(|e| e.to_string())?
                    .push(AudioCommand::SetInstrument(instrument))
                    .map_err(|_| "audio thread is not keeping up".to_string())
            });
        match result {
            Ok(()) => {
                *self.current.lock().map_err(|e| e.to_string())? = Some(info.clone());
                let _ = self
                    .events
                    .send(EngineEvent::InstrumentChanged(info.clone()));
                Ok(info)
            }
            Err(error) => {
                let _ = self.events.send(EngineEvent::InstrumentFailed {
                    instrument: info,
                    error: error.clone(),
                });
                Err(error)
            }
        }
    }

    /// 0 dB is unity; -60 dB or lower is silence.
    pub fn set_volume_db(&self, db: f32) {
        let db = db.clamp(-60.0, 6.0);
        self.volume_db.store(db.to_bits(), Ordering::Relaxed);
        let gain = if db <= -60.0 {
            0.0
        } else {
            10f32.powf(db / 20.0)
        };
        if let Ok(mut commands) = self.commands.lock() {
            let _ = commands.push(AudioCommand::SetGain(gain));
        }
    }

    pub fn status(&self) -> Status {
        Status {
            output_device: self.audio.device.clone(),
            sample_rate: self.audio.sample_rate,
            latency_ms: self.stats.latency_us.load(Ordering::Relaxed) as f32 / 1000.0,
            volume_db: f32::from_bits(self.volume_db.load(Ordering::Relaxed)),
            devices: self.devices.lock().map(|d| d.clone()).unwrap_or_default(),
            instrument: self.current.lock().ok().and_then(|c| c.clone()),
            sounds_dir: self.sounds_dir.clone(),
        }
    }
}
