import { describe, expect, it } from "vitest";
import { Practice } from "./practice";
import type { Song } from "./songs";

/** At 60 bpm one beat is one second, so beats and seconds read the same. */
function song(notes: [key: number, start: number, length: number][]): Song {
  return {
    version: 1,
    title: "t",
    artist: "",
    tempos: [{ beat: 0, bpm: 60 }],
    time_signatures: [{ beat: 0, numerator: 4, denominator: 4 }],
    notes: notes.map(([key, start, length]) => ({ key, start, length, velocity: 90, hand: null })),
  };
}

/** Plays `seconds` of real time in small frames, like the render loop does. */
function run(p: Practice, seconds: number) {
  for (let t = 0; t < seconds; t += 0.01) p.update(0.01);
}

describe("lead-in", () => {
  it("starts that many real seconds before the first note", () => {
    const p = new Practice(song([[60, 1, 1]]), 3);
    expect(p.time).toBe(-2);
    p.setLeadIn(1);
    expect(p.time).toBe(0);
  });

  it("stays in real seconds at slower speeds", () => {
    const p = new Practice(song([[60, 4, 1]]), 2);
    p.setSpeed(0.5);
    // 2 real seconds at half speed is 1 second of song.
    expect(p.time).toBe(3);
  });

  it("doesn't move a song that has already started", () => {
    const p = new Practice(song([[60, 1, 1]]), 1);
    p.playing = true;
    run(p, 0.5);
    const time = p.time;
    p.setLeadIn(5);
    expect(p.time).toBe(time);
  });
});

describe("playing", () => {
  it("ignores keys while paused", () => {
    const p = new Practice(song([[60, 0, 1]]), 0);
    p.press(60);
    p.press(61);
    expect([p.hits, p.wrong, p.playing]).toEqual([0, 0, false]);
  });

  it("waits at a chord until it is played", () => {
    const p = new Practice(song([[60, 1, 1], [64, 1, 1]]), 1);
    p.playing = true;
    run(p, 3);
    expect(p.time).toBe(1);
    p.press(60);
    p.press(64);
    expect(p.hits).toBe(2);
    run(p, 0.5);
    expect(p.time).toBeGreaterThan(1);
  });
});

describe("loop", () => {
  it("starts over at the end and keeps the last score", () => {
    const p = new Practice(song([[60, 0, 1]]), 1);
    p.loop = true;
    p.playing = true;
    run(p, 1);
    p.press(60);
    run(p, 2);
    expect(p.passes).toBe(1);
    expect(p.lastScore).toBe(100);
    expect(p.playing).toBe(true);
    expect(p.finished).toBe(false);
    expect(p.hits).toBe(0);
  });

  it("stops at the end when off", () => {
    const p = new Practice(song([[60, 0, 1]]), 0);
    p.mode = "tempo";
    p.playing = true;
    run(p, 3);
    expect([p.finished, p.playing, p.passes, p.lastScore]).toEqual([true, false, 1, 0]);
  });
});
