// Practice logic: where the song is, which notes are due, whether they were played, and which notes
// Pulsar plays itself. No drawing or audio here: sound goes out through `sound`.
import { barTimes, secondsAt, type Song } from "./songs";

/** wait: the song waits for you. tempo: it keeps going. listen: Pulsar plays it all for you. */
export type Mode = "wait" | "tempo" | "listen";
export type NoteState = "pending" | "hit" | "missed";

/** `track` indexes Song.tracks. */
export type PracticeNote = {
  key: number;
  start: number;
  end: number;
  velocity: number;
  track: number;
  state: NoteState;
};
/** Notes that start together: a chord, or a single note. */
export type Chord = { start: number; notes: PracticeNote[] };

/** Default real seconds from pressing play to the first note. */
export const DEFAULT_LEAD_IN = 2;
/** Notes starting within this many seconds of each other form one chord. */
const CHORD_SPREAD = 0.03;
/** Tempo mode: how far (real seconds) from a note's start a press still counts. */
const HIT_WINDOW = 0.18;
/** Wait mode: how early (real seconds) a press counts for the coming chord. */
const EARLY = 0.25;
/** With wrong keys silent, wait mode lets this many coming chords sound, so a quick next note isn't cut off. */
const SOUND_CHORDS = 2;
/** With wrong keys silent, tempo mode lets notes this far ahead (real seconds) sound already. */
const SOUND_AHEAD = 0.35;

export class Practice {
  readonly notes: PracticeNote[];
  readonly chords: Chord[];
  readonly bars: number[];
  readonly duration: number;
  /** Track names, and how many notes each has. */
  readonly tracks: { name: string; notes: number }[];
  /** When the first note starts, in song seconds. */
  private readonly firstNote: number;

  /** Song position in seconds (at 100% speed). Negative during the lead-in. */
  time: number;
  finished = false;
  speed = 1;
  /** Real seconds between pressing play and the first note. */
  leadIn: number;
  /** Start over by itself at the end. */
  loop = false;
  /** Times the song has been played through, and the score of the last full pass. */
  passes = 0;
  lastScore: number | null = null;
  mode: Mode = "wait";
  /** The tracks you play; the rest aren't drawn and are left out of the score. */
  played: Set<number>;
  /** Listen mode: the tracks Pulsar plays (and draws). */
  heard: Set<number>;
  /** Pulsar plays the tracks you don't. */
  accompany = false;
  hits = 0;
  wrong = 0;
  /** performance.now() of the last wrong press per key, for a brief flash. */
  readonly wrongAt = new Float64Array(128);
  /** Plays a note out loud (velocity 0 releases it): the notes Pulsar plays go out here. */
  sound: ((key: number, velocity: number) => void) | null = null;

  /** First chord not yet finished (wait mode) or not yet passed (tempo mode). */
  private cursor = 0;
  private _playing = false;
  /** Notes Pulsar is sounding now, and the first note it hasn't reached yet. */
  private readonly sounding = new Set<PracticeNote>();
  private autoCursor = 0;

  constructor(song: Song, leadIn = DEFAULT_LEAD_IN) {
    this.notes = song.notes.map((n) => ({
      key: n.key,
      start: secondsAt(song, n.start),
      end: secondsAt(song, n.start + n.length),
      velocity: n.velocity,
      track: n.track,
      state: "pending" as NoteState,
    }));
    this.chords = [];
    for (const note of this.notes) {
      const last = this.chords.at(-1);
      if (last && note.start - last.start <= CHORD_SPREAD) last.notes.push(note);
      else this.chords.push({ start: note.start, notes: [note] });
    }
    const endBeat = Math.max(0, ...song.notes.map((n) => n.start + n.length));
    this.bars = barTimes(song, endBeat);
    this.duration = Math.max(0, ...this.notes.map((n) => n.end));
    this.tracks = song.tracks.map((t, i) => ({ name: t.name, notes: song.notes.filter((n) => n.track === i).length }));
    this.played = new Set(this.tracks.keys());
    this.heard = new Set(this.tracks.keys());
    this.firstNote = this.chords[0]?.start ?? 0;
    this.leadIn = Math.max(0, leadIn);
    this.time = this.startTime;
  }

