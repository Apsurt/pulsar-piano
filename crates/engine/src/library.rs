//! The instruments the user can pick: built-ins plus every .sf2 dropped into the sounds folder.

use std::path::Path;

use serde::Serialize;

use crate::Instrument;
use crate::instruments::{Pulse, SoundFontInstrument};

const PULSE_ID: &str = "builtin:pulse";
const SF2_PREFIX: &str = "sf2:";

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct InstrumentInfo {
    /// Stable id: `builtin:<name>` or `sf2:<file name>`.
    pub id: String,
    pub name: String,
    pub kind: &'static str,
}

pub fn scan(sounds_dir: &Path) -> Vec<InstrumentInfo> {
    let mut fonts: Vec<InstrumentInfo> = std::fs::read_dir(sounds_dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("sf2")))
        .filter_map(|p| {
            Some(InstrumentInfo {
                id: format!("{SF2_PREFIX}{}", p.file_name()?.to_str()?),
                name: p.file_stem()?.to_str()?.to_owned(),
                kind: "soundfont",
            })
        })
        .collect();
    fonts.sort_by_key(|i| i.name.to_lowercase());
    fonts.push(InstrumentInfo {
        id: PULSE_ID.into(),
        name: "Pulse".into(),
        kind: "builtin",
    });
    fonts
}

pub fn load(sounds_dir: &Path, id: &str, sample_rate: u32) -> Result<Box<dyn Instrument>, String> {
    if id == PULSE_ID {
        return Ok(Box::new(Pulse::new(sample_rate)));
    }
    let file = id
        .strip_prefix(SF2_PREFIX)
        .ok_or_else(|| format!("unknown instrument {id}"))?;
    // Ids come from the UI: never let one point outside the sounds folder.
    if file.contains(['/', '\\']) || file.starts_with('.') {
        return Err(format!("invalid instrument {id}"));
    }
    Ok(Box::new(SoundFontInstrument::load(
        &sounds_dir.join(file),
        sample_rate,
    )?))
}
