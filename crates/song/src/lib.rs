//! Songs to practice: the notes, their timing and which hand plays them.
//! Pure data and parsing: no audio, no Tauri.

mod library;
mod midi;

pub use library::{Library, SongSummary};
pub use midi::from_midi;

use serde::{Deserialize, Serialize};

pub const FORMAT_VERSION: u32 = 1;

/// Shortest note kept, in beats; some files have zero-length notes.
const MIN_LENGTH: f64 = 1.0 / 32.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Hand {
    Left,
    Right,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Note {
    pub key: u8,
    /// In beats (quarter notes) from the start of the song.
    pub start: f64,
    /// In beats.
    pub length: f64,
    pub velocity: u8,
    pub hand: Option<Hand>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TempoChange {
    pub beat: f64,
    pub bpm: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TimeSignature {
    pub beat: f64,
    pub numerator: u8,
    pub denominator: u8,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Song {
    pub version: u32,
    pub title: String,
    #[serde(default)]
    pub artist: String,
    /// Sorted by beat; the first is at beat 0.
    pub tempos: Vec<TempoChange>,
    /// Sorted by beat; the first is at beat 0.
    pub time_signatures: Vec<TimeSignature>,
    /// Sorted by start, then key.
    pub notes: Vec<Note>,
}

impl Song {
    /// Sorts everything and fills in the MIDI defaults (120 bpm, 4/4) where a song starts without them.
    pub fn normalized(mut self) -> Self {
        self.version = FORMAT_VERSION;
        self.tempos.sort_by(|a, b| a.beat.total_cmp(&b.beat));
        self.tempos.dedup_by(|later, earlier| {
            later.beat == earlier.beat && {
                earlier.bpm = later.bpm;
                true
            }
        });
        if self.tempos.first().is_none_or(|t| t.beat > 0.0) {
            self.tempos.insert(
                0,
                TempoChange {
                    beat: 0.0,
                    bpm: 120.0,
                },
            );
        }
        self.time_signatures
            .sort_by(|a, b| a.beat.total_cmp(&b.beat));
        if self.time_signatures.first().is_none_or(|t| t.beat > 0.0) {
            self.time_signatures.insert(
                0,
                TimeSignature {
                    beat: 0.0,
                    numerator: 4,
                    denominator: 4,
                },
            );
        }
        for note in &mut self.notes {
            note.length = note.length.max(MIN_LENGTH);
        }
        self.notes
            .sort_by(|a, b| a.start.total_cmp(&b.start).then(a.key.cmp(&b.key)));
        self
    }

    /// The beat where the last note ends.
    pub fn end_beat(&self) -> f64 {
        self.notes
            .iter()
            .map(|n| n.start + n.length)
            .fold(0.0, f64::max)
    }

    /// Seconds from the start of the song to `beat`, following tempo changes.
    pub fn seconds_at(&self, beat: f64) -> f64 {
        let mut seconds = 0.0;
        for (i, tempo) in self.tempos.iter().enumerate() {
            if beat <= tempo.beat {
                break;
            }
            let next = self.tempos.get(i + 1).map_or(f64::INFINITY, |t| t.beat);
            seconds += (beat.min(next) - tempo.beat) * 60.0 / tempo.bpm;
        }
        seconds
    }

    pub fn has_hands(&self) -> bool {
        self.notes.iter().any(|n| n.hand.is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn song(tempos: Vec<TempoChange>) -> Song {
        Song {
            version: 0,
            title: "t".into(),
            artist: String::new(),
            tempos,
            time_signatures: vec![],
            notes: vec![Note {
                key: 60,
                start: 4.0,
                length: 0.0,
                velocity: 90,
                hand: None,
            }],
        }
        .normalized()
    }

    #[test]
    fn defaults_to_120_bpm_in_four_four() {
        let s = song(vec![]);
        assert_eq!(
            s.tempos,
            vec![TempoChange {
                beat: 0.0,
                bpm: 120.0
            }]
        );
        assert_eq!(
            (
                s.time_signatures[0].numerator,
                s.time_signatures[0].denominator
            ),
            (4, 4)
        );
        assert_eq!(s.seconds_at(4.0), 2.0);
        assert!(s.notes[0].length > 0.0);
    }

    #[test]
    fn follows_tempo_changes() {
        // 2 beats at 120 bpm (1 s), then 60 bpm (1 s per beat).
        let s = song(vec![
            TempoChange {
                beat: 0.0,
                bpm: 120.0,
            },
            TempoChange {
                beat: 2.0,
                bpm: 60.0,
            },
        ]);
        assert_eq!(s.seconds_at(2.0), 1.0);
        assert_eq!(s.seconds_at(5.0), 4.0);
    }
}