  /** Where playback begins: the lead-in before the first note, in song seconds at the current speed. */
  get startTime(): number {
    return this.firstNote - this.leadIn * this.speed;
  }

  get playing(): boolean {
    return this._playing;
  }

  /** Pausing also lets go of anything Pulsar is playing. */
  set playing(on: boolean) {
    this._playing = on;
    if (!on) this.silence();
  }

  private get atStart(): boolean {
    return !this.playing && this.time <= this.startTime + 1e-6;
  }

  setSpeed(speed: number) {
    const atStart = this.atStart;
    this.speed = speed;
    if (atStart) this.time = this.startTime;
  }

  setLeadIn(seconds: number) {
    const atStart = this.atStart;
    this.leadIn = Math.max(0, seconds);
    if (atStart) this.time = this.startTime;
  }

  /** The tracks on screen: the ones you play, or when listening, the ones you hear. */
  get shown(): Set<number> {
    return this.mode === "listen" ? this.heard : this.played;
  }

  /** On screen, and yours to play (or, when listening, to hear). */
  isActive(note: PracticeNote): boolean {
    return this.shown.has(note.track);
  }

  /** Pulsar plays this note: the heard tracks when listening, the other tracks when accompanying. */
  isAuto(note: PracticeNote): boolean {
    return this.mode === "listen" ? this.heard.has(note.track) : this.accompany && !this.played.has(note.track);
  }

  /** Whether Pulsar is sounding `key` right now (so its echo isn't taken for your press). */
  isAutoKey(key: number): boolean {
    for (const n of this.sounding) if (n.key === key) return true;
    return false;
  }

  /** Lets go of every note Pulsar is playing. */
  silence() {
    for (const n of this.sounding) this.sound?.(n.key, 0);
    this.sounding.clear();
  }

  get total(): number {
    return this.notes.filter((n) => this.isActive(n)).length;
  }

  /** For "wrong keys silent": the keys that may sound now (what's due, with some slack), or null for any key. */
  soundKeys(): number[] | null {
    if (!this.playing || this.finished || this.mode === "listen") return null;
    const keys = new Set<number>();
    const due = (n: PracticeNote) => this.isActive(n) && n.state === "pending";
    if (this.mode === "wait") {
      this.nextChord();
      for (const chord of this.chords.slice(this.cursor, this.cursor + SOUND_CHORDS))
        for (const n of chord.notes) if (due(n)) keys.add(n.key);
    } else {
      const from = this.time - HIT_WINDOW * this.speed;
      const to = this.time + (HIT_WINDOW + SOUND_AHEAD) * this.speed;
      for (const n of this.notes) if (n.start >= from && n.start <= to && due(n)) keys.add(n.key);
    }
    return [...keys].sort((a, b) => a - b);
  }

  /** Wait mode: the chord being waited for, once the song has reached it. */
  get waiting(): Chord | null {
    const chord = this.mode === "wait" ? this.nextChord() : undefined;
    return chord && this.time >= chord.start ? chord : null;
  }

  /** Advance by `dt` real seconds. Called once per frame. */
  update(dt: number) {
    if (!this.playing) return;
    let next = this.time + dt * this.speed;
    if (this.mode === "wait") {
      const chord = this.nextChord();
      if (chord && next >= chord.start) next = chord.start;
    } else if (this.mode === "tempo") {
      const late = next - HIT_WINDOW * this.speed;
      while (this.cursor < this.chords.length && this.chords[this.cursor].start < late) {
        for (const n of this.chords[this.cursor].notes) if (this.isActive(n) && n.state === "pending") n.state = "missed";
        this.cursor++;
      }
    }
    this.autoplay(this.time, next);
    this.time = next;
    if (this.time >= this.duration + 0.5) {
      this.passes++;
      this.lastScore = this.total ? Math.round((this.hits / this.total) * 100) : 0;
      if (this.loop) {
        this.restart();
        this.playing = true;
      } else {
        this.playing = false;
        this.finished = true;
      }
    }
  }

