// Hooktheory's "Copy" clipboard: a melody as scale degrees and chords as Roman numerals, in a key.
// Turned into a song with a Melody track and a Chords track (a simple left-hand voicing).
import type { Song, SongNote } from "./songs";

type HtNote = { beat: number; duration: number; sd: string; octave: number; isRest?: boolean };
type HtChord = {
  beat: number;
  duration: number;
  /** Scale degree 1–7 of the chord's root (of the chord it leads to, for applied chords). */
  root: number;
  /** 5 triad, 7 seventh, 9, 11, 13. */
  type: number;
  inversion?: number;
  /** Secondary chords: 5 is V/x, 7 is vii°/x and so on, with `root` the x. */
  applied?: number;
  adds?: number[];
  omits?: number[];
  alterations?: string[];
  suspensions?: number[];
  /** A mode the chord is borrowed from, like "minor". */
  borrowed?: unknown;
  isRest?: boolean;
};
type HtKey = { beat: number; scale: string; tonic: string };
type Ticked = { startTick: number; durationTicks?: number };

/** Beats are 1-based quarter notes; the newer tick format has 480 ticks per beat. */
const TICKS_PER_BEAT = 480;
const MELODY_VELOCITY = 90;
const CHORD_VELOCITY = 70;
/** Chord roots sit from G2 up, below most melodies. */
const CHORD_FLOOR = 43;

const MAJOR = [0, 2, 4, 5, 7, 9, 11];
const HARMONIC_MINOR = [0, 2, 3, 5, 7, 8, 11];
const SCALES: Record<string, number[]> = {
  major: MAJOR,
  ionian: MAJOR,
  dorian: [0, 2, 3, 5, 7, 9, 10],
  phrygian: [0, 1, 3, 5, 7, 8, 10],
  lydian: [0, 2, 4, 6, 7, 9, 11],
  mixolydian: [0, 2, 4, 5, 7, 9, 10],
  minor: [0, 2, 3, 5, 7, 8, 10],
  aeolian: [0, 2, 3, 5, 7, 8, 10],
  locrian: [0, 1, 3, 5, 6, 8, 10],
  harmonicminor: HARMONIC_MINOR,
  phrygiandominant: [0, 1, 4, 5, 7, 8, 10],
};
const LETTERS: Record<string, number> = { C: 0, D: 2, E: 4, F: 5, G: 7, A: 9, B: 11 };

export class HooktheoryError extends Error {}

export type HooktheoryImport = {
  song: Song;
  melody: number;
  chords: number;
  /** Like "A minor". */
  key: string;
  bars: number;
};

export function fromHooktheory(text: string, title: string, artist: string, bpm: number): HooktheoryImport {
  let data: Record<string, unknown>;
  try {
    data = JSON.parse(text);
  } catch {
    throw new HooktheoryError("That isn't Hooktheory's copied data. Click Copy on the Hooktheory page, then paste here.");
  }
  const payload = (data.modern as { payload?: Record<string, unknown> } | undefined)?.payload ?? {};
  const notes = (data.notes as HtNote[] | undefined) ?? fromTicks<HtNote>(payload.notes);
  const chords = (data.chords as HtChord[] | undefined) ?? fromTicks<HtChord>(payload.chords);
  const keys = [...((data.keys as HtKey[] | undefined) ?? fromTicks<HtKey>(payload.keys))].sort((a, b) => a.beat - b.beat);
  if (!Array.isArray(notes) || !Array.isArray(chords)) throw new HooktheoryError("No notes or chords in the pasted data.");
  const keyAt = (beat: number): HtKey =>
    keys.filter((k) => k.beat <= beat).at(-1) ?? keys[0] ?? { beat: 1, scale: "major", tonic: "C" };

  const melody: SongNote[] = notes
    .filter((n) => !n.isRest)
    .map((n) => ({
      key: degreePitch(n.sd, n.octave, keyAt(n.beat)),
      start: n.beat - 1,
      length: n.duration,
      velocity: MELODY_VELOCITY,
      track: 0,
    }));
  const chordTrack = melody.length ? 1 : 0;
  const voiced = chords.filter((c) => !c.isRest);
  const chordNotes: SongNote[] = voiced.flatMap((c) =>
    chordPitches(c, keyAt(c.beat)).map((key) => ({
      key,
      start: c.beat - 1,
      length: c.duration,
      velocity: CHORD_VELOCITY,
      track: chordTrack,
    })),
  );
  if (!melody.length && !chordNotes.length) throw new HooktheoryError("The pasted data has no notes or chords in it.");

  const tracks = [...(melody.length ? [{ name: "Melody" }] : []), ...(chordNotes.length ? [{ name: "Chords" }] : [])];
  const end = Math.max(0, ...[...melody, ...chordNotes].map((n) => n.start + n.length));
  const key = keyAt(1);
  return {
    song: {
      version: 2,
      title: title.trim() || "Untitled",
      artist: artist.trim(),
      tempos: [{ beat: 0, bpm }],
      time_signatures: [{ beat: 0, numerator: 4, denominator: 4 }],
      tracks,
      notes: [...melody, ...chordNotes].sort((a, b) => a.start - b.start || a.key - b.key),
    },
    melody: melody.length,
    chords: voiced.length,
    key: `${key.tonic} ${key.scale.replace(/([a-z])([A-Z])/g, "$1 $2").toLowerCase()}`,
    bars: Math.ceil(end / 4),
  };
}

