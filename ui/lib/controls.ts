// Hardware controls (transport pads) as app actions. Whichever view is open handles them.

export type Action = "loop" | "stop" | "play" | "record" | "tap";

/** Arturia MiniLab 3 / MiniLab 37 transport pads, sent as CCs on the "DAW" port (127 down, 0 up). */
const MINILAB: Record<number, Action> = { 105: "loop", 106: "stop", 107: "play", 108: "record", 109: "tap" };

/** The action for a control-port CC, on press only. */
export function actionFor(controller: number, value: number): Action | null {
  return value > 0 ? (MINILAB[controller] ?? null) : null;
}

let handler: ((action: Action) => void) | null = null;

/** Makes `fn` the current handler (one view at a time); returns an unsubscribe for onMount. */
export function onAction(fn: (action: Action) => void): () => void {
  handler = fn;
  return () => {
    if (handler === fn) handler = null;
  };
}

export function fire(action: Action) {
  handler?.(action);
}

// ---- Knobs and faders: continuous controls on the keyboard port ----------------------------
//
// MiniLab 37, Arturia preset (measured): everything below is on the "MIDI" port, absolute 0–127.
//   knobs 1–8   CC 74, 71, 76, 77, 93, 18, 19, 16   (93 is also "chorus send" to SoundFonts)
//   faders 1–4  CC 82, 83, 85, 17
//   left strip  pitch bend (14-bit, springs back to centre), handled by the instrument
//   right strip CC 1 (mod wheel), handled by the instrument

const MINILAB_KNOBS: Record<number, number> = { 74: 1, 71: 2, 76: 3, 77: 4, 93: 5, 18: 6, 19: 7, 16: 8 };

/** The knob number (1–8) for a keyboard-port CC, if it is one. Not assigned to anything yet. */
export function knobFor(controller: number): number | null {
  return MINILAB_KNOBS[controller] ?? null;
}

/** MiniLab 37 faders 1–4 in its Arturia preset (CC on the "MIDI" port, 0 bottom to 127 top). */
const MINILAB_FADERS: Record<number, number> = { 82: 1, 83: 2, 85: 3, 17: 4 };

/** The fader number (1–4) for a keyboard-port CC, if it is one. */
export function faderFor(controller: number): number | null {
  return MINILAB_FADERS[controller] ?? null;
}

let faderHandler: ((fader: number, value: number) => void) | null = null;

/** `value` is 0 (bottom) to 1 (top). One view at a time; returns an unsubscribe for onMount. */
export function onFader(fn: (fader: number, value: number) => void): () => void {
  faderHandler = fn;
  return () => {
    if (faderHandler === fn) faderHandler = null;
  };
}

export function moveFader(fader: number, value: number) {
  faderHandler?.(fader, value);
}

// ---- Touch strips: the left one is pitch bend (springs back to the middle), the right one CC 1 ----------

export type Strip = "left" | "right";
/** The right strip (mod wheel) in the MiniLab's Arturia preset. */
export const MOD_WHEEL = 1;

let stripHandler: ((strip: Strip, value: number) => void) | null = null;

/** `value`: left strip -1 (down) to 1 (up), 0 at rest; right strip 0 (bottom) to 1 (top).
 *  One view at a time; returns an unsubscribe for onMount. */
export function onStrip(fn: (strip: Strip, value: number) => void): () => void {
  stripHandler = fn;
  return () => {
    if (stripHandler === fn) stripHandler = null;
  };
}

export function moveStrip(strip: Strip, value: number) {
  stripHandler?.(strip, value);
}