  /** A key went down (from any source). Ignored while paused: only play/pause starts the song. */
  press(key: number) {
    if (this.finished || !this.playing || this.mode === "listen") return;
    if (this.mode === "wait") {
      const chord = this.nextChord();
      if (!chord) return;
      const soon = (chord.start - this.time) / this.speed <= EARLY;
      const note = soon && chord.notes.find((n) => n.key === key && this.isActive(n) && n.state === "pending");
      if (note) {
        note.state = "hit";
        this.hits++;
      } else {
        this.miss(key);
      }
      return;
    }
    let best: PracticeNote | null = null;
    let distance = HIT_WINDOW * this.speed;
    for (const n of this.notes) {
      const d = Math.abs(n.start - this.time);
      if (n.key === key && n.state === "pending" && this.isActive(n) && d <= distance) {
        best = n;
        distance = d;
      }
    }
    if (best) {
      best.state = "hit";
      this.hits++;
    } else {
      this.miss(key);
    }
  }

  /** Jump to a song position; everything from there on is to be played again. */
  seek(time: number) {
    this.time = Math.min(Math.max(time, this.startTime), this.duration);
    for (const n of this.notes) if (n.start >= this.time - 0.001) n.state = "pending";
    this.recount();
    this.cursor = this.chords.findIndex((c) => c.start >= this.time - 0.001);
    if (this.cursor < 0) this.cursor = this.chords.length;
    this.silence();
    this.autoCursor = this.notes.findIndex((n) => n.start >= this.time - 0.001);
    if (this.autoCursor < 0) this.autoCursor = this.notes.length;
    this.finished = false;
  }

  restart() {
    this.wrong = 0;
    this.playing = false;
    this.seek(this.startTime);
  }

  setMode(mode: Mode) {
    this.mode = mode;
    this.seek(this.time);
  }

  /** Which tracks to play; unknown ones are dropped, and nothing selected means all of them. */
  setPlayed(tracks: Iterable<number>) {
    this.played = this.validTracks(tracks);
    this.recount();
  }

  /** Which tracks to hear in listen mode, the same way. */
  setHeard(tracks: Iterable<number>) {
    this.heard = this.validTracks(tracks);
  }

  private validTracks(tracks: Iterable<number>): Set<number> {
    const valid = [...tracks].filter((t) => t >= 0 && t < this.tracks.length);
    return new Set(valid.length ? valid : this.tracks.keys());
  }

  /** Pulsar's notes between two song times: ends first, so a repeated key sounds again. */
  private autoplay(from: number, to: number) {
    for (const n of this.sounding) {
      if (n.end > to && this.isAuto(n)) continue;
      this.sound?.(n.key, 0);
      this.sounding.delete(n);
    }
    while (this.autoCursor < this.notes.length && this.notes[this.autoCursor].start < to) {
      const n = this.notes[this.autoCursor++];
      if (n.start < from - 0.001 || !this.isAuto(n)) continue;
      // The same key starting again takes over from the note still sounding there.
      for (const other of this.sounding) if (other.key === n.key) this.sounding.delete(other);
      this.sound?.(n.key, n.velocity);
      this.sounding.add(n);
    }
  }

  private nextChord(): Chord | undefined {
    const done = (c: Chord) => c.notes.every((n) => !this.isActive(n) || n.state !== "pending");
    while (this.cursor < this.chords.length && done(this.chords[this.cursor])) this.cursor++;
    return this.chords[this.cursor];
  }

  private miss(key: number) {
    this.wrong++;
    this.wrongAt[key] = performance.now();
  }

  private recount() {
    this.hits = this.notes.filter((n) => n.state === "hit" && this.isActive(n)).length;
  }
}
