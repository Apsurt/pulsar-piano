import { describe, expect, it } from "vitest";
import { octaveLabel, planOctaves } from "./octave";

/** One note per second, so note i starts at second i; bars every 4 seconds. */
const notes = (keys: number[]) => keys.map((key, i) => ({ key, start: i }));
const bars = Array.from({ length: 40 }, (_, i) => i * 4);

describe("octave plan", () => {
  it("keeps the normal range when the song fits it", () => {
    expect(planOctaves(notes([48, 60, 72, 84]), bars)).toEqual({ shift: 0, changes: [], outside: 0 });
  });

  it("shifts the whole song when it sits higher or lower", () => {
    expect(planOctaves(notes([72, 84, 91, 96]), bars).shift).toBe(1);
    expect(planOctaves(notes([36, 40, 43, 55]), bars).shift).toBe(-1);
  });

  it("changes octave mid-song only when enough notes need it", () => {
    // Eight notes low, then eight notes two octaves up: one change is worth it.
    const plan = planOctaves(notes([...Array(8).fill(50), ...Array(8).fill(95)]), bars);
    expect(plan.shift).toBe(0);
    expect(plan.changes).toEqual([{ at: 8, bar: 3, shift: 1 }]);
    expect(plan.outside).toBe(0);

    // A single high note isn't worth two presses: it just falls off the keys.
    const once = planOctaves(notes([50, 52, 53, 96, 52, 50]), bars);
    expect(once).toEqual({ shift: 0, changes: [], outside: 1 });
  });

  it("names the setting the way the buttons count", () => {
    expect([octaveLabel(1), octaveLabel(0), octaveLabel(-2)]).toEqual(["Oct +1", "Oct 0", "Oct −2"]);
  });
});
