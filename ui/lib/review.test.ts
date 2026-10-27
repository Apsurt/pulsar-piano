import { describe, expect, it } from "vitest";
import { Practice, type PlayedNote, type Recording } from "./practice";
import { addPass, expectedFor, keepTake, noTakes, review, type Expected, type Take } from "./review";
import type { Song } from "./songs";

const expect4: Expected[] = [
  { key: 60, start: 0, end: 1, track: 0 },
  { key: 62, start: 1, end: 2, track: 0 },
  { key: 64, start: 2, end: 3, track: 0 },
  { key: 65, start: 3, end: 4, track: 0 },
];

function take(played: [key: number, start: number, end: number][], extra: Partial<Recording> = {}): Recording {
  return {
    mode: "tempo",
    speed: 1,
    tracks: [0],
    played: played.map(([key, start, end]): PlayedNote => ({ key, velocity: 90, start, end })),
    waits: [],
    ...extra,
  };
}

describe("reviewing a pass", () => {
  it("scores a clean pass 100", () => {
    const r = review(expect4, take([[60, 0.02, 1], [62, 1, 1.95], [64, 1.99, 3], [65, 3.03, 4.1]]));
    expect(r.notes.map((n) => [n.timing, n.length])).toEqual(Array(4).fill(["good", "ok"]));
    expect(r.score).toMatchObject({ overall: 100, notes: { hit: 4, total: 4 }, timing: 100, length: 100, extra: 0 });
  });

  it("finds late, early, short, missed and extra notes", () => {
    const r = review(
      expect4,
      take([
        [60, 0.2, 1.2], // late
        [62, 0.88, 1.2], // early and short
        [61, 2, 2.5], // wrong key, so the E4 is missed
        [65, 3, 4],
      ]),
    );
    expect(r.notes.map((n) => n.timing)).toEqual(["late", "early", "missed", "good"]);
    expect(r.notes[1].length).toBe("short");
    expect(r.extra.map((n) => n.key)).toEqual([61]);
    expect(r.score.notes).toEqual({ hit: 3, total: 4 });
    expect(r.score.averageOffset).toBeCloseTo((0.2 - 0.12 + 0) / 3);
    expect(r.score.overall).toBeLessThan(80);
  });

  it("judges timing in real seconds when slowed down", () => {
    // At half speed, 0.1 song seconds late is 0.2 real seconds: late.
    const r = review(expect4.slice(0, 1), take([[60, 0.1, 1]], { speed: 0.5 }));
    expect(r.notes[0].timing).toBe("late");
    expect(r.notes[0].offset).toBeCloseTo(0.2);
  });

  it("counts hesitation per chord when the song waits for you", () => {
    const r = review(
      expect4.slice(0, 2),
      take([[60, 0, 1], [62, 1, 2]], { mode: "wait", waits: [{ start: 1, seconds: 1.5 }] }),
    );
    expect(r.notes.map((n) => n.timing)).toEqual(["good", "late"]);
    expect(r.score.averageOffset).toBeCloseTo(0.75);
    expect(r.score.timing).toBeLessThan(100);
  });
});

