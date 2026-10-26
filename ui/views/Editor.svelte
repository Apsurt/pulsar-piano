<script lang="ts">
  // Song editor: a piano roll laid out like the falling-notes view (keys along the bottom, time going up),
  // with three ways in: click/drag on the roll, step entry from the keyboard, and typed note names.
  // Changes save automatically.
  import { onDestroy, onMount } from "svelte";
  import Segmented from "../lib/Segmented.svelte";
  import Sheet from "../lib/Sheet.svelte";
  import TopBar from "../lib/TopBar.svelte";
  import { leaveEditor, library, nav } from "../lib/app.svelte";
  import { onAction } from "../lib/controls";
  import { engine } from "../lib/engine";
  import { HIGHEST, LOWEST, layout, noteName, type KeyRect } from "../lib/keyboard";
  import { heldCount, live } from "../lib/live";
  import { NotationError, parseNotation } from "../lib/notation";
  import { blankSong, songs, type Hand, type Song, type SongNote } from "../lib/songs";

  let { id, song }: { id: string | null; song: Song | null } = $props();

  // The app re-creates this view per song. The document is a plain object mutated in place;
  // `version` tells the UI something changed.
  // svelte-ignore state_referenced_locally
  let songId = $state<string | null>(id);
  // svelte-ignore state_referenced_locally
  const doc: Song = structuredClone($state.snapshot(song) ?? blankSong());
  let version = $state(0);
  const noteCount = $derived.by(() => {
    void version;
    return doc.notes.length;
  });

  let title = $state(doc.title);
  let artist = $state(doc.artist);
  let bpm = $state(Math.round(doc.tempos[0]?.bpm ?? 90));
  let meter = $state(`${doc.time_signatures[0]?.numerator ?? 4}/${doc.time_signatures[0]?.denominator ?? 4}`);

  /** Lengths in beats (quarter notes). */
  let stepLength = $state("1");
  let dotted = $state(false);
  let hand = $state<Hand>("right");
  /** Where step entry and typed notes go, in beats. */
  let cursor = $state(0);
  let stepInput = $state(false);
  let typing = $state(false);
  let typed = $state("");
  let typedError = $state<string | null>(null);
  let saveState = $state<"saved" | "saving" | "unsaved" | "failed">("saved");
  let selected: SongNote | null = null;

  const length = $derived(Number(stepLength) * (dotted ? 1.5 : 1));
  const beatsPerBar = $derived.by(() => {
    const [n, d] = meter.split("/").map(Number);
    return (n * 4) / d;
  });

  // ---- History and saving ------------------------------------------------------------------

  const undoStack: string[] = [];
  const redoStack: string[] = [];
  const snapshot = () => JSON.stringify(doc.notes);

  /** Call before a change to the notes, so it can be undone. */
  function remember() {
    undoStack.push(snapshot());
    if (undoStack.length > 200) undoStack.shift();
    redoStack.length = 0;
  }

  function restore(from: string[], to: string[]) {
    const state = from.pop();
    if (state === undefined) return;
    to.push(snapshot());
    doc.notes = JSON.parse(state);
    selected = null;
    changed();
  }
  const undo = () => restore(undoStack, redoStack);
  const redo = () => restore(redoStack, undoStack);

  function changed() {
    version++;
    saveState = "unsaved";
    clearTimeout(saveTimer);
    saveTimer = setTimeout(save, 700);
  }

  let saveTimer: ReturnType<typeof setTimeout>;
  let saving = Promise.resolve();

  /** Saves are chained so a new song is only ever created once. */
  function save(): Promise<void> {
    clearTimeout(saveTimer);
    saving = saving.then(async () => {
      if (saveState === "saved") return;
      doc.title = title.trim() || "Untitled";
      doc.artist = artist.trim();
      doc.tempos[0] = { beat: 0, bpm: Math.min(300, Math.max(20, bpm || 90)) };
      const [numerator, denominator] = meter.split("/").map(Number);
      doc.time_signatures[0] = { beat: 0, numerator, denominator };
      // An untouched new song isn't worth a file.
      if (!songId && doc.notes.length === 0 && doc.title === "Untitled") {
        saveState = "saved";
        return;
      }
      saveState = "saving";
      try {
        songId = (await songs.save(songId, doc)).id;
        saveState = "saved";
        library.version++;
      } catch {
        saveState = "failed";
      }
    });
    return saving;
  }

  onDestroy(() => {
    save();
  });

  /** Leave for practice (or go back) once the last edits are saved. */
  async function leave(practice: boolean) {
    await save();
    leaveEditor(songId, practice);
  }

  let infoOpen = $state(false);
  let helpOpen = $state(false);

  // ---- Note operations -----------------------------------------------------------------------

  const SNAP = 0.25;
  const end = () => doc.notes.reduce((e, n) => Math.max(e, n.start + n.length), 0);

  function addNote(key: number, start: number, len: number): SongNote {
    const note: SongNote = { key, start, length: len, velocity: 90, hand };
    doc.notes.push(note);
    return note;
  }

  function remove(note: SongNote) {
    remember();
    doc.notes = doc.notes.filter((n) => n !== note);
    if (selected === note) selected = null;
    changed();
  }

  /** A short blip of the note, so placing it is audible. */
  function audition(key: number) {
    engine.note(key, 80);
    setTimeout(() => engine.note(key, 0), 220);
  }

  // ---- Step entry: play a key or chord on the keyboard, it lands at the cursor ----------------

  let chordStart: number | null = null;

  function onNote(key: number, velocity: number) {
    if (!stepInput) return;
    if (velocity > 0) {
      if (chordStart === null) {
        chordStart = cursor;
        remember();
      }
      const at = chordStart;
      if (!doc.notes.some((n) => n.key === key && n.start === at)) addNote(key, at, length);
      version++;
    } else if (chordStart !== null && heldCount() === 0) {
      cursor = chordStart + length;
      chordStart = null;
      reveal(cursor);
      changed();
    }
  }

  function toggleStepInput() {
    stepInput = !stepInput;
    chordStart = null;
    if (stepInput) reveal(cursor);
  }

  function rest() {
    cursor += length;
    reveal(cursor);
  }

  // ---- Typed notes ----------------------------------------------------------------------------

  function addTyped() {
    try {
      const parsed = parseNotation(typed, length);
      if (!parsed.notes.length && !parsed.beats) return;
      remember();
      for (const n of parsed.notes) addNote(n.key, cursor + n.start, n.length);
      cursor += parsed.beats;
      typed = "";
      typedError = null;
      reveal(cursor);
      changed();
    } catch (e) {
      typedError = e instanceof NotationError ? e.message : String(e);
    }
  }

  // ---- Piano roll -----------------------------------------------------------------------------

  type RGB = [number, number, number];
  const PULSE: RGB = [127, 212, 255];
  const AMBER: RGB = [255, 181, 71];
  const rgba = (c: RGB, a: number) => `rgba(${c[0]},${c[1]},${c[2]},${a})`;
  /** Room below beat 0. */
  const PAD = 30;

  let canvas: HTMLCanvasElement;
  let ctx: CanvasRenderingContext2D;
  let w = 0, h = 0, dpr = 1;
  let keys: KeyRect[] = [];
  /** Beat at the bottom edge, and pixels per beat. */
  let scroll = 0;
  let ppb = 56;
  let hover: { key: number; beat: number } | null = null;

  const yOf = (beat: number) => h - PAD - (beat - scroll) * ppb;
  const beatAt = (y: number) => scroll + (h - PAD - y) / ppb;
  const floorSnap = (beat: number) => Math.max(0, Math.floor(beat / SNAP + 1e-6) * SNAP);
  const roundSnap = (beat: number) => Math.round(beat / SNAP) * SNAP;

  /** Keep `beat` comfortably on screen. */
  function reveal(beat: number) {
    const y = yOf(beat);
    if (y < h * 0.25 || y > h - PAD) scroll = Math.max(0, beat - (h - PAD) / ppb / 2);
  }

  /** The key column under x: black keys are narrower and sit on top. */
  function keyAt(x: number): number | null {
    let white: number | null = null;
    for (let k = LOWEST; k <= HIGHEST; k++) {
      const r = keys[k];
      if (!r || x < r.x || x >= r.x + r.w) continue;
      if (r.black) return k;
      white = k;
    }
    return white;
  }

  function hitNote(x: number, y: number): { note: SongNote; edge: boolean } | null {
    for (let i = doc.notes.length - 1; i >= 0; i--) {
      const n = doc.notes[i];
      const r = keys[n.key];
      if (!r) continue;
      const top = yOf(n.start + n.length);
      const bottom = yOf(n.start);
      if (x >= r.x && x < r.x + r.w && y >= top - 2 && y <= bottom) return { note: n, edge: y - top < 7 };
    }
    return null;
  }

  function resize() {
    const box = canvas.getBoundingClientRect();
    dpr = window.devicePixelRatio || 1;
    w = box.width;
    h = box.height;
    canvas.width = Math.round(w * dpr);
    canvas.height = Math.round(h * dpr);
    keys = layout(w, 1);
  }

  function draw(now: number) {
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.fillStyle = "#0a0a0f";
    ctx.fillRect(0, 0, w, h);

    // Key columns, lined up with the keyboard below: black-key lanes a touch darker, C lanes labelled.
    ctx.font = "500 10px 'JetBrains Mono', monospace";
    ctx.textBaseline = "alphabetic";
    for (let k = LOWEST; k <= HIGHEST; k++) {
      const r = keys[k];
      if (!r) continue;
      if (r.black) {
        ctx.fillStyle = "rgba(0,0,0,0.35)";
        ctx.fillRect(r.x, 0, r.w, h);
      } else if (k % 12 === 0) {
        ctx.fillStyle = "rgba(255,255,255,0.07)";
        ctx.fillRect(Math.round(r.x), 0, 1, h);
        ctx.fillStyle = "rgba(138,138,150,0.8)";
        ctx.fillText(noteName(k), r.x + 3, h - 10);
      }
    }

    // Beats and bars.
    const top = beatAt(0);
    for (let b = Math.max(0, Math.floor(scroll)); b <= top; b++) {
      const y = Math.round(yOf(b));
      const bar = Math.abs(b / beatsPerBar - Math.round(b / beatsPerBar)) < 1e-6;
      ctx.fillStyle = bar ? "rgba(255,255,255,0.12)" : "rgba(255,255,255,0.04)";
      ctx.fillRect(0, y, w, 1);
      if (bar) {
        ctx.fillStyle = "rgba(138,138,150,0.9)";
        ctx.fillText(String(Math.round(b / beatsPerBar) + 1), 6, y - 4);
      }
    }

    // Notes.
    ctx.font = "600 10px 'Inter Variable', sans-serif";
    ctx.textAlign = "center";
    ctx.textBaseline = "middle";
    for (const n of doc.notes) {
      const r = keys[n.key];
      if (!r) continue;
      const y0 = yOf(n.start + n.length);
      const y1 = yOf(n.start);
      if (y1 < 0 || y0 > h) continue;
      const x = r.black ? r.x : r.x + 2;
      const nw = r.black ? r.w : r.w - 4;
      ctx.fillStyle = rgba(n.hand === "left" ? AMBER : PULSE, 0.92);
      ctx.beginPath();
      ctx.roundRect(x, y0 + 1, nw, Math.max(4, y1 - y0 - 2), Math.min(5, nw / 2));
      ctx.fill();
      if (n === selected) {
        ctx.strokeStyle = "#ffffff";
        ctx.lineWidth = 2;
        ctx.stroke();
      }
      if (y1 - y0 > 18 && nw >= 18) {
        ctx.fillStyle = "rgba(10,10,15,0.9)";
        ctx.fillText(noteName(n.key), x + nw / 2, y1 - 9);
      }
    }
    ctx.textAlign = "left";

    // Where a click would put a note.
    if (hover && !drag) {
      const r = keys[hover.key];
      if (r) {
        ctx.fillStyle = rgba(hand === "left" ? AMBER : PULSE, 0.22);
        ctx.fillRect(r.x + (r.black ? 0 : 2), yOf(hover.beat + length), r.w - (r.black ? 0 : 4), length * ppb);
      }
    }

    // Cursor: where step entry and typed notes land. Pulses while step input is on.
    const cy = yOf(cursor);
    const a = stepInput ? 0.6 + 0.4 * Math.sin(now / 180) : 0.45;
    ctx.fillStyle = rgba(PULSE, a);
    ctx.fillRect(0, cy - 1, w, 2);
    if (stepInput) {
      ctx.shadowColor = rgba(PULSE, 0.8);
      ctx.shadowBlur = 14;
      ctx.fillRect(0, cy - 1, w, 2);
      ctx.shadowBlur = 0;
    }
  }

  // Pointer: click empty space to add, drag a note to move, drag its top edge to lengthen,
  // right-click to delete, Alt+click to move the cursor. Shift turns off snapping.
  type Drag = {
    mode: "move" | "resize";
    note: SongNote;
    downY: number;
    grabBeat: number;
    grabKey: number;
    start: number;
    key: number;
    changed: boolean;
  };
  let drag: Drag | null = null;

  function point(e: PointerEvent | MouseEvent) {
    const box = canvas.getBoundingClientRect();
    return { x: e.clientX - box.left, y: e.clientY - box.top };
  }

  function down(e: PointerEvent) {
    if (e.button !== 0) return;
    const { x, y } = point(e);
    if (e.altKey) {
      cursor = floorSnap(beatAt(y));
      return;
    }
    canvas.setPointerCapture(e.pointerId);
    remember();
    const hit = hitNote(x, y);
    if (hit) {
      selected = hit.note;
      drag = {
        mode: hit.edge ? "resize" : "move",
        note: hit.note,
        downY: y,
        grabBeat: beatAt(y),
        grabKey: keyAt(x) ?? hit.note.key,
        start: hit.note.start,
        key: hit.note.key,
        changed: false,
      };
      version++;
      return;
    }
    const key = keyAt(x);
    if (key === null) {
      undoStack.pop();
      return;
    }
    const note = addNote(key, floorSnap(beatAt(y)), length);
    selected = note;
    drag = { mode: "resize", note, downY: y, grabBeat: note.start, grabKey: key, start: note.start, key, changed: true };
    audition(key);
    version++;
  }

  function move(e: PointerEvent) {
    const { x, y } = point(e);
    const k = keyAt(x);
    const over = hitNote(x, y);
    hover = k === null || over ? null : { key: k, beat: floorSnap(beatAt(y)) };
    canvas.style.cursor = drag?.mode === "resize" || over?.edge ? "ns-resize" : over ? "grab" : "crosshair";
    if (!drag) return;
    const free = e.shiftKey;
    const n = drag.note;
    if (drag.mode === "move") {
      const delta = beatAt(y) - drag.grabBeat;
      const start = Math.max(0, drag.start + (free ? delta : roundSnap(delta)));
      const key = Math.min(HIGHEST, Math.max(LOWEST, drag.key + ((k ?? drag.grabKey) - drag.grabKey)));
      if (start !== n.start || key !== n.key) {
        if (key !== n.key) audition(key);
        n.start = start;
        n.key = key;
        drag.changed = true;
      }
    } else if (Math.abs(y - drag.downY) > 4) {
      const top = free ? beatAt(y) : Math.ceil(beatAt(y) / SNAP - 1e-6) * SNAP;
      n.length = Math.max(free ? 1 / 32 : SNAP, top - n.start);
      drag.changed = true;
    }
    version++;
  }

  function up() {
    if (drag && !drag.changed) undoStack.pop();
    if (drag?.changed) changed();
    drag = null;
  }

  function contextmenu(e: MouseEvent) {
    e.preventDefault();
    const { x, y } = point(e);
    const hit = hitNote(x, y);
    if (hit) remove(hit.note);
  }

  function wheel(e: WheelEvent) {
    e.preventDefault();
    if (e.ctrlKey) {
      const { y } = point(e);
      const anchor = beatAt(y);
      ppb = Math.min(220, Math.max(14, ppb * (e.deltaY < 0 ? 1.12 : 1 / 1.12)));
      scroll = Math.max(0, anchor - (h - PAD - y) / ppb);
    } else {
      scroll = Math.max(0, scroll - e.deltaY / ppb);
    }
  }

  function keydown(e: KeyboardEvent) {
    if ((e.target as HTMLElement).closest?.("input, textarea")) return;
    const mod = e.ctrlKey || e.metaKey;
    if (mod && e.key.toLowerCase() === "z") {
      e.preventDefault();
      if (e.shiftKey) redo();
      else undo();
    } else if (mod && e.key.toLowerCase() === "y") {
      e.preventDefault();
      redo();
    } else if (e.key === "Delete" || e.key === "Backspace") {
      if (selected) remove(selected);
      else if (stepInput) undo();
    } else if (e.key === "Escape") {
      selected = null;
      stepInput = false;
      version++;
    } else if (selected && e.key.startsWith("Arrow")) {
      e.preventDefault();
      remember();
      const n = selected;
      if (e.key === "ArrowUp") n.start += SNAP;
      if (e.key === "ArrowDown") n.start = Math.max(0, n.start - SNAP);
      if (e.key === "ArrowRight") n.key = Math.min(HIGHEST, n.key + 1);
      if (e.key === "ArrowLeft") n.key = Math.max(LOWEST, n.key - 1);
      if (e.key === "ArrowRight" || e.key === "ArrowLeft") audition(n.key);
      changed();
    }
  }

  onMount(() => {
    ctx = canvas.getContext("2d")!;
    const observer = new ResizeObserver(resize);
    observer.observe(canvas);
    resize();
    cursor = end();
    reveal(cursor);
    live.noteListener = onNote;
    const stopPads = onAction((action) => {
      if (action === "record") toggleStepInput();
      else if (action === "stop") {
        cursor = 0;
        scroll = 0;
      } else if (action === "play") leave(true);
    });
    let raf = requestAnimationFrame(function frame(now) {
      draw(now);
      raf = requestAnimationFrame(frame);
    });
    return () => {
      cancelAnimationFrame(raf);
      observer.disconnect();
      stopPads();
      if (live.noteListener === onNote) live.noteListener = null;
    };
  });

  const lengths = [
    { value: "4", label: "1" },
    { value: "2", label: "1/2" },
    { value: "1", label: "1/4" },
    { value: "0.5", label: "1/8" },
    { value: "0.25", label: "1/16" },
  ];
