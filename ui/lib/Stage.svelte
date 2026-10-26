<script lang="ts">
  // The play surface: light beams rise from every note, over an 88-key keyboard. One canvas, one rAF loop.
  import { onMount } from "svelte";
  import { live, type Beam, type Flare } from "./live";
  import { HIGHEST, LOWEST, hitTest, layout, noteName, type KeyRect } from "./keyboard";
  import type { Practice, PracticeNote } from "./practice";
  import { trackColor } from "./songs";

  let {
    onpress,
    onrelease,
    keyboardHeight = $bindable(0),
  }: {
    onpress: (key: number, velocity: number) => void;
    onrelease: (key: number) => void;
    keyboardHeight?: number;
  } = $props();

  type RGB = [number, number, number];
  const PULSE: RGB = [127, 212, 255];
  const TINT: RGB = [197, 236, 255];
  const WHITE: RGB = [233, 233, 239];
  const WHITE_LIT: RGB = [172, 227, 255];
  const BLACK: RGB = [14, 14, 19];
  const CORAL: RGB = [255, 122, 89];
  const VOID: RGB = [10, 10, 15];
  const GREY: RGB = [70, 70, 82];
  const SNOW: RGB = [242, 242, 247];
  /** How fast light travels up, px per second. */
  const SPEED = 240;
  /** Falling notes: px per second of song. */
  const FALL = 150;
  /** Note tape: px per second of song, and the playhead's position as a fraction of the width. */
  const TAPE = 120;
  const PLAYHEAD = 0.28;
  /** Room the practice controls take at the top of the stage, and the gap kept above the keyboard. */
  const OVERLAY_TOP = 72;
  const OVERLAY_BOTTOM = 48;

  const rgba = (c: RGB, a: number) => `rgba(${c[0]},${c[1]},${c[2]},${a})`;
  const mix = (a: RGB, b: RGB, t: number) =>
    `rgb(${a.map((v, i) => Math.round(v + (b[i] - v) * t)).join(",")})`;

  let canvas: HTMLCanvasElement;
  let ctx: CanvasRenderingContext2D;
  let w = 0, h = 0, dpr = 1, kbY = 0, kbH = 0;
  let keys: KeyRect[] = [];
  let whites: KeyRect[] = [];
  let blacks: KeyRect[] = [];
  let energy = 0;
  let sustainGlow = 0;

  function resize() {
    const r = canvas.getBoundingClientRect();
    dpr = window.devicePixelRatio || 1;
    w = r.width;
    h = r.height;
    canvas.width = Math.round(w * dpr);
    canvas.height = Math.round(h * dpr);
    kbH = Math.round(Math.min(220, Math.max(120, h * 0.26)));
    kbY = h - kbH;
    keyboardHeight = kbH;
    keys = layout(w, kbH);
    whites = keys.filter((k) => k && !k.black);
    blacks = keys.filter((k) => k && k.black);
  }

  function draw(now: number, dt: number) {
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, w, h);

    // How much is sounding, smoothed: rises fast, settles slowly. Drives the floor light.
    let sounding = 0;
    for (let k = LOWEST; k <= HIGHEST; k++) sounding += live.velocity[k] / 127;
    const target = Math.min(sounding / 5, 1);
    energy += (target - energy) * Math.min(1, dt * (target > energy ? 10 : 1.5));

    const floorH = Math.min(340, kbY);
    const floor = ctx.createLinearGradient(0, kbY - floorH, 0, kbY);
    floor.addColorStop(0, rgba(PULSE, 0));
    floor.addColorStop(1, rgba(PULSE, 0.035 + 0.17 * energy));
    ctx.fillStyle = floor;
    ctx.fillRect(0, kbY - floorH, w, floorH);

    const practice = live.practice;
    if (practice) {
      practice.update(dt);
      if (live.songView === "falling") drawFalling(practice);
      else drawTape(practice);
    }

    ctx.globalCompositeOperation = "lighter";
    live.beams = live.beams.filter((b) => drawBeam(b, now));
    live.flares = live.flares.filter((f) => drawFlare(f, now));
    ctx.globalCompositeOperation = "source-over";

    drawKeyboard(dt);
    if (practice) {
      drawGuides(practice, now);
      drawProgress(practice);
    }
  }

  /** Song progress as light along the keyboard's top edge. */
  function drawProgress(p: Practice) {
    const span = p.duration - p.startTime || 1;
    const x = Math.max(0, Math.min(1, (p.time - p.startTime) / span)) * w;
    ctx.fillStyle = "rgba(255,255,255,0.07)";
    ctx.fillRect(x, kbY - 3, w - x, 3);
    ctx.shadowColor = rgba(PULSE, 0.9);
    ctx.shadowBlur = 12;
    ctx.fillStyle = rgba(TINT, 0.95);
    ctx.fillRect(0, kbY - 3, x, 3);
    ctx.shadowBlur = 0;
  }

  /** Sets fill and alpha for a song note: bright when hit, grey when missed. */
  function styleNote(n: PracticeNote) {
    const color = trackColor(n.track);
    ctx.globalAlpha = n.state === "missed" ? 0.45 : 1;
    ctx.fillStyle =
      n.state === "hit" ? mix(color, SNOW, 0.6) : n.state === "missed" ? mix(color, GREY, 0.7) : rgba(color, 0.92);
  }

  /** A note name in a small pill, so it reads even on narrow black-key notes. */
  function label(name: string, cx: number, cy: number, fill: string) {
    const tw = ctx.measureText(name).width + 8;
    ctx.fillStyle = fill;
    ctx.beginPath();
    ctx.roundRect(cx - tw / 2, cy - 7, tw, 14, 4);
    ctx.fill();
    ctx.fillStyle = rgba(VOID, 0.9);
    ctx.fillText(name, cx, cy + 0.5);
  }

  function drawFalling(p: Practice) {
    const ahead = kbY / FALL;
    ctx.fillStyle = "rgba(255,255,255,0.05)";
    for (const bar of p.bars) {
      const y = kbY - (bar - p.time) * FALL;
      if (y >= 0 && y <= kbY) ctx.fillRect(0, Math.round(y), w, 1);
    }
    ctx.font = "600 10px 'Inter Variable', sans-serif";
    ctx.textAlign = "center";
    ctx.textBaseline = "middle";
    // Tracks you switched off aren't drawn at all.
    for (const n of p.notes) {
      if (n.end < p.time || n.start > p.time + ahead || !p.isActive(n)) continue;
      const r = keys[n.key];
      if (!r) continue;
      const bottom = Math.min(kbY, kbY - (n.start - p.time) * FALL);
      const top = kbY - (n.end - p.time) * FALL;
      if (bottom - top < 1) continue;
      const x = r.black ? r.x : r.x + 2;
      const nw = r.black ? r.w : r.w - 4;
      styleNote(n);
      const fill = ctx.fillStyle as string;
      ctx.beginPath();
      ctx.roundRect(x, top, nw, bottom - top, Math.min(5, nw / 2));
      ctx.fill();
      if (n.state === "pending" && bottom - top > 16) label(noteName(n.key), x + nw / 2, bottom - 10, fill);
    }
    ctx.globalAlpha = 1;
  }

  /** The tape's key range fits the tracks being shown, and is worked out again when they change. */
  let tapeRange = { practice: null as Practice | null, shown: null as Set<number> | null, low: 48, high: 84 };

  function drawTape(p: Practice) {
    if (tapeRange.practice !== p || tapeRange.shown !== p.shown) {
      const shown = p.notes.filter((n) => p.isActive(n));
      const keysUsed = (shown.length ? shown : p.notes).map((n) => n.key);
      tapeRange = { practice: p, shown: p.shown, low: Math.min(...keysUsed), high: Math.max(...keysUsed) };
    }
    const top = OVERLAY_TOP + 16;
    const bottom = kbY - OVERLAY_BOTTOM - 16;
    const span = Math.max(12, tapeRange.high - tapeRange.low);
    const yOf = (key: number) => bottom - ((key - tapeRange.low) / span) * (bottom - top);
    const playX = w * PLAYHEAD;

    ctx.fillStyle = "rgba(255,255,255,0.05)";
    for (const bar of p.bars) {
      const x = playX + (bar - p.time) * TAPE;
      if (x >= 0 && x <= w) ctx.fillRect(Math.round(x), top - 12, 1, bottom - top + 24);
    }
    ctx.fillStyle = rgba(PULSE, 0.55);
    ctx.fillRect(playX - 1, top - 20, 2, bottom - top + 40);

    ctx.font = "600 12px 'Inter Variable', sans-serif";
    ctx.textAlign = "left";
    ctx.textBaseline = "middle";
    for (const n of p.notes) {
      const x0 = playX + (n.start - p.time) * TAPE;
      const x1 = playX + (n.end - p.time) * TAPE;
      if (x1 < 0 || x0 > w || !p.isActive(n)) continue;
      const name = noteName(n.key);
      const width = Math.max(x1 - x0, ctx.measureText(name).width + 12);
      const y = yOf(n.key);
      styleNote(n);
      ctx.beginPath();
      ctx.roundRect(x0, y - 10, width, 20, 6);
      ctx.fill();
      ctx.fillStyle = rgba(VOID, 0.9);
      ctx.fillText(name, x0 + 6, y + 0.5);
    }
    ctx.globalAlpha = 1;
  }

  /** Lights the keys to play now: pulsing for the chord being waited on, softly for sounding notes. */
  function drawGuides(p: Practice, now: number) {
    const waiting = p.waiting;
    const due = waiting
      ? waiting.notes.filter((n) => p.isActive(n) && n.state === "pending")
      : p.notes.filter((n) => n.start <= p.time && n.end > p.time && p.isActive(n) && n.state !== "missed");
    const strength = waiting ? 0.35 + 0.25 * Math.sin(now / 140) : 0.3;
    for (const n of due) {
      const r = keys[n.key];
      if (!r || live.velocity[n.key] > 0) continue;
      const x = r.black ? r.x : r.x + 1;
      const kw = r.black ? r.w : r.w - 2;
      ctx.fillStyle = rgba(trackColor(n.track), strength);
      ctx.beginPath();
      ctx.roundRect(x, kbY, kw, r.h, [0, 0, 5, 5]);
      ctx.fill();
    }
    for (let k = LOWEST; k <= HIGHEST; k++) {
      const age = (now - p.wrongAt[k]) / 300;
      const r = keys[k];
      if (age >= 1 || !r) continue;
      ctx.fillStyle = rgba(CORAL, 0.6 * (1 - age));
      ctx.beginPath();
      ctx.roundRect(r.black ? r.x : r.x + 1, kbY, r.black ? r.w : r.w - 2, r.h, [0, 0, 5, 5]);
      ctx.fill();
    }
  }

  function drawBeam(b: Beam, now: number): boolean {
    const r = keys[b.key];
    if (!r) return false;
    const v = b.velocity / 127;
    const grown = (((b.released ?? now) - b.born) / 1000) * SPEED;
    const drift = b.released === null ? 0 : ((now - b.released) / 1000) * SPEED;
    const fade = b.released === null ? 1 : 1 - (now - b.released) / 2600;
    const bottom = kbY - drift;
    if (bottom < 0 || fade <= 0) return false;
    const top = bottom - Math.max(grown, 6);
    const a = (0.35 + 0.65 * v) * fade;
    const cx = r.x + r.w / 2;
    const bw = (r.black ? r.w * 0.9 : r.w * 0.55) * (0.6 + 0.4 * v);

    const glow = ctx.createLinearGradient(0, top, 0, bottom);
    glow.addColorStop(0, rgba(PULSE, 0));
    glow.addColorStop(1, rgba(PULSE, 0.22 * a));
    ctx.fillStyle = glow;
    ctx.beginPath();
    ctx.roundRect(cx - bw * 1.8, top, bw * 3.6, bottom - top, bw * 1.8);
    ctx.fill();

    const core = ctx.createLinearGradient(0, top, 0, bottom);
    core.addColorStop(0, rgba(PULSE, 0));
    core.addColorStop(0.6, rgba(PULSE, 0.75 * a));
    core.addColorStop(1, rgba(TINT, a));
    ctx.fillStyle = core;
    ctx.beginPath();
    ctx.roundRect(cx - bw / 2, top, bw, bottom - top, bw / 2);
    ctx.fill();
    return true;
  }

  function drawFlare(f: Flare, now: number): boolean {
    const life = 0.7;
    const t = (now - f.born) / 1000;
    const r = keys[f.key];
    if (t > life || !r) return false;
    const v = f.velocity / 127;
    const k = 1 - t / life;
    const cx = r.x + r.w / 2;
    const radius = r.w * (2 + 5 * t) * (0.6 + 0.6 * v);
    const g = ctx.createRadialGradient(cx, kbY, 0, cx, kbY, radius);
    g.addColorStop(0, rgba(TINT, 0.55 * v * k * k));
    g.addColorStop(1, rgba(PULSE, 0));
    ctx.fillStyle = g;
    ctx.fillRect(cx - radius, kbY - radius, radius * 2, radius);
    return true;
  }

  function drawKeyboard(dt: number) {
    const decay = Math.exp(-dt * 9);
    ctx.fillStyle = "#050508";
    ctx.fillRect(0, kbY, w, kbH);
    for (const r of whites) drawKey(r, decay, WHITE, WHITE_LIT, 6);
    // Every C says which one it is (C4 is middle C), small, near the front of the key.
    ctx.font = "600 9px 'Inter Variable', sans-serif";
    ctx.textAlign = "center";
    ctx.textBaseline = "alphabetic";
    ctx.fillStyle = "rgba(10,10,15,0.42)";
    for (const r of whites) if (r.key % 12 === 0 && r.w >= 12) ctx.fillText(noteName(r.key), r.x + r.w / 2, kbY + r.h - 11);
    for (const r of blacks) drawKey(r, decay, BLACK, PULSE, 4);

    // Depth: the keys slide under the stage edge.
    const shade = ctx.createLinearGradient(0, kbY, 0, kbY + 14);
    shade.addColorStop(0, "rgba(0,0,0,0.45)");
    shade.addColorStop(1, "rgba(0,0,0,0)");
    ctx.fillStyle = shade;
    ctx.fillRect(0, kbY, w, 14);

    // Sustain pedal: the keyboard's top edge lights up.
    sustainGlow += ((live.sustain ? 1 : 0) - sustainGlow) * Math.min(1, dt * 10);
    if (sustainGlow > 0.01) {
      const g = ctx.createLinearGradient(0, kbY - 22, 0, kbY);
      g.addColorStop(0, rgba(PULSE, 0));
      g.addColorStop(1, rgba(PULSE, 0.45 * sustainGlow));
      ctx.fillStyle = g;
      ctx.fillRect(0, kbY - 22, w, 22);
      ctx.fillStyle = rgba(TINT, 0.9 * sustainGlow);
      ctx.fillRect(0, kbY - 1.5, w, 1.5);
    }
  }

  function drawKey(r: KeyRect, decay: number, rest: RGB, lit: RGB, radius: number) {
    if (live.velocity[r.key] === 0) live.glow[r.key] *= decay;
    const g = live.glow[r.key];
    const x = r.black ? r.x : r.x + 1;
    const kw = r.black ? r.w : r.w - 2;
    if (g > 0.02) {
      ctx.shadowColor = rgba(PULSE, 0.9 * g);
      ctx.shadowBlur = 28 * g;
    }
    ctx.fillStyle = mix(rest, lit, g);
    ctx.beginPath();
    ctx.roundRect(x, kbY, kw, r.h, [0, 0, radius, radius]);
    ctx.fill();
    ctx.shadowBlur = 0;
    // A lip at the front edge that disappears as the key goes down.
    ctx.fillStyle = r.black ? `rgba(60,60,72,${0.8 * (1 - g)})` : `rgba(0,0,0,${0.1 * (1 - g)})`;
    ctx.fillRect(x + (r.black ? 2 : 0), kbY + r.h - (r.black ? 6 : 5), kw - (r.black ? 4 : 0), r.black ? 3 : 5);
  }

  // Pointer input: press, glissando across keys, release. Velocity follows how far down the key you click.
  const pointers = new Map<number, number | null>();

  function keyAt(e: PointerEvent): { key: number; velocity: number } | null {
    const box = canvas.getBoundingClientRect();
    const x = e.clientX - box.left;
    const y = e.clientY - box.top - kbY;
    const r = hitTest(keys, x, y);
    return r ? { key: r.key, velocity: Math.round(36 + 91 * Math.min(1, y / r.h)) } : null;
  }

  function down(e: PointerEvent) {
    const hit = keyAt(e);
    if (!hit) return;
    canvas.setPointerCapture(e.pointerId);
    pointers.set(e.pointerId, hit.key);
    onpress(hit.key, hit.velocity);
  }

  function move(e: PointerEvent) {
    canvas.style.cursor = e.clientY - canvas.getBoundingClientRect().top >= kbY ? "pointer" : "default";
    if (!pointers.has(e.pointerId)) return;
    const prev = pointers.get(e.pointerId);
    const hit = keyAt(e);
    if ((hit?.key ?? null) === prev) return;
    if (prev != null) onrelease(prev);
    if (hit) onpress(hit.key, hit.velocity);
    pointers.set(e.pointerId, hit?.key ?? null);
  }

  function up(e: PointerEvent) {
    const prev = pointers.get(e.pointerId);
    if (prev != null) onrelease(prev);
    pointers.delete(e.pointerId);
  }

  onMount(() => {
    ctx = canvas.getContext("2d")!;
    const observer = new ResizeObserver(resize);
    observer.observe(canvas);
    resize();
    let last = performance.now();
    let raf = requestAnimationFrame(function frame(now) {
      draw(now, Math.min(0.1, (now - last) / 1000));
      last = now;
      raf = requestAnimationFrame(frame);
    });
    return () => {
      cancelAnimationFrame(raf);
      observer.disconnect();
    };
  });
</script>

<canvas
  bind:this={canvas}
  onpointerdown={down}
  onpointermove={move}
  onpointerup={up}
  onpointercancel={up}
  aria-label="Piano keyboard"
></canvas>

<style>
  canvas {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    touch-action: none;
  }
</style>
