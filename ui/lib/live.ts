// Per-frame state the canvas reads. Deliberately not reactive: it changes on every note.

export type Beam = { key: number; velocity: number; born: number; released: number | null };
export type Flare = { key: number; velocity: number; born: number };

export const live = {
  /** 0 = key up */
  velocity: new Uint8Array(128),
  /** 1 on press, decays after release */
  glow: new Float32Array(128),
  sustain: false,
  beams: [] as Beam[],
  flares: [] as Flare[],
};

export function press(key: number, velocity: number) {
  const now = performance.now();
  release(key, now);
  live.velocity[key] = velocity;
  live.glow[key] = 1;
  live.beams.push({ key, velocity, born: now, released: null });
  live.flares.push({ key, velocity, born: now });
  if (live.beams.length > 600) live.beams.shift();
}

export function release(key: number, now = performance.now()) {
  live.velocity[key] = 0;
  for (const b of live.beams) if (b.key === key && b.released === null) b.released = now;
}

export const heldCount = () => live.velocity.reduce((n, v) => n + (v > 0 ? 1 : 0), 0);