</script>

<svelte:window onkeydown={keydown} />

<section class="editor">
  <TopBar back={nav.backLabel} onback={() => leave(false)}>
    {#snippet heading()}
      <label class="title-field">
        <span class="sr-only">Title</span>
        <input bind:value={title} oninput={changed} placeholder="Song title" />
      </label>
      <span class="status dim" class:bad={saveState === "failed"} aria-live="polite">
        {#if saveState === "saving"}Saving…{:else if saveState === "failed"}Couldn't save{:else if saveState === "unsaved"}Edited{:else if songId}Saved{/if}
      </span>
    {/snippet}
    {#snippet actions()}
      <button class="ghost icon" onclick={undo} aria-label="Undo">
        <svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true"><path d="M9 14L4 9l5-5M4 9h11a5 5 0 0 1 0 10h-3" /></svg>
      </button>
      <button class="ghost icon" onclick={redo} aria-label="Redo">
        <svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true"><path d="M15 14l5-5-5-5M20 9H9a5 5 0 0 0 0 10h3" /></svg>
      </button>
      <div class="anchor">
        <button class="ghost" aria-expanded={infoOpen} onclick={() => (infoOpen = !infoOpen)}>Song info</button>
        <Sheet label="Song info" open={infoOpen} onclose={() => (infoOpen = false)} width={300}>
          <div class="info">
            <label class="info-field">
              <span class="label">Artist</span>
              <input bind:value={artist} oninput={changed} placeholder="Artist" />
            </label>
            <label class="info-field">
              <span class="label">Tempo</span>
              <span class="bpm">
                <input type="number" min="20" max="300" bind:value={bpm} oninput={changed} />
                <span class="dim">BPM</span>
              </span>
            </label>
            <div class="info-field">
              <span class="label">Meter</span>
              <Segmented
                label="Meter"
                value={meter}
                onchange={(m) => {
                  meter = m;
                  changed();
                }}
                options={[
                  { value: "2/4", label: "2/4" },
                  { value: "3/4", label: "3/4" },
                  { value: "4/4", label: "4/4" },
                  { value: "6/8", label: "6/8" },
                ]}
              />
            </div>
          </div>
        </Sheet>
      </div>
      <button class="primary" onclick={() => leave(true)} disabled={noteCount === 0}>Practice</button>
    {/snippet}
  </TopBar>

  <div class="roll">
    <canvas
      bind:this={canvas}
      onpointerdown={down}
      onpointermove={move}
      onpointerup={up}
      onpointercancel={up}
      onpointerleave={() => (hover = null)}
      oncontextmenu={contextmenu}
      onwheel={wheel}
      aria-label="Piano roll"
    ></canvas>

    {#if noteCount === 0 && !typing}
      <div class="empty">
        <div class="empty-title">An empty song</div>
        <div class="dim">
          Click above a key to place a note, turn on <b>Step input</b> (or the record pad) and play your keyboard, or
          <b>Type notes</b> you found online.
        </div>
      </div>
    {/if}

    {#if typing}
      <form
        class="typing"
        onsubmit={(e) => {
          e.preventDefault();
          addTyped();
        }}
      >
        <label class="sr-only" for="typed">Notes to add</label>
        <textarea
          id="typed"
          bind:value={typed}
          rows="3"
          placeholder="E4 E4 F4 G4 | G4 F4 E4 D4 | C4+E4:2 -:2"
          onkeydown={(e) => {
            if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) addTyped();
          }}
        ></textarea>
        <div class="typing-row">
          <span class="dim help">
            Notes like <b>C4</b> <b>F#3</b> <b>Bb2</b> · chords <b>C4+E4</b> · length <b>:1 :2 :4 :8 :16</b> (add
            <b>.</b> for dotted) · rest <b>-</b> · <b>|</b> is ignored · no length = the selected one
          </span>
          <button class="primary small" type="submit">Add at cursor</button>
        </div>
        {#if typedError}<div class="error">{typedError}</div>{/if}
      </form>
    {/if}

    <div class="dock">
      <button class="record" class:on={stepInput} onclick={toggleStepInput} aria-pressed={stepInput}>
        <span class="dot" aria-hidden="true"></span>
        Step input
      </button>
      <Segmented label="Note length" value={stepLength} onchange={(v) => (stepLength = v)} options={lengths} />
      <button class="toggle" class:on={dotted} aria-pressed={dotted} aria-label="Dotted" title="Dotted (×1.5)" onclick={() => (dotted = !dotted)}>·</button>
      <Segmented
        label="Hand"
        value={hand}
        onchange={(v) => (hand = v)}
        options={[
          { value: "left", label: "L" },
          { value: "right", label: "R" },
        ]}
      />
      <button class="ghost" onclick={rest}>Rest</button>
      <button class="toggle" class:on={typing} aria-pressed={typing} onclick={() => (typing = !typing)}>Type</button>
      <div class="anchor">
        <button class="ghost icon" aria-label="Editing help" aria-expanded={helpOpen} onclick={() => (helpOpen = !helpOpen)}>?</button>
        <Sheet label="Editing help" open={helpOpen} onclose={() => (helpOpen = false)} width={280} align="right">
          <ul class="help-list">
            <li>Click above a key to add a note</li>
            <li>Drag a note to move it; drag its top edge to lengthen</li>
            <li>Right-click a note to delete it</li>
            <li>Alt+click moves the cursor (where new notes go)</li>
            <li>Ctrl+wheel zooms, wheel scrolls</li>
            <li>Step input: play a key or chord, it lands at the cursor</li>
            <li>Ctrl+Z / Ctrl+Y undo and redo</li>
          </ul>
        </Sheet>
      </div>
    </div>
  </div>
</section>

<style>
  svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .dim {
    color: var(--mist);
  }
  .editor {
    position: absolute;
    inset: 0;
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    background: var(--void);
  }
  .anchor {
    position: relative;
  }
  .title-field {
    flex: 1;
    min-width: 0;
    display: flex;
  }
  .title-field input {
    flex: 1;
    font-size: 18px;
    font-weight: 600;
    letter-spacing: -0.02em;
    border-color: transparent;
    background: transparent;
    padding: 6px 8px;
  }
  .title-field input:hover {
    border-color: var(--line);
  }
  .info {
    display: grid;
    gap: 14px;
  }
  .info-field {
    display: grid;
    gap: 6px;
    justify-items: start;
  }
  .info-field input {
    width: 100%;
  }
  .label {
    font-size: 12px;
    color: var(--mist);
  }
  .bpm {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .dock {
    position: absolute;
    left: 50%;
    bottom: 34px;
    transform: translateX(-50%);
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px;
    border-radius: 999px;
    background: rgba(22, 22, 29, 0.9);
    border: 1px solid var(--line-strong);
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.45);
    backdrop-filter: blur(18px);
    white-space: nowrap;
  }
  .dock .anchor :global(.sheet) {
    top: auto;
    bottom: calc(100% + 10px);
    transform-origin: bottom right;
  }
  .help-list {
    margin: 0;
    padding-left: 18px;
    display: grid;
    gap: 6px;
    font-size: 13px;
    line-height: 1.4;
    white-space: normal;
  }
  button {
    font: inherit;
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    color: var(--snow);
  }
  button:disabled {
    opacity: 0.4;
    cursor: default;
  }
  input,
  textarea {
    font: inherit;
    font-size: 13px;
    color: var(--snow);
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid var(--line);
    border-radius: 10px;
    padding: 8px 10px;
    min-width: 0;
    user-select: text;
  }
  input:focus,
  textarea:focus {
    outline: none;
    border-color: var(--pulse);
  }
  .bpm input {
    width: 64px;
  }
  .status {
    font-size: 12px;
    min-width: 7ch;
    text-align: right;
  }
  .status.bad {
    color: var(--coral);
  }
  .primary {
    padding: 9px 18px;
    border-radius: 999px;
    border: 0;
    background: var(--pulse);
    color: var(--void);
    box-shadow: 0 0 20px rgba(127, 212, 255, 0.3);
  }
  .primary.small {
    padding: 7px 14px;
  }
  .ghost,
  .toggle {
    padding: 7px 12px;
    border-radius: 999px;
    border: 1px solid var(--line);
    background: transparent;
    color: var(--mist);
  }
  .ghost:hover,
  .toggle:hover {
    color: var(--snow);
  }
  .toggle.on {
    background: var(--snow);
    color: var(--void);
    border-color: transparent;
  }
  .icon {
    width: 32px;
    height: 32px;
    padding: 0;
    display: grid;
    place-items: center;
  }
  .record {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 7px 14px 7px 10px;
    border-radius: 999px;
    border: 1px solid var(--line-strong);
    background: rgba(255, 255, 255, 0.05);
  }
  .record .dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: var(--slate);
  }
  .record.on {
    border-color: rgba(255, 122, 89, 0.6);
    background: rgba(255, 122, 89, 0.12);
  }
  .record.on .dot {
    background: var(--coral);
    box-shadow: 0 0 10px var(--coral);
    animation: blink 1s ease-in-out infinite;
  }

  .roll {
    position: relative;
    min-height: 0;
  }
  canvas {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    touch-action: none;
  }
  .empty {
    position: absolute;
    top: 30%;
    left: 50%;
    transform: translateX(-50%);
    max-width: 420px;
    text-align: center;
    font-size: 13px;
    line-height: 1.5;
    pointer-events: none;
  }
  .empty-title {
    font-size: 22px;
    font-weight: 600;
    letter-spacing: -0.02em;
    margin-bottom: 6px;
  }
  .typing {
    position: absolute;
    left: 16px;
    right: 16px;
    top: 12px;
    display: grid;
    gap: 8px;
    padding: 12px;
    border-radius: 16px;
    background: rgba(22, 22, 29, 0.94);
    border: 1px solid var(--line-strong);
    backdrop-filter: blur(16px);
  }
  .typing textarea {
    font-family: var(--mono);
    resize: vertical;
  }
  .typing-row {
    display: flex;
    gap: 12px;
    align-items: center;
    justify-content: space-between;
  }
  .help {
    font-size: 11px;
    line-height: 1.5;
  }
  .help b {
    font-family: var(--mono);
    font-weight: 500;
    color: var(--tint);
  }
  .error {
    font-size: 12px;
    color: var(--coral);
  }
  @keyframes blink {
    50% {
      opacity: 0.4;
    }
  }
</style>
