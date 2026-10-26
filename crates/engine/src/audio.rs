//! The audio thread. The cpal callback only pops lock-free queues and renders: no locks, no allocation.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{BufferSize, FromSample, SampleFormat, SizedSample, StreamConfig, SupportedBufferSize};
use rtrb::{Consumer, Producer};

use crate::{Instrument, MidiEvent};

pub(crate) enum AudioCommand {
    SetInstrument(Box<dyn Instrument>),
    SetGain(f32),
}

#[derive(Default)]
pub(crate) struct AudioStats {
    /// Time from the callback to the samples reaching the speakers.
    pub latency_us: AtomicU32,
}

const MAX_BLOCK: usize = 512;

pub(crate) struct Renderer {
    instrument: Option<Box<dyn Instrument>>,
    events: Consumer<MidiEvent>,
    commands: Consumer<AudioCommand>,
    /// Replaced instruments go back out to be dropped off the audio thread.
    garbage: Producer<Box<dyn Instrument>>,
    gain: f32,
    target_gain: f32,
    left: [f32; MAX_BLOCK],
    right: [f32; MAX_BLOCK],
    stats: Arc<AudioStats>,
}

impl Renderer {
    pub fn new(
        events: Consumer<MidiEvent>,
        commands: Consumer<AudioCommand>,
        garbage: Producer<Box<dyn Instrument>>,
        stats: Arc<AudioStats>,
    ) -> Self {
        Self {
            instrument: None,
            events,
            commands,
            garbage,
            gain: 1.0,
            target_gain: 1.0,
            left: [0.0; MAX_BLOCK],
            right: [0.0; MAX_BLOCK],
            stats,
        }
    }

    fn process<T: SizedSample + FromSample<f32>>(&mut self, out: &mut [T], channels: usize) {
        while let Ok(command) = self.commands.pop() {
            match command {
                AudioCommand::SetInstrument(new) => {
                    if let Some(old) = self.instrument.replace(new) {
                        // The queue is drained every second; if it is somehow full, leak rather than free here.
                        if let Err(rtrb::PushError::Full(old)) = self.garbage.push(old) {
                            std::mem::forget(old);
                        }
                    }
                }
                AudioCommand::SetGain(gain) => self.target_gain = gain,
            }
        }
        while let Ok(event) = self.events.pop() {
            if let Some(instrument) = self.instrument.as_mut() {
                instrument.handle(event);
            }
        }

        for chunk in out.chunks_mut(MAX_BLOCK * channels) {
            let frames = chunk.len() / channels;
            let (left, right) = (&mut self.left[..frames], &mut self.right[..frames]);
            match self.instrument.as_mut() {
                Some(instrument) => instrument.render(left, right),
                None => {
                    left.fill(0.0);
                    right.fill(0.0);
                }
            }
            for (f, frame) in chunk.chunks_exact_mut(channels).enumerate() {
                // Glide towards the target so volume changes don't click.
                self.gain += (self.target_gain - self.gain) * 0.002;
                let l = (left[f] * self.gain).clamp(-1.0, 1.0);
                let r = (right[f] * self.gain).clamp(-1.0, 1.0);
                if channels == 1 {
                    frame[0] = T::from_sample((l + r) * 0.5);
                    continue;
                }
                for (c, sample) in frame.iter_mut().enumerate() {
                    *sample = T::from_sample(match c {
                        0 => l,
                        1 => r,
                        _ => 0.0,
                    });
                }
            }
        }
    }
}

pub(crate) struct AudioHandle {
    pub sample_rate: u32,
    pub device: String,
    /// Dropping this stops the stream.
    _stop: mpsc::Sender<()>,
}

