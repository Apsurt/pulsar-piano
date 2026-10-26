//! Writes a two-hand demo MIDI file (Ode to Joy, public domain) and optionally imports it:
//! `cargo run -p pulsar-song --example demo -- <out.mid> [songs dir to import into]`

use std::path::PathBuf;

use midly::{Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind};
use pulsar_song::Library;

const TICKS: u32 = 480;

/// (MIDI key, length in beats), played one after another; one line per bar.
#[rustfmt::skip]
const MELODY: &[(u8, f32)] = &[
    (64, 1.0), (64, 1.0), (65, 1.0), (67, 1.0),
    (67, 1.0), (65, 1.0), (64, 1.0), (62, 1.0),
    (60, 1.0), (60, 1.0), (62, 1.0), (64, 1.0),
    (64, 1.5), (62, 0.5), (62, 2.0),
    (64, 1.0), (64, 1.0), (65, 1.0), (67, 1.0),
    (67, 1.0), (65, 1.0), (64, 1.0), (62, 1.0),
    (60, 1.0), (60, 1.0), (62, 1.0), (64, 1.0),
    (62, 1.5), (60, 0.5), (60, 2.0),
];
#[rustfmt::skip]
const BASS: &[(u8, f32)] = &[
    (48, 4.0), (43, 4.0), (48, 4.0), (43, 4.0),
    (48, 4.0), (43, 4.0), (48, 4.0), (43, 2.0), (48, 2.0),
];

fn part(channel: u8, notes: &[(u8, f32)], velocity: u8) -> Vec<TrackEvent<'static>> {
    let mut events = Vec::new();
    let mut gap = 0;
    for &(key, beats) in notes {
        let length = (beats * TICKS as f32) as u32;
        let held = length * 9 / 10;
        let midi = |message| TrackEventKind::Midi {
            channel: channel.into(),
            message,
        };
        events.push(TrackEvent {
            delta: gap.into(),
            kind: midi(MidiMessage::NoteOn {
                key: key.into(),
                vel: velocity.into(),
            }),
        });
        events.push(TrackEvent {
            delta: held.into(),
            kind: midi(MidiMessage::NoteOff {
                key: key.into(),
                vel: 0.into(),
            }),
        });
        gap = length - held;
    }
    events.push(TrackEvent {
        delta: 0.into(),
        kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
    });
    events
}

fn main() {
    let mut args = std::env::args().skip(1);
    let out = PathBuf::from(args.next().unwrap_or_else(|| "Ode to Joy.mid".into()));

    let mut smf = Smf::new(Header::new(
        Format::Parallel,
        Timing::Metrical((TICKS as u16).into()),
    ));
    smf.tracks.push(vec![
        TrackEvent {
            delta: 0.into(),
            kind: TrackEventKind::Meta(MetaMessage::Tempo(600_000.into())),
        },
        TrackEvent {
            delta: 0.into(),
            kind: TrackEventKind::Meta(MetaMessage::TimeSignature(4, 2, 24, 8)),
        },
        TrackEvent {
            delta: 0.into(),
            kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
        },
    ]);
    smf.tracks.push(part(0, MELODY, 96));
    smf.tracks.push(part(1, BASS, 72));
    smf.save(&out).expect("could not write the MIDI file");
    println!("wrote {}", out.display());

    if let Some(dir) = args.next() {
        let library = Library::open(PathBuf::from(dir)).expect("could not open the songs folder");
        let song = library.import(&out).expect("import failed");
        println!(
            "imported as {} ({} notes, {:.0} s)",
            song.id, song.notes, song.seconds
        );
    }
}