describe("recording a pass", () => {
  const song: Song = {
    version: 2,
    title: "t",
    artist: "",
    tempos: [{ beat: 0, bpm: 60 }],
    time_signatures: [{ beat: 0, numerator: 4, denominator: 4 }],
    tracks: [{ name: "Piano" }],
    notes: [
      { key: 60, start: 0, length: 1, velocity: 90, track: 0 },
      { key: 62, start: 1, length: 1, velocity: 90, track: 0 },
    ],
  };
  const run = (p: Practice, seconds: number) => {
    for (let t = 0; t < seconds; t += 0.01) p.update(0.01);
  };

  it("hands over what you played when a full pass ends", () => {
    const p = new Practice(song, 0);
    const passes: Recording[] = [];
    p.onPass = (r) => passes.push(r);
    p.setMode("tempo");
    p.playing = true;
    p.press(60, 80);
    run(p, 0.9);
    p.release(60);
    run(p, 0.15);
    p.press(62);
    run(p, 2);
    expect(passes).toHaveLength(1);
    const [played60, played62] = passes[0].played;
    expect(played60).toMatchObject({ key: 60, velocity: 80, start: 0 });
    expect(played60.end).toBeCloseTo(0.9, 1);
    // Still held when the pass ended: it ends there.
    expect(played62.end).toBeGreaterThan(2);
    expect(passes[0]).toMatchObject({ mode: "tempo", speed: 1, tracks: [0] });
  });

  it("records how long the song waited, and skips passes that jumped", () => {
    const p = new Practice(song, 0);
    const passes: Recording[] = [];
    p.onPass = (r) => passes.push(r);
    p.playing = true;
    run(p, 0.5);
    p.press(60);
    run(p, 1);
    p.press(62);
    run(p, 2);
    expect(passes[0].waits.map((w) => w.start)).toEqual([0]);
    expect(passes[0].waits[0].seconds).toBeCloseTo(0.5, 1);

    p.restart();
    p.playing = true;
    p.seek(1);
    run(p, 3);
    expect(passes).toHaveLength(1);
  });
});

describe("takes", () => {
  const scored = (at: number, overall: number): Take => ({
    at,
    recording: take([]),
    score: { overall, notes: { hit: 0, total: 0 }, timing: 0, length: 0, extra: 0, averageOffset: 0 },
  });

  it("keeps the latest pass, the best one, and the ones you keep", () => {
    const takes = noTakes();
    expect(addPass(takes, scored(1, 70))).toBe(true);
    expect(addPass(takes, scored(2, 60))).toBe(false);
    expect([takes.latest?.at, takes.best?.at]).toEqual([2, 1]);
    expect(addPass(takes, scored(3, 70))).toBe(true);
    expect(takes.best?.at).toBe(3);

    expect(keepTake(takes, takes.latest!)).toBe(true);
    expect(keepTake(takes, takes.latest!)).toBe(false);
    expect(takes.kept.map((t) => t.at)).toEqual([3]);
  });
});

describe("practicing a section", () => {
  const song: Song = {
    version: 2,
    title: "t",
    artist: "",
    tempos: [{ beat: 0, bpm: 60 }],
    time_signatures: [{ beat: 0, numerator: 4, denominator: 4 }],
    tracks: [{ name: "Piano" }],
    notes: [60, 62, 64, 65].map((key, i) => ({ key, start: i, length: 1, velocity: 90, track: 0 })),
  };
  const run = (p: Practice, seconds: number) => {
    for (let t = 0; t < seconds; t += 0.01) p.update(0.01);
  };

  it("plays only from the start point to the end point, with a lead-in", () => {
    const p = new Practice(song, 1);
    p.setSection(1, 3);
    expect(p.time).toBe(0);
    expect(p.total).toBe(2);
    expect(p.notes.map((n) => p.isActive(n))).toEqual([false, true, true, false]);

    const passes: Recording[] = [];
    p.onPass = (r) => passes.push(r);
    p.setMode("tempo");
    p.playing = true;
    run(p, 1);
    p.press(62);
    run(p, 1);
    p.press(64);
    run(p, 1.6);
    expect(p.finished).toBe(true);
    expect(passes[0].section).toEqual({ from: 1, to: 3 });

    // Scored against the section only, and never the best (that's for the whole song).
    const notes = expectedFor(p.notes, passes[0]);
    expect(notes.map((n) => n.key)).toEqual([62, 64]);
    const scored = review(notes, passes[0]).score;
    expect(scored.notes).toEqual({ hit: 2, total: 2 });
    const takes = noTakes();
    expect(addPass(takes, { at: 1, recording: passes[0], score: scored })).toBe(false);
    expect([takes.latest?.at, takes.best]).toEqual([1, null]);
  });

  it("waits only for chords inside the section", () => {
    const p = new Practice(song, 0);
    p.setSection(0, 1);
    p.playing = true;
    p.press(60);
    run(p, 2);
    expect(p.finished).toBe(true);
  });
});
