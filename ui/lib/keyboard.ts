// 88-key geometry, note names and the computer-keyboard map.

export const LOWEST = 21; // A0
export const HIGHEST = 108; // C8

export type KeyRect = { key: number; x: number; w: number; h: number; black: boolean };

export const isBlack = (key: number) => [1, 3, 6, 8, 10].includes(key % 12);

/** Rects indexed by MIDI key; white keys share the width evenly, black keys sit on the seams. */
export function layout(width: number, height: number): KeyRect[] {
  let whites = 0;
  for (let k = LOWEST; k <= HIGHEST; k++) if (!isBlack(k)) whites++;
  const ww = width / whites;
  const bw = ww * 0.58;
  const rects: KeyRect[] = [];
  let i = 0;
  for (let k = LOWEST; k <= HIGHEST; k++) {
    if (isBlack(k)) rects[k] = { key: k, x: i * ww - bw / 2, w: bw, h: height * 0.62, black: true };
    else rects[k] = { key: k, x: i++ * ww, w: ww, h: height, black: false };
  }
  return rects;
}

/** Black keys sit on top, so they win. `y` is relative to the top of the keyboard. */
export function hitTest(rects: KeyRect[], x: number, y: number): KeyRect | null {
  let white: KeyRect | null = null;
  for (let k = LOWEST; k <= HIGHEST; k++) {
    const r = rects[k];
    if (!r || x < r.x || x >= r.x + r.w || y < 0 || y > r.h) continue;
    if (r.black) return r;
    white = r;
  }
  return white;
}

const NAMES = ["C", "C♯", "D", "D♯", "E", "F", "F♯", "G", "G♯", "A", "A♯", "B"];
export const noteName = (key: number) => NAMES[key % 12] + (Math.floor(key / 12) - 1);

/** Semitone offsets from the current octave's C, by physical key (layout independent). */
export const COMPUTER_KEYS: Record<string, number> = {
  KeyA: 0, KeyW: 1, KeyS: 2, KeyE: 3, KeyD: 4, KeyF: 5, KeyT: 6, KeyG: 7, KeyY: 8, KeyH: 9,
  KeyU: 10, KeyJ: 11, KeyK: 12, KeyO: 13, KeyL: 14, KeyP: 15, Semicolon: 16, Quote: 17,
};
