// Practice logic: where the song is, which notes are due, and whether they were played. No drawing here.
import { barTimes, secondsAt, type Hand, type Song } from "./songs";

export type Mode = "wait" | "tempo";
export type Hands = "both" | Hand;
export type NoteState = "pending" | "hit" | "missed";

export type PracticeNote = { key: number; start: number; end: number; hand: Hand | null; state: NoteState };
/** Notes that start together: a chord, or a single note. */
export type Chord = { start: number; notes: PracticeNote[] };

/** Seconds shown before the first note, so it can fall into view. */
const LEAD_IN = 2;
/** Notes starting within this many seconds of each other form one chord. */
const CHORD_SPREAD = 0.03;
/** Tempo mode: how far (real seconds) from a note's start a press still counts. */
const HIT_WINDOW = 0.18;
/** Wait mode: how early (real seconds) a press counts for the coming chord. */
const EARLY = 0.25;

export class Practice {
  readonly notes: PracticeNote[];
  readonly chords: Chord[];
  readonly bars: number[];
  readonly duration: number;
  readonly hasHands: boolean;
  readonly startTime: number;

  /** Song position in seconds (at 100% speed). Negative during the lead-in. */
  time: number;
  playing = false;
  finished = false;
  speed = 1;
  mode: Mode = "wait";
  hands: Hands = "both";
  hits = 0;
  wrong = 0;
  /** performance.now() of the last wrong press per key, for a brief flash. */
  readonly wrongAt = new Float64Array(128);

  /** First chord not yet finished (wait mode) or not yet passed (tempo mode). */
  private cursor = 0;

  constructor(song: Song) {
    this.notes = song.notes.map((n) => ({
      key: n.key,
      start: secondsAt(song, n.start),
      end: secondsAt(song, n.start + n.length),
      hand: n.hand,
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
    this.hasHands = this.notes.some((n) => n.hand !== null);
    this.startTime = (this.chords[0]?.start ?? 0) - LEAD_IN;
    this.time = this.startTime;
  }

  isActive(note: PracticeNote): boolean {
    return this.hands === "both" || note.hand === null || note.hand === this.hands;
  }

  get total(): number {
    return this.notes.filter((n) => this.isActive(n)).length;
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
    } else {
      const late = next - HIT_WINDOW * this.speed;
      while (this.cursor < this.chords.length && this.chords[this.cursor].start < late) {
        for (const n of this.chords[this.cursor].notes) if (this.isActive(n) && n.state === "pending") n.state = "missed";
        this.cursor++;
      }
    }
    this.time = next;
    if (this.time >= this.duration + 0.5) {
      this.playing = false;
      this.finished = true;
    }
  }

  /** A key went down (from any source). */
  press(key: number) {
    if (this.finished) return;
    if (this.mode === "wait") {
      const chord = this.nextChord();
      if (!chord) return;
      // While paused any distance counts, so playing the first chord's notes starts the song.
      const soon = !this.playing || (chord.start - this.time) / this.speed <= EARLY;
      const note = soon && chord.notes.find((n) => n.key === key && this.isActive(n) && n.state === "pending");
      if (note) {
        note.state = "hit";
        this.hits++;
        this.playing = true;
      } else if (this.playing) {
        this.miss(key);
      }
      return;
    }
    if (!this.playing) return;
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

  setHands(hands: Hands) {
    this.hands = hands;
    this.recount();
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
