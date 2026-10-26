//! Standard MIDI File import.

use std::collections::HashMap;

use midly::{MetaMessage, MidiMessage, Smf, Timing, TrackEventKind};

use crate::{FORMAT_VERSION, Hand, Note, Song, TempoChange, TimeSignature};

/// General MIDI drums live on channel 10 (index 9); they aren't piano notes.
const DRUM_CHANNEL: u8 = 9;

/// Reads a .mid file. With exactly two note-carrying parts (tracks, or channels within a track),
/// the lower-pitched part becomes the left hand and the other the right.
pub fn from_midi(bytes: &[u8], title: &str) -> Result<Song, String> {
    let smf = Smf::parse(bytes).map_err(|e| format!("not a readable MIDI file ({e})"))?;
    let ppq = match smf.header.timing {
        Timing::Metrical(t) => t.as_int().max(1) as f64,
        Timing::Timecode(..) => {
            return Err("MIDI files timed in SMPTE frames aren't supported".into());
        }
    };

    let mut tempos = Vec::new();
    let mut time_signatures = Vec::new();
    let mut parts: HashMap<(usize, u8), Vec<Note>> = HashMap::new();

    for (track_index, track) in smf.tracks.iter().enumerate() {
        let mut tick = 0u64;
        let mut sounding: HashMap<(u8, u8), (u64, u8)> = HashMap::new();
        for event in track {
            tick += event.delta.as_int() as u64;
            let beat = tick as f64 / ppq;
            match event.kind {
                TrackEventKind::Meta(MetaMessage::Tempo(us)) => tempos.push(TempoChange {
                    beat,
                    bpm: 60_000_000.0 / us.as_int().max(1) as f64,
                }),
                TrackEventKind::Meta(MetaMessage::TimeSignature(numerator, power, _, _)) => {
                    time_signatures.push(TimeSignature {
                        beat,
                        numerator: numerator.max(1),
                        denominator: 1u8.checked_shl(power as u32).unwrap_or(4),
                    })
                }
                TrackEventKind::Midi { channel, message } => {
                    let channel = channel.as_int();
                    let (key, velocity) = match message {
                        MidiMessage::NoteOn { key, vel } => (key.as_int(), vel.as_int()),
                        MidiMessage::NoteOff { key, .. } => (key.as_int(), 0),
                        _ => continue,
                    };
                    if channel == DRUM_CHANNEL {
                        continue;
                    }
                    // A note-off, or a new note-on on a key that is still sounding, ends the note.
                    if let Some((start, on_velocity)) = sounding.remove(&(channel, key)) {
                        parts.entry((track_index, channel)).or_default().push(note(
                            key,
                            start,
                            tick,
                            on_velocity,
                            ppq,
                        ));
                    }
                    if velocity > 0 {
                        sounding.insert((channel, key), (tick, velocity));
                    }
                }
                _ => {}
            }
        }
        // Notes never released end with their track.
        for ((channel, key), (start, velocity)) in sounding {
            parts
                .entry((track_index, channel))
                .or_default()
                .push(note(key, start, tick, velocity, ppq));
        }
    }

    let mut parts: Vec<Vec<Note>> = parts.into_values().collect();
    if parts.len() == 2 {
        parts.sort_by(|a, b| average_key(a).total_cmp(&average_key(b)));
        for (part, hand) in parts.iter_mut().zip([Hand::Left, Hand::Right]) {
            part.iter_mut().for_each(|n| n.hand = Some(hand));
        }
    }
    let notes: Vec<Note> = parts.into_iter().flatten().collect();
    if notes.is_empty() {
        return Err("this MIDI file has no notes to play".into());
    }

    Ok(Song {
        version: FORMAT_VERSION,
        title: title.to_owned(),
        artist: String::new(),
        tempos,
        time_signatures,
        notes,
    }
    .normalized())
}

fn note(key: u8, start: u64, end: u64, velocity: u8, ppq: f64) -> Note {
    Note {
        key,
        start: start as f64 / ppq,
        length: end.saturating_sub(start) as f64 / ppq,
        velocity,
        hand: None,
    }
}