/** The newer format counts in ticks from 0; turn it into the older 1-based beats. */
function fromTicks<T>(items: unknown): T[] {
  if (!Array.isArray(items)) return [];
  return (items as (Ticked & Record<string, unknown>)[]).map((i) => ({
    ...i,
    beat: i.startTick / TICKS_PER_BEAT + 1,
    duration: (i.durationTicks ?? 0) / TICKS_PER_BEAT,
  })) as T[];
}

function scaleOf(name: unknown): number[] | undefined {
  return typeof name === "string" ? SCALES[name.toLowerCase().replace(/[^a-z]/g, "")] : undefined;
}

function pitchClass(tonic: string): number {
  const m = /^([A-Ga-g])([#b♯♭]*)$/.exec(tonic.trim());
  if (!m) throw new HooktheoryError(`Unknown key "${tonic}".`);
  const shift = [...m[2]].reduce((s, c) => s + (c === "#" || c === "♯" ? 1 : -1), 0);
  return LETTERS[m[1].toUpperCase()] + shift;
}

/** Semitones above the scale's tonic for a 1-based degree, past the octave too (9 is 2 an octave up). */
function step(scale: number[], degree: number): number {
  const i = degree - 1;
  return scale[((i % 7) + 7) % 7] + 12 * Math.floor(i / 7);
}

/** The scale read from its `index`th note: the mode a chord on that degree is built from. */
function mode(scale: number[], index: number): number[] {
  return scale.map((_, i) => step(scale, index + i + 1) - scale[index]);
}

/** A melody note: degrees like "5", "b3" or "#4", with octave 0 starting at the first tonic from middle C. */
function degreePitch(sd: string, octave: number, key: HtKey): number {
  const m = /^([#b♯♭]*)(\d+)$/.exec(String(sd).trim());
  if (!m) throw new HooktheoryError(`Unknown scale degree "${sd}".`);
  const shift = [...m[1]].reduce((s, c) => s + (c === "#" || c === "♯" ? 1 : -1), 0);
  const scale = scaleOf(key.scale) ?? MAJOR;
  return 60 + pitchClass(key.tonic) + 12 * octave + step(scale, Number(m[2])) + shift;
}

/** Stacks thirds on the chord's root within its scale, then applies sus/add/omit/alterations and the inversion. */
function chordPitches(chord: HtChord, key: HtKey): number[] {
  const keyScale = scaleOf(key.scale) ?? MAJOR;
  const tonic = pitchClass(key.tonic);
  let root: number;
  let scale: number[];
  if (chord.applied) {
    // V/x is built on the major scale of x, vii°/x on its harmonic minor.
    const target = tonic + step(keyScale, chord.root);
    const parent = chord.applied === 7 ? HARMONIC_MINOR : MAJOR;
    root = target + parent[chord.applied - 1];
    scale = mode(parent, chord.applied - 1);
  } else {
    const parent = scaleOf(chord.borrowed) ?? keyScale;
    root = tonic + parent[chord.root - 1];
    scale = mode(parent, chord.root - 1);
  }

  const size = chord.type >= 13 ? 7 : chord.type >= 11 ? 6 : chord.type >= 9 ? 5 : chord.type >= 7 ? 4 : 3;
  const degrees = new Map<number, number>();
  for (let i = 0; i < size; i++) degrees.set(2 * i + 1, step(scale, 2 * i + 1));
  if (chord.suspensions?.length) {
    degrees.delete(3);
    for (const d of chord.suspensions) degrees.set(d, step(scale, d));
  }
  for (const d of chord.adds ?? []) degrees.set(d, step(scale, d));
  for (const d of chord.omits ?? []) degrees.delete(d);
  for (const a of chord.alterations ?? []) {
    const m = /^([#b])(\d+)$/.exec(a);
    if (m) degrees.set(Number(m[2]), step(scale, Number(m[2])) + (m[1] === "#" ? 1 : -1));
  }

  const bass = CHORD_FLOOR + ((((root - CHORD_FLOOR) % 12) + 12) % 12);
  const tones = [...degrees.values()].sort((a, b) => a - b).map((s) => bass + s);
  for (let i = 0; i < (chord.inversion ?? 0) && i < tones.length; i++) tones.push(tones.shift()! + 12);
  return tones.sort((a, b) => a - b);
}
