use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;

use rustysynth::{SoundFont, Synthesizer, SynthesizerSettings};

use crate::{Instrument, MidiEvent};

/// An .sf2 file played through rustysynth.
pub struct SoundFontInstrument {
    synth: Synthesizer,
}

impl SoundFontInstrument {
    /// Reads the whole file into memory, so call it off the audio thread.
    pub fn load(path: &Path, sample_rate: u32) -> Result<Self, String> {
        let file = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let font = SoundFont::new(&mut BufReader::new(file)).map_err(|e| format!("{e:?}"))?;
        let font = Arc::new(font);
        let mut settings = SynthesizerSettings::new(sample_rate as i32);
        settings.maximum_polyphony = 128;
        let mut synth = Synthesizer::new(&font, &settings).map_err(|e| format!("{e:?}"))?;

        // Play the font's first melodic preset rather than assuming bank 0 / program 0 exists.
        if let Some(preset) = font
            .get_presets()
            .iter()
            .filter(|p| p.get_bank_number() < 128)
            .min_by_key(|p| (p.get_bank_number(), p.get_patch_number()))
        {
            synth.process_midi_message(0, 0xB0, 0x00, preset.get_bank_number());
            synth.process_midi_message(0, 0xC0, preset.get_patch_number(), 0);
        }
        Ok(Self { synth })
    }
}

impl Instrument for SoundFontInstrument {
    fn handle(&mut self, event: MidiEvent) {
        // Everything plays on channel 0 so any keyboard, whatever channel it sends on, hits the loaded preset.
        let [status, d1, d2] = event.to_bytes();
        self.synth
            .process_midi_message(0, (status & 0xF0) as i32, d1 as i32, d2 as i32);
    }

    fn render(&mut self, left: &mut [f32], right: &mut [f32]) {
        self.synth.render(left, right);
    }
}
