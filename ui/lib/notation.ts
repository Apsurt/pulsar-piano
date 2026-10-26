// Plain-text notes, for typing in songs found online: "C4 E4 G4:2 | C4+E4+G4:1 - F#3:8."
//
//   note     name + octave: C4, F#3, Bb2 (also ♯ / ♭); MIDI 60 is C4
//   chord    notes joined with +: C4+E4+G4
//   length   optional :1 whole, :2 half, :4 quarter, :8 eighth, :16, :32; a trailing . makes it dotted
//   rest     - (or r), optionally with a length: -:2
//   bar      | is allowed anywhere and ignored (it's only there to help you read)

export type TypedNote = { key: number; start: number; length: number };
export type Parsed = { notes: TypedNote[]; beats: number };

export class NotationError extends Error {
  constructor(
    message: string,
    readonly token: string,
  ) {
    super(message);
  }
}

const PITCH: Record<string, number> = { c: 0, d: 2, e: 4, f: 5, g: 7, a: 9, b: 11 };
const LENGTHS: Record<string, number> = { "1": 4, "2": 2, "4": 1, "8": 0.5, "16": 0.25, "32": 0.125 };

/** Parses `text` into notes starting at beat 0; tokens without a length use `defaultLength` (beats). */
export function parseNotation(text: string, defaultLength: number): Parsed {
  const notes: TypedNote[] = [];
  let beat = 0;
  for (const token of text.split(/\s+/).filter((t) => t && t !== "|")) {
    const [body, lengthPart] = token.split(":");
    const length = lengthPart === undefined ? defaultLength : parseLength(lengthPart, token);
    if (body === "-" || body.toLowerCase() === "r") {
      beat += length;
      continue;
    }
    for (const name of body.split("+")) notes.push({ key: parseNote(name, token), start: beat, length });
    beat += length;
  }
  return { notes, beats: beat };
}

/** MIDI key for a note name like "F#3"; throws NotationError. */
export function parseNote(name: string, token = name): number {
  const m = /^([a-gA-G])([#♯b♭]?)(-?\d)$/.exec(name);
  if (!m) throw new NotationError(`"${name}" isn't a note — write it like C4, F#3 or Bb2`, token);
  const accidental = m[2] === "#" || m[2] === "♯" ? 1 : m[2] ? -1 : 0;
  const key = (Number(m[3]) + 1) * 12 + PITCH[m[1].toLowerCase()] + accidental;
  if (key < 0 || key > 127) throw new NotationError(`"${name}" is outside the MIDI range`, token);
  return key;
}

function parseLength(part: string, token: string): number {
  const dotted = part.endsWith(".");
  const base = LENGTHS[dotted ? part.slice(0, -1) : part];
  if (base === undefined) throw new NotationError(`"${token}": length must be :1, :2, :4, :8, :16 or :32`, token);
  return dotted ? base * 1.5 : base;
}
