// Reviewing a pass: what you played (a recording) matched note by note against what the song asks for.
// Pure: no drawing, no storage.
import type { PlayedNote, PracticeNote, Recording } from "./practice";

/** A note the song asks for. Times are song seconds. */
export type Expected = { key: number; start: number; end: number; track: number };

/** The song's notes on the tracks a recording was played on. */
export function expectedFor(notes: PracticeNote[], recording: Recording): Expected[] {
  return notes
    .filter((n) => recording.tracks.includes(n.track))
    .map(({ key, start, end, track }) => ({ key, start, end, track }));
}
export type Timing = "good" | "early" | "late" | "missed";
export type Length = "ok" | "short" | "long";
export type NoteReview = Expected & {
  timing: Timing;
  /** Real seconds: how late (+) or early (-) you started it; in wait mode, how long the song waited. */
  offset: number;
  length: Length | null;
  /** Your press of this note. */
  played: PlayedNote | null;
};
export type Score = {
  /** 0–100, from the four parts below. */
  overall: number;
  notes: { hit: number; total: number };
  /** 0–100 each. */
  timing: number;
  length: number;
  /** Presses that weren't any note of the song. */
  extra: number;
  /** Keep tempo: average real seconds late (+) or early (-). Wait for me: average wait per chord. */
  averageOffset: number;
};
export type Review = { notes: NoteReview[]; extra: PlayedNote[]; score: Score };

/** One recorded pass and how it scored. `at` is Unix milliseconds. */
export type Take = { at: number; recording: Recording; score: Score };
/** Per song: the latest pass, the best one, and the ones you chose to keep (rec pad). */
export type Takes = { latest: Take | null; best: Take | null; kept: Take[] };
/** Kept takes beyond this many drop the oldest. */
const KEEP_LIMIT = 30;

export const noTakes = (): Takes => ({ latest: null, best: null, kept: [] });

/** A finished pass becomes the latest, and the best if it scores at least as well. True if it's a new best. */
export function addPass(takes: Takes, take: Take): boolean {
  takes.latest = take;
  if (takes.best && takes.best.score.overall > take.score.overall) return false;
  takes.best = take;
  return true;
}

/** Keeps a take (once). False if it was already kept. */
export function keepTake(takes: Takes, take: Take): boolean {
  if (takes.kept.some((t) => t.at === take.at)) return false;
  takes.kept.push(take);
  if (takes.kept.length > KEEP_LIMIT) takes.kept.shift();
  return true;
}

/** How far (real seconds) a press may be from a note and still count as playing it. */
const MATCH = 0.35;
/** Within this (real seconds) a note is on time; at FAR it scores nothing for timing. */
const ON_TIME = 0.07;
const FAR = 0.25;
/** Wait for me: a pause up to EASY_WAIT (real seconds) costs nothing; at LONG_WAIT it costs all. */
const EASY_WAIT = 0.3;
const LONG_WAIT = 2;
/** Real seconds of slack when judging how long a note was held. */
const LENGTH_SLACK = 0.1;
/** How much each part counts toward the overall score. */
const WEIGHTS = { notes: 0.4, timing: 0.3, length: 0.15, extra: 0.15 };

const clamp = (x: number) => Math.min(1, Math.max(0, x));

export function review(expected: Expected[], recording: Recording): Review {
  const { speed, mode } = recording;
  const window = MATCH * speed;

  // Pair presses with notes of the same key, closest first, one press per note.
  const pairs: { e: number; p: number; d: number }[] = [];
  expected.forEach((e, ei) =>
    recording.played.forEach((p, pi) => {
      const d = Math.abs(p.start - e.start);
      if (p.key === e.key && d <= window) pairs.push({ e: ei, p: pi, d });
    }),
  );
  pairs.sort((a, b) => a.d - b.d);
  const playedFor = new Map<number, number>();
  const used = new Set<number>();
  for (const { e, p } of pairs) {
    if (playedFor.has(e) || used.has(p)) continue;
    playedFor.set(e, p);
    used.add(p);
  }

  const waitAt = (start: number) => recording.waits.find((w) => Math.abs(w.start - start) < 0.001)?.seconds ?? 0;
  const notes: NoteReview[] = expected.map((e, i) => {
    const pi = playedFor.get(i);
    if (pi === undefined) return { ...e, timing: "missed", offset: 0, length: null, played: null };
    const p = recording.played[pi];
    const offset = mode === "wait" ? waitAt(e.start) : (p.start - e.start) / speed;
    const timing: Timing =
      mode === "wait"
        ? offset > EASY_WAIT * 2
          ? "late"
          : "good"
        : Math.abs(offset) <= ON_TIME
          ? "good"
          : offset < 0
            ? "early"
            : "late";
    return { ...e, timing, offset, length: judgeLength(e, p, speed), played: p };
  });

  const hit = notes.filter((n) => n.played);
  const extra = recording.played.filter((_, i) => !used.has(i));
  const mean = (xs: number[]) => (xs.length ? xs.reduce((a, b) => a + b, 0) / xs.length : 0);

  let timing: number;
  let averageOffset: number;
  if (mode === "wait") {
    // Per chord, not per note: one pause before a chord is one hesitation.
    const chordWaits = [...new Set(hit.map((n) => n.start))].map(waitAt);
    timing = mean(chordWaits.map((w) => clamp(1 - (w - EASY_WAIT) / (LONG_WAIT - EASY_WAIT))));
    averageOffset = mean(chordWaits);
  } else {
    timing = mean(hit.map((n) => clamp(1 - (Math.abs(n.offset) - ON_TIME) / (FAR - ON_TIME))));
    averageOffset = mean(hit.map((n) => n.offset));
  }
  const length = mean(hit.map((n) => lengthScore(n, speed)));
  const total = expected.length;
  const parts = {
    notes: total ? hit.length / total : 0,
    timing,
    length,
    extra: total ? clamp(1 - extra.length / total) : 0,
  };
  const overall =
    parts.notes * WEIGHTS.notes + parts.timing * WEIGHTS.timing + parts.length * WEIGHTS.length + parts.extra * WEIGHTS.extra;

  return {
    notes,
    extra,
    score: {
      overall: Math.round(overall * 100),
      notes: { hit: hit.length, total },
      timing: Math.round(timing * 100),
      length: Math.round(length * 100),
      extra: extra.length,
      averageOffset,
    },
  };
}

/** Held about as long as written (legato a bit longer is fine), too short, or far too long. */
function judgeLength(e: Expected, p: PlayedNote, speed: number): Length {
  const want = e.end - e.start;
  const held = p.end - p.start;
  const slack = LENGTH_SLACK * speed;
  if (held < want * 0.7 - slack) return "short";
  if (held > want * 1.5 + slack) return "long";
  return "ok";
}

function lengthScore(n: NoteReview, speed: number): number {
  if (!n.played) return 0;
  if (n.length === "ok") return 1;
  const ratio = (n.played.end - n.played.start) / Math.max(n.end - n.start, 0.01 * speed);
  return n.length === "short" ? clamp(ratio / 0.7) : clamp(1 - (ratio - 1.5) / 1.5);
}
