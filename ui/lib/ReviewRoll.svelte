<script lang="ts">
  // The review's piano roll: the song's notes as outlines, what you played filled in over them, coloured by
  // how it went. Time runs left to right; the wheel scrolls, Ctrl+wheel zooms, hovering a note explains it,
  // clicking moves the playhead.
  import { onMount, untrack } from "svelte";
  import { isBlack, noteName } from "./keyboard";
  import type { Mode, PlayedNote } from "./practice";
  import type { NoteReview, Review } from "./review";

  let {
    review,
    bars,
    mode,
    playhead = null,
    onseek,
  }: {
    review: Review;
    bars: number[];
    mode: Mode;
    /** Where playback is, in song seconds; the roll keeps it in view. */
    playhead?: number | null;
    onseek?: (time: number) => void;
  } = $props();

  type RGB = [number, number, number];
  const GOOD: RGB = [110, 231, 183];
  const OFF: RGB = [255, 181, 71];
  const BAD: RGB = [255, 122, 89];
  const rgba = (c: RGB, a: number) => `rgba(${c[0]},${c[1]},${c[2]},${a})`;
  /** Room on the left for note names. */
  const GUTTER = 40;
  /** Seconds of air before the first and after the last note. */
  const AIR = 0.4;

  let canvas: HTMLCanvasElement;
  let ctx: CanvasRenderingContext2D;
  let w = $state(0);
  let h = $state(0);
  let dpr = 1;
  /** Seconds at the left edge, and pixels per second. */
  let scroll = $state(0);
  let pps = $state(60);
  let hover = $state<{ x: number; y: number; text: string } | null>(null);

  const span = $derived.by(() => {
    const all = [...review.notes, ...review.extra];
    const keys = all.map((n) => n.key);
    const ends = all.map((n) => ("played" in n && n.played ? Math.max(n.end, n.played.end) : n.end));
    return {
      start: Math.min(0, ...all.map((n) => n.start)) - AIR,
      end: Math.max(0, ...ends) + AIR,
      low: (keys.length ? Math.min(...keys) : 60) - 1,
      high: (keys.length ? Math.max(...keys) : 72) + 1,
    };
  });
  const rowH = $derived(h / (span.high - span.low + 1));
  const visible = $derived((w - GUTTER) / pps);

  const xOf = (t: number) => GUTTER + (t - scroll) * pps;
  const yOf = (key: number) => (span.high - key) * rowH;

  /** Shows the whole pass when it fits, else starts at the beginning at a readable zoom. */
  function fit() {
    pps = Math.max(40, (w - GUTTER) / (span.end - span.start));
    scroll = span.start;
  }

  function clampScroll() {
    scroll = Math.min(Math.max(scroll, span.start), Math.max(span.start, span.end - visible));
  }

  function resize() {
    const box = canvas.getBoundingClientRect();
    dpr = window.devicePixelRatio || 1;
    w = box.width;
    h = box.height;
    canvas.width = Math.round(w * dpr);
    canvas.height = Math.round(h * dpr);
  }

  function rect(x0: number, x1: number, key: number, inset: number) {
    const y = yOf(key) + inset;
    ctx.beginPath();
    ctx.roundRect(x0, y, Math.max(3, x1 - x0), Math.max(2, rowH - inset * 2), Math.min(4, rowH / 3));
  }

  function draw() {
    if (!ctx || !w) return;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, w, h);

    // Rows: black keys darker, C rows named in the gutter.
    ctx.font = "500 10px 'JetBrains Mono', monospace";
    ctx.textBaseline = "middle";
    for (let k = span.low; k <= span.high; k++) {
      if (isBlack(k)) {
        ctx.fillStyle = "rgba(0,0,0,0.28)";
        ctx.fillRect(GUTTER, yOf(k), w - GUTTER, rowH);
      }
      if (k % 12 === 0 || k === span.low + 1) {
        ctx.fillStyle = "rgba(138,138,150,0.85)";
        ctx.fillText(noteName(k), 6, yOf(k) + rowH / 2);
      }
    }
    ctx.save();
    ctx.beginPath();
    ctx.rect(GUTTER, 0, w - GUTTER, h);
    ctx.clip();

    ctx.fillStyle = "rgba(255,255,255,0.06)";
    for (const bar of bars) {
      const x = Math.round(xOf(bar));
      if (x >= GUTTER && x <= w) ctx.fillRect(x, 0, 1, h);
    }

    const inset = Math.max(1, rowH * 0.12);
    const inner = Math.max(2, rowH * 0.3);
    for (const n of review.notes) {
      const x0 = xOf(n.start);
      const x1 = xOf(n.end);
      if (x1 < GUTTER || x0 > w) continue;
      // The note as written.
      rect(x0, x1, n.key, inset);
      ctx.fillStyle = "rgba(255,255,255,0.05)";
      ctx.fill();
      ctx.setLineDash(n.timing === "missed" ? [4, 3] : []);
      ctx.strokeStyle = n.timing === "missed" ? rgba(BAD, 0.9) : "rgba(255,255,255,0.4)";
      ctx.lineWidth = 1.25;
      ctx.stroke();
      ctx.setLineDash([]);
      if (!n.played) continue;
      // What you played, and a line from where it should have started.
      const color = n.timing === "good" ? GOOD : OFF;
      const px0 = xOf(n.played.start);
      if (mode !== "wait" && n.timing !== "good") {
        ctx.fillStyle = rgba(OFF, 0.7);
        ctx.fillRect(Math.min(x0, px0), yOf(n.key) + rowH / 2 - 0.5, Math.abs(px0 - x0), 1);
      }
      rect(px0, xOf(n.played.end), n.key, inner);
      ctx.fillStyle = rgba(color, 0.9);
      ctx.fill();
    }
    for (const p of review.extra) {
      rect(xOf(p.start), xOf(p.end), p.key, inner);
      ctx.fillStyle = rgba(BAD, 0.9);
      ctx.fill();
    }
    if (playhead !== null) {
      const x = Math.round(xOf(playhead));
      ctx.shadowColor = "rgba(127,212,255,0.9)";
      ctx.shadowBlur = 10;
      ctx.fillStyle = "rgba(197,236,255,0.95)";
      ctx.fillRect(x - 1, 0, 2, h);
      ctx.shadowBlur = 0;
    }
    ctx.restore();
  }

  // When the playhead moves out of view, the roll follows it (a quarter of the way in). Only the playhead
  // moving triggers this, so scrolling away while paused stays put.
  $effect(() => {
    const at = playhead;
    untrack(() => {
      if (at === null || !w) return;
      const x = xOf(at);
      if (x < GUTTER || x > w - 24) {
        scroll = at - visible / 4;
        clampScroll();
      }
    });
  });

  $effect(() => {
    void review;
    void w;
    fit();
  });
  $effect(() => {
    void [scroll, pps, rowH, review, bars, playhead];
    draw();
  });

  function down(e: PointerEvent) {
    const x = e.clientX - canvas.getBoundingClientRect().left;
    if (x > GUTTER) onseek?.(Math.min(span.end, Math.max(span.start, scroll + (x - GUTTER) / pps)));
  }

  const ms = (seconds: number) => `${Math.round(Math.abs(seconds) * 1000)} ms`;

  function describe(n: NoteReview): string {
    const parts = [noteName(n.key)];
    if (n.timing === "missed") return `${parts[0]} · missed`;
    if (mode === "wait") parts.push(n.offset > 0.05 ? `waited ${n.offset.toFixed(1)} s` : "no wait");
    else parts.push(n.timing === "good" ? "on time" : `${ms(n.offset)} ${n.timing}`);
    if (n.length === "short") parts.push("let go too soon");
    if (n.length === "long") parts.push("held too long");
    return parts.join(" · ");
  }

  function pointer(e: PointerEvent) {
    const box = canvas.getBoundingClientRect();
    const x = e.clientX - box.left;
    const y = e.clientY - box.top;
    const t = scroll + (x - GUTTER) / pps;
    const key = Math.round(span.high - (y - rowH / 2) / rowH);
    const inside = (from: number, to: number) => t >= from - 4 / pps && t <= to + 4 / pps;
    const extra = review.extra.find((p: PlayedNote) => p.key === key && inside(p.start, p.end));
    const note = review.notes.find(
      (n) => n.key === key && (inside(n.start, n.end) || (n.played && inside(n.played.start, n.played.end))),
    );
    const text = note ? describe(note) : extra ? `${noteName(extra.key)} · not in the song here` : null;
    hover = text && x > GUTTER ? { x, y, text } : null;
  }

  function wheel(e: WheelEvent) {
    e.preventDefault();
    if (e.ctrlKey) {
      const box = canvas.getBoundingClientRect();
      const anchor = scroll + (e.clientX - box.left - GUTTER) / pps;
      pps = Math.min(600, Math.max((w - GUTTER) / (span.end - span.start), pps * (e.deltaY < 0 ? 1.15 : 1 / 1.15)));
      scroll = anchor - (e.clientX - box.left - GUTTER) / pps;
    } else {
      scroll += (e.deltaY + e.deltaX) / pps;
    }
    clampScroll();
  }

  onMount(() => {
    ctx = canvas.getContext("2d")!;
    const observer = new ResizeObserver(resize);
    observer.observe(canvas);
    resize();
    return () => observer.disconnect();
  });
</script>

<div class="roll">
  <canvas
    bind:this={canvas}
    onpointerdown={down}
    onpointermove={pointer}
    onpointerleave={() => (hover = null)}
    onwheel={wheel}
    aria-label="Your playing compared with the song"
  ></canvas>
  {#if hover}
    <div class="tip" style:left="{hover.x}px" style:top="{hover.y}px">{hover.text}</div>
  {/if}
</div>

<style>
  .roll {
    position: relative;
    min-height: 0;
    border-radius: 14px;
    background: rgba(10, 10, 15, 0.7);
    border: 1px solid var(--line);
    overflow: hidden;
  }
  canvas {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    cursor: pointer;
  }
  .tip {
    position: absolute;
    transform: translate(-50%, calc(-100% - 10px));
    padding: 5px 9px;
    border-radius: 8px;
    background: rgba(22, 22, 29, 0.96);
    border: 1px solid var(--line-strong);
    font-size: 12px;
    white-space: nowrap;
    pointer-events: none;
  }
</style>
