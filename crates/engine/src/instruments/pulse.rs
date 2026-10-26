//! "Pulse": a small built-in electric-piano-ish synth, so the app makes sound with no files installed.

use std::f32::consts::TAU;

use crate::{Instrument, MidiEvent, event::CC_SUSTAIN};

const VOICES: usize = 32;
/// (frequency ratio, level, decay-speed multiplier) per partial.
const PARTIALS: [(f32, f32, f32); 4] = [
    (1.0, 1.0, 1.0),
    (2.0, 0.45, 1.8),
    (3.0, 0.18, 2.6),
    (4.01, 0.08, 3.5),
];

#[derive(Clone, Copy, Default)]
struct Voice {
    key: u8,
    active: bool,
    held: bool,
    phase: [f32; PARTIALS.len()],
    step: [f32; PARTIALS.len()],
    level: [f32; PARTIALS.len()],
    decay: [f32; PARTIALS.len()],
    attack: f32,
    release: f32,
    pan: (f32, f32),
}

pub struct Pulse {
    voices: [Voice; VOICES],
    sample_rate: f32,
    sustain: bool,
}

impl Pulse {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            voices: [Voice::default(); VOICES],
            sample_rate: sample_rate as f32,
            sustain: false,
        }
    }

    fn note_on(&mut self, key: u8, velocity: u8) {
        // Reuse the voice already on this key, else a free one, else steal the quietest.
        let slot = self
            .voices
            .iter()
            .position(|v| v.active && v.key == key)
            .or_else(|| self.voices.iter().position(|v| !v.active))
            .unwrap_or_else(|| {
                (0..VOICES)
                    .min_by(|&a, &b| self.voices[a].level[0].total_cmp(&self.voices[b].level[0]))
                    .unwrap()
            });

        let freq = 440.0 * 2f32.powf((key as f32 - 69.0) / 12.0);
        let vel = velocity as f32 / 127.0;
        // Higher notes and harder hits ring shorter / brighter, like a struck string.
        let seconds = (4.5 - (key as f32 - 21.0) / 88.0 * 3.5).max(0.6);
        let pan = 0.5 + (key as f32 - 64.0) / 140.0;
        let mut v = Voice {
            key,
            active: true,
            held: true,
            attack: 0.0,
            release: 1.0,
            pan: ((1.0 - pan).sqrt(), pan.sqrt()),
            ..Voice::default()
        };
        for (i, (ratio, level, speed)) in PARTIALS.iter().enumerate() {
            v.step[i] = freq * ratio / self.sample_rate;
            let brightness = if i == 0 { 1.0 } else { 0.35 + 0.65 * vel };
            v.level[i] = level * brightness * vel * vel * 0.25;
            v.decay[i] = (-speed / (seconds * self.sample_rate)).exp();
        }
        self.voices[slot] = v;
    }

    fn note_off(&mut self, key: u8) {
        for v in self.voices.iter_mut().filter(|v| v.active && v.key == key) {
            v.held = false;
        }
    }
}

impl Instrument for Pulse {
    fn handle(&mut self, event: MidiEvent) {
        match event {
            MidiEvent::NoteOn { key, velocity, .. } => self.note_on(key, velocity),
            MidiEvent::NoteOff { key, .. } => self.note_off(key),
            MidiEvent::ControlChange {
                controller: CC_SUSTAIN,
                value,
                ..
            } => self.sustain = value >= 64,
            MidiEvent::ControlChange {
                controller: 120 | 123,
                ..
            } => self.voices = [Voice::default(); VOICES],
            _ => {}
        }
    }

    fn render(&mut self, left: &mut [f32], right: &mut [f32]) {
        left.fill(0.0);
        right.fill(0.0);
        let attack_step = 1.0 / (0.002 * self.sample_rate);
        let release_mul = (-1.0 / (0.12 * self.sample_rate)).exp();
        for v in self.voices.iter_mut().filter(|v| v.active) {
            let releasing = !v.held && !self.sustain;
            for (l, r) in left.iter_mut().zip(right.iter_mut()) {
                v.attack = (v.attack + attack_step).min(1.0);
                if releasing {
                    v.release *= release_mul;
                }
                let mut s = 0.0;
                for i in 0..PARTIALS.len() {
                    s += (v.phase[i] * TAU).sin() * v.level[i];
                    v.phase[i] = (v.phase[i] + v.step[i]).fract();
                    v.level[i] *= v.decay[i];
                }
                s *= v.attack * v.release;
                *l += s * v.pan.0;
                *r += s * v.pan.1;
            }
            if v.release < 1e-4 || v.level[0] < 1e-5 {
                v.active = false;
            }
        }
    }
}
