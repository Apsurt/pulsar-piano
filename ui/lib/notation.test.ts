import { describe, expect, it } from "vitest";
import { NotationError, parseNotation, parseNote } from "./notation";

describe("parseNote", () => {
  it("maps names and octaves to MIDI keys", () => {
    expect(parseNote("C4")).toBe(60);
    expect(parseNote("a0")).toBe(21);
    expect(parseNote("F#3")).toBe(54);
    expect(parseNote("Bb2")).toBe(46);
    expect(parseNote("C♯4")).toBe(61);
    expect(parseNote("C8")).toBe(108);
  });

  it("rejects things that aren't notes", () => {
    expect(() => parseNote("H4")).toThrow(NotationError);
    expect(() => parseNote("C")).toThrow(NotationError);
    expect(() => parseNote("C#")).toThrow(NotationError);
  });
});

describe("parseNotation", () => {
  it("lays notes out one after another, chords together, rests as gaps", () => {
    const { notes, beats } = parseNotation("C4 E4:2 | C4+G4:8. - D4", 1);
    expect(notes).toEqual([
      { key: 60, start: 0, length: 1 },
      { key: 64, start: 1, length: 2 },
      { key: 60, start: 3, length: 0.75 },
      { key: 67, start: 3, length: 0.75 },
      { key: 62, start: 4.75, length: 1 },
    ]);
    expect(beats).toBe(5.75);
  });

  it("uses the default length and accepts rests with a length", () => {
    expect(parseNotation("r:2 C4", 0.5)).toEqual({ notes: [{ key: 60, start: 2, length: 0.5 }], beats: 2.5 });
  });

  it("reports the offending token", () => {
    try {
      parseNotation("C4 E4:3", 1);
      expect.unreachable();
    } catch (e) {
      expect((e as NotationError).token).toBe("E4:3");
    }
  });

  it("treats empty input as nothing", () => {
    expect(parseNotation("  |  ", 1)).toEqual({ notes: [], beats: 0 });
  });
});
