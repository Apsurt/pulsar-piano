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
