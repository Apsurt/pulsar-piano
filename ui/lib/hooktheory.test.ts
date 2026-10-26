import { describe, expect, it } from "vitest";
import { fromHooktheory, HooktheoryError } from "./hooktheory";
import { noteName } from "./keyboard";

const chord = (beat: number, duration: number, root: number, type: number, extra = {}) => ({
  beat,
  duration,
  root,
  type,
  inversion: 0,
  applied: 0,
  adds: [],
  omits: [],
  alterations: [],
  suspensions: [],
  borrowed: "",
  isRest: false,
  ...extra,
});

function paste(notes: object[], chords: object[], tonic = "C", scale = "major") {
  return JSON.stringify({ notes, chords, keys: [{ beat: 1, scale, tonic }] });
}

const names = (keys: number[]) => keys.map(noteName).join(" ");

describe("Hooktheory import", () => {
  it("puts the melody and the chords on their own tracks, from beat 1", () => {
    const text = paste(
      [
        { beat: 1.5, duration: 0.5, sd: "5", octave: -1, isRest: false },
        { beat: 2, duration: 0.25, sd: "3", octave: -1, isRest: false },
        { beat: 3, duration: 1, sd: "1", octave: 0, isRest: true },
      ],
      [chord(1, 4, 4, 9), chord(5, 2, 5, 7), chord(7, 2, 1, 7, { isRest: true })],
      "A",
      "minor",
    );
    const { song, melody, chords, key, bars } = fromHooktheory(text, " Espresso ", "", 104);
    expect(song.tracks).toEqual([{ name: "Melody" }, { name: "Chords" }]);
    expect([melody, chords, key, bars]).toEqual([2, 2, "A minor", 2]);
    expect(song.title).toBe("Espresso");
    expect(song.tempos).toEqual([{ beat: 0, bpm: 104 }]);

    const tune = song.notes.filter((n) => n.track === 0);
    expect(tune.map((n) => [noteName(n.key), n.start, n.length])).toEqual([
      ["E4", 0.5, 0.5],
      ["C4", 1, 0.25],
    ]);
    const at = (beat: number) => song.notes.filter((n) => n.track === 1 && n.start === beat).map((n) => n.key);
    // iv9 and v7 in A minor: Dm9 and Em7.
    expect(names(at(0))).toBe("D3 F3 A3 C4 E4");
    expect(names(at(4))).toBe("E3 G3 B3 D4");
  });

  it("reads accidentals, inversions, suspensions, applied and borrowed chords", () => {
    const text = paste(
      [
        { beat: 1, duration: 1, sd: "b3", octave: 0 },
        { beat: 2, duration: 1, sd: "#4", octave: -1 },
      ],
      [
        chord(1, 1, 5, 7, { inversion: 1 }),
        chord(2, 1, 1, 5, { suspensions: [4] }),
        chord(3, 1, 2, 7, { applied: 5 }),
        chord(4, 1, 6, 5, { borrowed: "minor" }),
        chord(5, 1, 5, 7, { applied: 7 }),
      ],
    );
    const { song } = fromHooktheory(text, "t", "", 120);
    expect(names(song.notes.filter((n) => n.track === 0).map((n) => n.key))).toBe("D♯4 F♯3");
    const at = (beat: number) => names(song.notes.filter((n) => n.track === 1 && n.start === beat).map((n) => n.key));
    expect(at(0)).toBe("B2 D3 F3 G3"); // V7, first inversion
    expect(at(1)).toBe("C3 F3 G3"); // Csus4
    expect(at(2)).toBe("A2 C♯3 E3 G3"); // V7/ii
    expect(at(3)).toBe("G♯2 C3 D♯3"); // bVI from the minor
    expect(at(4)).toBe("F♯3 A3 C4 D♯4"); // vii°7/V
  });

  it("reads the tick-based format when the older lists are missing", () => {
    const text = JSON.stringify({
      modern: {
        payload: {
          notes: [{ sd: "1", octave: 0, startTick: 960, durationTicks: 240 }],
          chords: [],
          keys: [{ startTick: 0, scale: "major", tonic: "G" }],
        },
      },
    });
    const { song } = fromHooktheory(text, "t", "", 90);
    expect(song.tracks).toEqual([{ name: "Melody" }]);
    expect(song.notes.map((n) => [noteName(n.key), n.start, n.length])).toEqual([["G4", 2, 0.5]]);
  });

  it("explains what's wrong with other text", () => {
    expect(() => fromHooktheory("C4 E4 G4", "t", "", 90)).toThrow(HooktheoryError);
    expect(() => fromHooktheory(paste([], []), "t", "", 90)).toThrow(/no notes/);
  });
});
