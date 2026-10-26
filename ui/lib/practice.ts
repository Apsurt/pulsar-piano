// Practice logic: where the song is, which notes are due, and whether they were played. No drawing here.
import { barTimes, secondsAt, type Hand, type Song } from "./songs";

export type Mode = "wait" | "tempo";
export type Hands = "both" | Hand;
export type NoteState = "pending" | "hit" | "missed";

export type PracticeNote = { key: number; start: number; end: number; hand: Hand | null; state: NoteState };
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

export class Practice {
  readonly notes: PracticeNote[];
  readonly chords: Chord[];
  readonly bars: number[];
  readonly duration: number;
  readonly hasHands: boolean;
  /** When the first note starts, in song seconds. */
  private readonly firstNote: number;

  /** Song position in seconds (at 100% speed). Negative during the lead-in. */
  time: number;
  playing = false;
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
  hands: Hands = "both";
  hits = 0;
  wrong = 0;
  /** performance.now() of the last wrong press per key, for a brief flash. */
  readonly wrongAt = new Float64Array(128);

  /** First chord not yet finished (wait mode) or not yet passed (tempo mode). */
  private cursor = 0;

  constructor(song: Song, leadIn = DEFAULT_LEAD_IN) {
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
    this.firstNote = this.chords[0]?.start ?? 0;
    this.leadIn = Math.max(0, leadIn);
    this.time = this.startTime;
  }

  /** Where playback begins: the lead-in before the first note, in song seconds at the current speed. */
  get startTime(): number {
    return this.firstNote - this.leadIn * this.speed;
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
    if (this.finished || !this.playing) return;
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
