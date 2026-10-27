import { describe, expect, it } from "vitest";
import { NotePlayer } from "./player";

function player(speed = 1) {
  const heard: string[] = [];
  const p = new NotePlayer((key, velocity) => heard.push(`${key}:${velocity}`), speed);
  p.setNotes([
    { key: 64, velocity: 70, start: 1, end: 1.5 },
    { key: 60, velocity: 80, start: 0, end: 1 },
    { key: 60, velocity: 90, start: 1, end: 2 },
  ]);
  const run = (seconds: number) => {
    for (let t = 0; t < seconds; t += 0.01) p.update(0.01);
  };
  return { p, heard, run };
}

describe("note player", () => {
  it("plays notes in order, and a repeated key again", () => {
    const { p, heard, run } = player();
    p.play();
    run(1.2);
    expect(heard).toEqual(["60:80", "60:0", "60:90", "64:70"]);
    run(1);
    expect(heard.slice(4)).toEqual(["64:0", "60:0"]);
  });

  it("is silent while paused, and lets go when paused or moved", () => {
    const { p, heard, run } = player();
    run(1);
    expect(heard).toEqual([]);
    p.play();
    run(0.5);
    p.pause();
    expect(heard).toEqual(["60:80", "60:0"]);
    p.seek(1);
    p.play();
    run(0.05);
    expect(heard.slice(2)).toEqual(["60:90", "64:70"]);
  });

  it("runs at the pace a take was played", () => {
    const { p, heard, run } = player(0.5);
    p.play();
    run(1.5);
    // 1.5 real seconds at half speed is 0.75 song seconds: still on the first note.
    expect(heard).toEqual(["60:80"]);
  });
});
