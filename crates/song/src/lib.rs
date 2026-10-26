//! Songs to practice: the notes, their timing and which track (a hand, a voice, a part) they belong to.
//! Pure data and parsing: no audio, no Tauri.

mod library;
mod midi;

pub use library::{Library, SongSummary};
pub use midi::from_midi;

use serde::{Deserialize, Serialize};

/// 2: notes belong to named tracks (version 1 had a left/right `hand` per note).
pub const FORMAT_VERSION: u32 = 2;

/// Shortest note kept, in beats; some files have zero-length notes.
const MIN_LENGTH: f64 = 1.0 / 32.0;

/// A part of the song you can choose to play or leave out: a hand, a melody, an accompaniment.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Track {
    pub name: String,
}

impl Track {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Note {
    pub key: u8,
    /// In beats (quarter notes) from the start of the song.
    pub start: f64,
    /// In beats.
    pub length: f64,
    pub velocity: u8,
    /// Index into `Song::tracks`.
    #[serde(default)]
    pub track: usize,
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
    /// At least one.
    #[serde(default)]
    pub tracks: Vec<Track>,
    /// Sorted by start, then key.
    pub notes: Vec<Note>,
}

impl Song {
    /// Reads a saved song, upgrading older formats.
    pub fn from_json(text: &str) -> Result<Self, String> {
        let mut value: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
        upgrade_hands(&mut value);
        let song: Song = serde_json::from_value(value).map_err(|e| e.to_string())?;
        Ok(song.normalized())
    }

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
        if self.tracks.is_empty() {
            self.tracks.push(Track::new("Piano"));
        }
        let last_track = self.tracks.len() - 1;
        for note in &mut self.notes {
            note.length = note.length.max(MIN_LENGTH);
            note.track = note.track.min(last_track);
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
}

/// Version 1 songs had `"hand": "left" | "right" | null` on each note and no tracks.
/// Hands become a "Right hand" and a "Left hand" track; songs without hands get one track.
fn upgrade_hands(song: &mut serde_json::Value) {
    if song.get("tracks").is_some() {
        return;
    }
    let Some(notes) = song.get_mut("notes").and_then(|n| n.as_array_mut()) else {
        return;
    };
    let hand =
        |note: &serde_json::Value| note.get("hand").and_then(|h| h.as_str()).map(str::to_owned);
    let has_hands = notes.iter().any(|n| hand(n).is_some());
    for note in notes.iter_mut() {
        let track = usize::from(hand(note).as_deref() == Some("left"));
        if let Some(note) = note.as_object_mut() {
            note.remove("hand");
            note.insert("track".into(), track.into());
        }
    }
    let tracks = if has_hands {
        vec![Track::new("Right hand"), Track::new("Left hand")]
    } else {
        vec![Track::new("Piano")]
    };
    song["tracks"] = serde_json::to_value(tracks).unwrap_or_default();
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
            tracks: vec![],
            notes: vec![Note {
                key: 60,
                start: 4.0,
                length: 0.0,
                velocity: 90,
                track: 3,
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
        // Always at least one track, and every note on one that exists.
        assert_eq!(s.tracks, vec![Track::new("Piano")]);
        assert_eq!(s.notes[0].track, 0);
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

    #[test]
    fn upgrades_hands_to_tracks() {
        let v1 = r#"{"version":1,"title":"t","tempos":[],"time_signatures":[],"notes":[
            {"key":48,"start":0,"length":1,"velocity":80,"hand":"left"},
            {"key":64,"start":0,"length":1,"velocity":80,"hand":"right"},
            {"key":67,"start":1,"length":1,"velocity":80,"hand":null}]}"#;
        let s = Song::from_json(v1).unwrap();
        assert_eq!(s.version, FORMAT_VERSION);
        assert_eq!(
            s.tracks,
            vec![Track::new("Right hand"), Track::new("Left hand")]
        );
        let tracks: Vec<_> = s.notes.iter().map(|n| (n.key, n.track)).collect();
        assert_eq!(tracks, vec![(48, 1), (64, 0), (67, 0)]);

        let plain = r#"{"version":1,"title":"t","tempos":[],"time_signatures":[],"notes":[
            {"key":60,"start":0,"length":1,"velocity":80,"hand":null}]}"#;
        assert_eq!(
            Song::from_json(plain).unwrap().tracks,
            vec![Track::new("Piano")]
        );
    }
}