/// Opens the default output device. The stream lives on its own thread because it is not `Send` on every platform.
pub(crate) fn start(renderer: Renderer) -> Result<AudioHandle, String> {
    let (ready_tx, ready_rx) = mpsc::channel();
    let (stop_tx, stop_rx) = mpsc::channel::<()>();
    thread::Builder::new()
        .name("pulsar-audio".into())
        .spawn(move || match open(renderer) {
            Ok((stream, sample_rate, device)) => {
                let _ = ready_tx.send(Ok((sample_rate, device)));
                let _ = stop_rx.recv();
                drop(stream);
            }
            Err(e) => {
                let _ = ready_tx.send(Err(e));
            }
        })
        .map_err(|e| e.to_string())?;
    let (sample_rate, device) = ready_rx
        .recv()
        .map_err(|_| "audio thread exited".to_string())??;
    Ok(AudioHandle {
        sample_rate,
        device,
        _stop: stop_tx,
    })
}

fn open(renderer: Renderer) -> Result<(cpal::Stream, u32, String), String> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or("no audio output device")?;
    let name = device
        .description()
        .map(|d| d.name().to_owned())
        .unwrap_or_else(|_| "Audio output".into());
    let supported = device.default_output_config().map_err(|e| e.to_string())?;
    let mut config = supported.config();
    // Left at Default, WASAPI picks a large buffer. Ask for a small one, within what the device allows.
    if let SupportedBufferSize::Range { min, max } = *supported.buffer_size() {
        config.buffer_size = BufferSize::Fixed(LOW_LATENCY_FRAMES.clamp(min, max));
    }
    eprintln!(
        "audio: {name} @ {} Hz, buffer {:?} (device allows {:?})",
        config.sample_rate,
        config.buffer_size,
        supported.buffer_size()
    );
    let sample_rate = config.sample_rate;
    let renderer = Arc::new(Mutex::new(Some(renderer)));
    let stream = build_any(&device, supported.sample_format(), config, &renderer).or_else(|e| {
        eprintln!("low-latency buffer refused ({e}), using the device default");
        config.buffer_size = BufferSize::Default;
        build_any(&device, supported.sample_format(), config, &renderer)
    })?;
    stream.play().map_err(|e| e.to_string())?;
    Ok((stream, sample_rate, name))
}

/// Frames per callback we ask for: about 5.8 ms at 44.1 kHz.
const LOW_LATENCY_FRAMES: u32 = 256;

/// The renderer sits in a slot so a failed attempt can hand it back for the next one.
type RendererSlot = Arc<Mutex<Option<Renderer>>>;

fn build_any(
    device: &cpal::Device,
    format: SampleFormat,
    config: StreamConfig,
    slot: &RendererSlot,
) -> Result<cpal::Stream, String> {
    match format {
        SampleFormat::F32 => build::<f32>(device, config, slot),
        SampleFormat::I16 => build::<i16>(device, config, slot),
        SampleFormat::U16 => build::<u16>(device, config, slot),
        SampleFormat::I32 => build::<i32>(device, config, slot),
        other => Err(format!("unsupported sample format {other:?}")),
    }
}

fn build<T: SizedSample + FromSample<f32>>(
    device: &cpal::Device,
    config: StreamConfig,
    slot: &RendererSlot,
) -> Result<cpal::Stream, String> {
    let channels = config.channels as usize;
    // Taken out of the slot on the first callback, so the callback itself never locks.
    let slot = slot.clone();
    let mut renderer: Option<Renderer> = None;
    device
        .build_output_stream::<T, _, _>(
            config,
            move |out: &mut [T], info: &cpal::OutputCallbackInfo| {
                if renderer.is_none() {
                    renderer = slot.try_lock().ok().and_then(|mut s| s.take());
                }
                let Some(renderer) = renderer.as_mut() else {
                    out.fill(T::EQUILIBRIUM);
                    return;
                };
                let ts = info.timestamp();
                let latency = ts.playback.duration_since(ts.callback);
                renderer
                    .stats
                    .latency_us
                    .store(latency.as_micros() as u32, Ordering::Relaxed);
                renderer.process(out, channels);
            },
            |e| eprintln!("audio stream error: {e}"),
            None,
        )
        .map_err(|e| e.to_string())
}
