// Plays a list of notes out loud on a song clock: hearing a take, or the song, in the review.
// No timers of its own: whoever owns it calls `update` every frame.

/** Times in song seconds. */
export type Playable = { key: number; velocity: number; start: number; end: number };

export class NotePlayer {
  /** Song seconds. */
  time = 0;
  playing = false;
  private notes: Playable[] = [];
  /** The first note not started yet, and the notes sounding now. */
  private cursor = 0;
  private readonly sounding = new Set<Playable>();

  /** `speed` turns real seconds into song seconds, so a take plays back at the pace it was played. */
  constructor(
    private readonly sound: (key: number, velocity: number) => void,
    public speed = 1,
  ) {}

  /** What to play; carries on from the same place. */
  setNotes(notes: Playable[]) {
    this.notes = [...notes].sort((a, b) => a.start - b.start || a.key - b.key);
    this.seek(this.time);
  }

  seek(time: number) {
    this.silence();
    this.time = time;
    this.cursor = this.notes.findIndex((n) => n.start >= time - 1e-6);
    if (this.cursor < 0) this.cursor = this.notes.length;
  }

  play() {
    this.playing = true;
  }

  pause() {
    this.playing = false;
    this.silence();
  }

  /** Advance by `dt` real seconds: ends first, so a repeated key sounds again. */
  update(dt: number) {
    if (!this.playing) return;
    const to = this.time + dt * this.speed;
    for (const n of this.sounding) {
      if (n.end > to) continue;
      this.sound(n.key, 0);
      this.sounding.delete(n);
    }
    while (this.cursor < this.notes.length && this.notes[this.cursor].start < to) {
      const n = this.notes[this.cursor++];
      // The same key starting again takes over from the note still sounding there.
      for (const other of this.sounding) if (other.key === n.key) this.sounding.delete(other);
      this.sound(n.key, n.velocity);
      this.sounding.add(n);
    }
    this.time = to;
  }

  silence() {
    for (const n of this.sounding) this.sound(n.key, 0);
    this.sounding.clear();
  }
}