fn average_key(notes: &[Note]) -> f64 {
    notes.iter().map(|n| n.key as f64).sum::<f64>() / notes.len().max(1) as f64
}

#[cfg(test)]
pub(crate) mod tests {
    use midly::{Format, Header, TrackEvent};

    use super::*;

    pub(crate) fn on(delta: u32, channel: u8, key: u8, vel: u8) -> TrackEvent<'static> {
        TrackEvent {
            delta: delta.into(),
            kind: TrackEventKind::Midi {
                channel: channel.into(),
                message: MidiMessage::NoteOn {
                    key: key.into(),
                    vel: vel.into(),
                },
            },
        }
    }

    pub(crate) fn meta(delta: u32, message: MetaMessage<'static>) -> TrackEvent<'static> {
        TrackEvent {
            delta: delta.into(),
            kind: TrackEventKind::Meta(message),
        }
    }

    /// A Standard MIDI File at 480 ticks per beat.
    pub(crate) fn smf(tracks: Vec<Vec<TrackEvent<'static>>>) -> Vec<u8> {
        let mut smf = Smf::new(Header::new(Format::Parallel, Timing::Metrical(480.into())));
        smf.tracks = tracks
            .into_iter()
            .map(|mut t| {
                t.push(meta(0, MetaMessage::EndOfTrack));
                t
            })
            .collect();
        let mut bytes = Vec::new();
        smf.write_std(&mut bytes).unwrap();
        bytes
    }

    #[test]
    fn imports_two_hands_tempo_and_meter() {
        let bytes = smf(vec![
            vec![
                meta(0, MetaMessage::Tempo(600_000.into())),
                meta(0, MetaMessage::TimeSignature(3, 2, 24, 8)),
            ],
            // Right hand: E4 for one beat, then G4 (note-on with velocity 0 as note-off).
            vec![
                on(0, 0, 64, 100),
                on(480, 0, 64, 0),
                on(0, 0, 67, 90),
                on(240, 0, 67, 0),
            ],
            // Left hand: C3 for two beats, plus a drum hit that must be ignored.
            vec![
                on(0, 1, 48, 80),
                on(0, DRUM_CHANNEL, 36, 127),
                on(960, 1, 48, 0),
            ],
        ]);
        let song = from_midi(&bytes, "Test").unwrap();

        assert_eq!(
            song.tempos,
            vec![TempoChange {
                beat: 0.0,
                bpm: 100.0
            }]
        );
        assert_eq!(
            (
                song.time_signatures[0].numerator,
                song.time_signatures[0].denominator
            ),
            (3, 4)
        );
        let notes: Vec<_> = song
            .notes
            .iter()
            .map(|n| (n.key, n.start, n.length, n.hand))
            .collect();
        assert_eq!(
            notes,
            vec![
                (48, 0.0, 2.0, Some(Hand::Left)),
                (64, 0.0, 1.0, Some(Hand::Right)),
                (67, 1.0, 0.5, Some(Hand::Right)),
            ]
        );
    }

    #[test]
    fn retriggered_key_ends_previous_note() {
        let bytes = smf(vec![vec![
            on(0, 0, 60, 100),
            on(240, 0, 60, 100),
            on(240, 0, 60, 0),
        ]]);
        let song = from_midi(&bytes, "t").unwrap();
        let spans: Vec<_> = song.notes.iter().map(|n| (n.start, n.length)).collect();
        assert_eq!(spans, vec![(0.0, 0.5), (0.5, 0.5)]);
        assert!(!song.has_hands());
    }

    #[test]
    fn rejects_files_without_notes_or_midi() {
        assert!(
            from_midi(
                &smf(vec![vec![meta(0, MetaMessage::Tempo(500_000.into()))]]),
                "t"
            )
            .is_err()
        );
        assert!(from_midi(b"not midi", "t").is_err());
    }
}
