// Where to set a small keyboard's octave buttons for a song. A 37-key keyboard covers C3–C6 at Oct 0 and
// shifts by whole octaves; the plan keeps the notes you play on your keys with as few presses as it can.

/** The MiniLab 37 at Oct 0: C3 to C6. */
export const KEYBOARD = { low: 48, high: 84 };
/** Octave buttons go this far either way. */
const MAX_SHIFT = 3;
/** One octave change is worth it only if it brings more than this many notes back onto your keys. */
const CHANGE_COST = 4;

export type OctaveChange = { at: number; bar: number; shift: number };
export type OctavePlan = {
  /** Where to set it before you start: 0 is the keyboard's normal range, +1 an octave up. */
  shift: number;
  /** Octave changes during the song, if any are worth it. */
  changes: OctaveChange[];
  /** Notes that still fall off your keys. */
  outside: number;
};

/** `notes` in song seconds, sorted by start; `bars` are bar start times (for saying where changes go). */
export function planOctaves(notes: { key: number; start: number }[], bars: number[], range = KEYBOARD): OctavePlan {
  const shifts = Array.from({ length: MAX_SHIFT * 2 + 1 }, (_, i) => i - MAX_SHIFT);
  // Notes that start together are played together, so the octave can only change between them.
  const groups: { start: number; keys: number[] }[] = [];
  for (const n of notes) {
    const last = groups.at(-1);
    if (last && n.start - last.start < 0.03) last.keys.push(n.key);
    else groups.push({ start: n.start, keys: [n.key] });
  }
  if (!groups.length) return { shift: 0, changes: [], outside: 0 };
  const off = (keys: number[], s: number) =>
    keys.filter((k) => k < range.low + 12 * s || k > range.high + 12 * s).length;

  // Cheapest way to reach each group at each shift: notes off the keys, plus a cost per change. Starting
  // nearer Oct 0 breaks ties, so the normal range wins whenever it does just as well.
  let cost = shifts.map((s) => off(groups[0].keys, s) + Math.abs(s) * 0.01);
  const from: number[][] = [];
  for (let g = 1; g < groups.length; g++) {
    const prev = cost;
    const step: number[] = [];
    cost = shifts.map((s, i) => {
      // Stay at this shift, or change to it from another one if that's cheaper.
      let best = i;
      let bestCost = prev[i];
      for (let j = 0; j < shifts.length; j++) {
        if (j !== i && prev[j] + CHANGE_COST < bestCost) {
          best = j;
          bestCost = prev[j] + CHANGE_COST;
        }
      }
      step.push(best);
      return bestCost + off(groups[g].keys, s);
    });
    from.push(step);
  }

  // Walk back from the cheapest end.
  let i = cost.indexOf(Math.min(...cost));
  const path = [i];
  for (let g = from.length - 1; g >= 0; g--) {
    i = from[g][i];
    path.unshift(i);
  }
  const barAt = (t: number) => Math.max(1, bars.filter((b) => b <= t + 1e-6).length);
  const changes: OctaveChange[] = [];
  for (let g = 1; g < groups.length; g++)
    if (path[g] !== path[g - 1]) changes.push({ at: groups[g].start, bar: barAt(groups[g].start), shift: shifts[path[g]] });
  const outside = groups.reduce((sum, grp, g) => sum + off(grp.keys, shifts[path[g]]), 0);
  return { shift: shifts[path[0]], changes, outside };
}

/** "Oct +1", "Oct 0", "Oct −2". */
export const octaveLabel = (shift: number) => `Oct ${shift > 0 ? "+" : shift < 0 ? "−" : ""}${Math.abs(shift)}`;
