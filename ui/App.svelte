<script lang="ts">
  import { onMount } from "svelte";
  import { Spring } from "svelte/motion";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import Stage from "./lib/Stage.svelte";
  import { engine, type InstrumentInfo, type MidiDevice } from "./lib/engine";
  import { live, press, release, heldCount } from "./lib/live";
  import { COMPUTER_KEYS, noteName } from "./lib/keyboard";
  import { songs, type Song } from "./lib/songs";
  import { actionFor, fire } from "./lib/controls";
  import Editor from "./views/Editor.svelte";
  import Library from "./views/Library.svelte";
  import Practice from "./views/Practice.svelte";

  type View = "play" | "songs" | "practice" | "edit";
  let view = $state<View>("play");
  let openSong = $state<{ id: string; song: Song } | null>(null);
  /** The editor's target: an existing song or a new one; `key` re-creates the editor per open. */
  let editing = $state<{ id: string | null; song: Song | null; key: number } | null>(null);
  let libraryRefresh = $state(0);
  let dropping = $state(false);

  function openEditor(id: string | null) {
    if (!id) {
      editing = { id: null, song: null, key: Date.now() };
      view = "edit";
      return;
    }
    songs
      .get(id)
      .then((song) => {
        editing = { id, song, key: Date.now() };
        view = "edit";
      })
      .catch((e) => showError(String(e)));
  }

  function openPractice(id: string) {
    songs
      .get(id)
      .then((song) => {
        openSong = { id, song };
        view = "practice";
      })
      .catch((e) => showError(String(e)));
  }

  async function importDropped(paths: string[]) {
    const midi = paths.filter((p) => /\.midi?$/i.test(p));
    if (!midi.length) return showError("Only .mid / .midi files can be imported for now.");
    try {
      const result = await songs.importFiles(midi);
      if (result.errors.length) showError(result.errors.join("\n"));
      if (result.songs.length) {
        view = "songs";
        libraryRefresh++;
      }
    } catch (e) {
      showError(String(e));
    }
  }

  let instruments = $state<InstrumentInfo[]>([]);
  let current = $state<string | null>(null);
  let loading = $state<string | null>(null);
  let devices = $state<MidiDevice[]>([]);
  let latency = $state<number | null>(null);
  let volume = $state(0);
  let sustain = $state(false);
  let lastNote = $state<{ key: number; velocity: number } | null>(null);
  let held = $state(0);
  let activity = $state(0);
  let octave = $state(4);
  let keyboardHeight = $state(0);
  let update = $state<string | null>(null);
  let error = $state<string | null>(null);
  let fatal = $state<string | null>(null);

  const pop = new Spring(1, { stiffness: 0.18, damping: 0.32 });
  const connected = $derived(devices.filter((d) => d.connected));
  const busy = $derived(devices.filter((d) => !d.connected));

  let errorTimer: ReturnType<typeof setTimeout>;
  function showError(message: string) {
    error = message;
    clearTimeout(errorTimer);
    errorTimer = setTimeout(() => (error = null), 6000);
  }

  async function refreshInstruments() {
    instruments = await engine.instruments().catch(() => instruments);
  }

  function choose(id: string) {
    if (id === current || loading) return;
    loading = id;
    engine.load(id).catch((e) => {
      loading = null;
      showError(String(e));
    });
  }

  // Computer keyboard: A–' plays an octave and a half, Z/X shift octave, Space is the sustain pedal.
  const down = new Map<string, number>();

  function keydown(e: KeyboardEvent) {
    if (e.repeat || e.ctrlKey || e.metaKey || e.altKey) return;
    // Typing a song title must not play notes.
    if ((e.target as HTMLElement).closest?.("input:not([type=range]), textarea")) return;
    if (e.code === "Space") {
      e.preventDefault();
      (document.activeElement as HTMLElement | null)?.blur();
      engine.pedal(true);
    } else if (e.code === "KeyZ") {
      octave = Math.max(0, octave - 1);
    } else if (e.code === "KeyX") {
      octave = Math.min(7, octave + 1);
    } else if (e.code in COMPUTER_KEYS && !down.has(e.code)) {
      const key = (octave + 1) * 12 + COMPUTER_KEYS[e.code];
      down.set(e.code, key);
      engine.note(key, 96);
    }
  }

  function keyup(e: KeyboardEvent) {
    if (e.code === "Space") engine.pedal(false);
    const key = down.get(e.code);
    if (key !== undefined) {
      down.delete(e.code);
      engine.note(key, 0);
    }
  }

  function releaseAll() {
    for (const key of down.values()) engine.note(key, 0);
    down.clear();
  }

  onMount(() => {
    const unlisten = [
      engine.onNote(({ key, velocity }) => {
        if (velocity > 0) {
          press(key, velocity);
          live.practice?.press(key);
          lastNote = { key, velocity };
          pop.set(1.14, { instant: true });
          pop.target = 1;
          activity++;
        } else {
          release(key);
        }
        held = heldCount();
        live.noteListener?.(key, velocity);
      }),
      engine.onControl(({ controller, value }) => {
        const action = actionFor(controller, value);
        if (action) fire(action);
      }),
      engine.onPedal((isDown) => (live.sustain = sustain = isDown)),
      engine.onDevices((d) => (devices = d)),
      engine.onInstrument((s) => {
        if (s.state === "loading") loading = s.instrument.id;
        if (s.state === "ready") current = s.instrument.id;
        if (s.state !== "loading") loading = null;
        if (s.state === "failed") showError(`${s.instrument.name}: ${s.error}`);
      }),
      getCurrentWebview().onDragDropEvent(({ payload }) => {
        if (payload.type === "enter" || payload.type === "over") dropping = true;
        else if (payload.type === "leave") dropping = false;
        else if (payload.type === "drop") {
          dropping = false;
          importDropped(payload.paths);
        }
      }),
    ];

    engine
      .status()
      .then((s) => {
        devices = s.devices;
        current ??= s.instrument?.id ?? null;
        volume = s.volume_db;
        latency = s.latency_ms;
      })
      .catch((e) => (fatal = String(e)));
    refreshInstruments();
    engine.checkUpdate().then((v) => (update = v)).catch(() => {});
    const poll = setInterval(() => engine.status().then((s) => (latency = s.latency_ms)).catch(() => {}), 2000);

    return () => {
      unlisten.forEach((u) => u.then((off) => off()));
      clearInterval(poll);
    };
  });
</script>

<svelte:window onkeydown={keydown} onkeyup={keyup} onblur={releaseAll} onfocus={refreshInstruments} />

<div class="app">
  <header class="bar">
    <div class="left">
      <div class="brand"><span class="mark" aria-hidden="true"></span>Pulsar</div>
      <nav class="tabs" aria-label="Section">
        <button class:on={view === "play"} aria-current={view === "play"} onclick={() => (view = "play")}>Play</button>
        <button class:on={view === "songs"} aria-current={view === "songs"} onclick={() => (view = "songs")}>Songs</button>
        {#if openSong}
          <button class="song-tab" class:on={view === "practice"} aria-current={view === "practice"} onclick={() => (view = "practice")}>
            {openSong.song.title}
          </button>
        {/if}
        {#if view === "edit"}
          <button class="on" aria-current="true">Editor</button>
        {/if}
      </nav>
    </div>

    <nav class="instruments" aria-label="Instrument">
      {#each instruments as i (i.id)}
        <button
          class="pill"
          class:active={current === i.id}
          class:loading={loading === i.id}
          aria-pressed={current === i.id}
          disabled={loading !== null && loading !== i.id}
          onclick={() => choose(i.id)}
        >
          {i.name}
        </button>
      {/each}
      <button class="pill ghost" title="Drop .sf2 files into this folder" onclick={() => engine.openSoundsFolder()}>
        + Sounds
      </button>
    </nav>

    <div class="meta">
      <span class="device" title={devices.map((d) => `${d.name}${d.connected ? "" : " (in use by another app)"}`).join("\n")}>
        {#key activity}<span class="led" class:on={connected.length > 0}></span>{/key}
        {connected[0]?.name ?? (busy[0] ? `${busy[0].name} is busy` : "No MIDI keyboard")}
      </span>
      {#if latency !== null}<span class="mono dim">{latency.toFixed(1)} ms</span>{/if}
      <label class="volume">
        <span class="sr-only">Volume</span>
        <input
          type="range"
          min="-40"
          max="6"
          step="0.5"
          bind:value={volume}
          oninput={() => engine.setVolume(volume)}
          style:--fill="{((volume + 40) / 46) * 100}%"
        />
        <span class="mono dim db">{volume.toFixed(1)} dB</span>
      </label>
      {#if update}
        <button class="pill active small" onclick={() => engine.installUpdate()}>Update {update}</button>
      {/if}
    </div>
  </header>

  <main class="stage">
    <Stage bind:keyboardHeight onpress={(k, v) => engine.note(k, v)} onrelease={(k) => engine.note(k, 0)} />

    {#if view === "play"}
      <div class="readout" style:bottom="{keyboardHeight}px">
        {#if lastNote}
          <div class="big" style:transform="scale({pop.current})">{noteName(lastNote.key)}</div>
          <div class="mono line">
            {noteName(lastNote.key)} · vel {lastNote.velocity}{held > 1 ? ` · ${held} keys` : ""}
          </div>
        {:else}
          <div class="hint">
            <div class="hint-title">Play anything.</div>
            <div class="dim">Your MIDI keyboard, computer keys A – ' , or click the keys below.</div>
          </div>
        {/if}
      </div>

      <div class="chips" style:bottom="{keyboardHeight + 16}px">
        <span class="chip" class:on={sustain}>Sustain</span>
        <span class="chip mono">Keys {noteName((octave + 1) * 12)} · Z/X</span>
      </div>
    {/if}

    {#if view === "songs"}
      <div class="panel" style:bottom="{keyboardHeight}px">
        <Library refresh={libraryRefresh} onopen={openPractice} onedit={openEditor} onerror={showError} />
      </div>
    {/if}

    {#if openSong && view === "practice"}
      {#key openSong.id}
        <Practice
          song={openSong.song}
          {keyboardHeight}
          onback={() => (view = "songs")}
          onedit={() => openSong && openEditor(openSong.id)}
        />
      {/key}
    {/if}

    {#if editing && view === "edit"}
      {#key editing.key}
        <div class="panel" style:bottom="{keyboardHeight}px">
          <Editor
            id={editing.id}
            song={editing.song}
            onback={() => (view = "songs")}
            onpractice={openPractice}
            onsaved={() => libraryRefresh++}
          />
        </div>
      {/key}
    {/if}

    {#if dropping}
      <div class="drop" style:bottom="{keyboardHeight}px">Drop MIDI files to add them to your songs</div>
    {/if}

    {#if error || fatal}
      <div class="toast" role="alert">{fatal ?? error}</div>
    {/if}
  </main>
</div>

<style>
  .app {
    display: grid;
    grid-template-rows: auto 1fr;
    height: 100%;
  }

  .bar {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    align-items: center;
    gap: 24px;
    height: 60px;
    padding: 0 20px;
    background: rgba(22, 22, 29, 0.72);
    backdrop-filter: blur(24px) saturate(140%);
    border-bottom: 1px solid var(--line);
    position: relative;
    z-index: 2;
  }

  .left {
    display: flex;
    align-items: center;
    gap: 20px;
    min-width: 0;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 17px;
    font-weight: 600;
    letter-spacing: -0.02em;
  }
  .tabs {
    display: flex;
    gap: 2px;
    min-width: 0;
  }
  .tabs button {
    font-size: 13px;
    font-weight: 500;
    padding: 7px 12px;
    border: 0;
    border-radius: 8px;
    background: transparent;
    color: var(--mist);
    cursor: pointer;
  }
  .tabs button:hover {
    color: var(--snow);
  }
  .tabs button.on {
    color: var(--snow);
    background: rgba(255, 255, 255, 0.08);
  }
  .song-tab {
    max-width: 180px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .panel {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
  }
  .drop {
    position: absolute;
    top: 16px;
    left: 16px;
    right: 16px;
    display: grid;
    place-items: center;
    border: 2px dashed rgba(127, 212, 255, 0.6);
    border-radius: 24px;
    background: rgba(10, 10, 15, 0.8);
    color: var(--tint);
    font-size: 18px;
    font-weight: 500;
    pointer-events: none;
    z-index: 3;
  }
  .mark {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: var(--tint);
    box-shadow: 0 0 6px var(--pulse), 0 0 18px var(--pulse);
  }

  .instruments {
    display: flex;
    gap: 8px;
    /* "safe" keeps the first pill reachable when the row overflows and scrolls. */
    justify-content: safe center;
    overflow-x: auto;
    scrollbar-width: none;
    padding: 8px 4px;
  }

  .pill {
    flex-shrink: 0;
    font-size: 13px;
    font-weight: 500;
    padding: 9px 18px;
    border-radius: 999px;
    border: 1px solid var(--line-strong);
    background: rgba(255, 255, 255, 0.06);
    color: var(--snow);
    cursor: pointer;
    transition:
      background 180ms ease,
      box-shadow 240ms ease,
      transform 180ms cubic-bezier(0.3, 1.6, 0.5, 1);
  }
  .pill:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.1);
  }
  .pill:active:not(:disabled) {
    transform: scale(0.96);
  }
  .pill:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .pill.active {
    background: var(--pulse);
    border-color: transparent;
    color: var(--void);
    box-shadow: 0 0 24px rgba(127, 212, 255, 0.35);
  }
  .pill.loading {
    border-color: rgba(127, 212, 255, 0.5);
    background: linear-gradient(90deg, rgba(127, 212, 255, 0.06), rgba(127, 212, 255, 0.32), rgba(127, 212, 255, 0.06));
    background-size: 200% 100%;
    animation: shimmer 1.1s linear infinite;
  }
  .pill.ghost {
    background: transparent;
    border-style: dashed;
    color: var(--mist);
  }
  .pill.small {
    padding: 6px 12px;
    font-size: 12px;
  }

  .meta {
    display: flex;
    align-items: center;
    gap: 18px;
    font-size: 13px;
  }
  .device {
    display: flex;
    align-items: center;
    gap: 8px;
    max-width: 220px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .led {
    flex-shrink: 0;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--slate);
  }
  .led.on {
    background: var(--pulse);
    box-shadow: 0 0 8px var(--pulse);
    animation: flash 420ms ease-out;
  }
  .dim {
    color: var(--mist);
  }

  .volume {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .db {
    width: 6.5ch;
    text-align: right;
  }
  input[type="range"] {
    appearance: none;
    width: 120px;
    height: 18px;
    margin: 0;
    background: transparent;
    cursor: pointer;
  }
  input[type="range"]::-webkit-slider-runnable-track {
    height: 4px;
    border-radius: 2px;
    background: linear-gradient(to right, var(--pulse) var(--fill), var(--slate) var(--fill));
  }
  input[type="range"]::-webkit-slider-thumb {
    appearance: none;
    width: 16px;
    height: 16px;
    margin-top: -6px;
    border-radius: 50%;
    background: var(--snow);
    box-shadow: 0 1px 6px rgba(0, 0, 0, 0.5);
  }

  .stage {
    position: relative;
    overflow: hidden;
    background: radial-gradient(120% 80% at 50% 110%, #12121a 0%, var(--void) 60%);
  }

  .readout {
    position: absolute;
    inset: 0;
    display: grid;
    place-content: center;
    justify-items: center;
    gap: 6px;
    pointer-events: none;
  }
  .big {
    font-size: clamp(96px, 15vw, 200px);
    font-weight: 600;
    letter-spacing: -0.05em;
    line-height: 1;
    background: linear-gradient(180deg, rgba(242, 242, 247, 0.2), rgba(127, 212, 255, 0.04));
    background-clip: text;
    color: transparent;
    will-change: transform;
  }
  .line {
    font-size: 13px;
    color: var(--tint);
  }
  .hint {
    text-align: center;
    font-size: 14px;
  }
  .hint-title {
    font-size: 28px;
    font-weight: 600;
    letter-spacing: -0.03em;
    margin-bottom: 8px;
  }

  .chips {
    position: absolute;
    left: 20px;
    display: flex;
    gap: 8px;
    pointer-events: none;
  }
  .chip {
    font-size: 12px;
    padding: 5px 11px;
    border-radius: 999px;
    border: 1px solid var(--line);
    background: rgba(22, 22, 29, 0.6);
    color: var(--mist);
    transition: all 160ms ease;
  }
  .chip.on {
    color: var(--void);
    background: var(--pulse);
    border-color: transparent;
    box-shadow: 0 0 18px rgba(127, 212, 255, 0.45);
  }

  .toast {
    position: absolute;
    top: 16px;
    left: 50%;
    transform: translateX(-50%);
    max-width: min(640px, 90%);
    padding: 10px 16px;
    border-radius: 12px;
    background: rgba(22, 22, 29, 0.92);
    border: 1px solid rgba(255, 122, 89, 0.4);
    color: var(--coral);
    font-size: 13px;
    white-space: pre-line;
    z-index: 4;
  }

  @keyframes shimmer {
    from {
      background-position: 100% 0;
    }
    to {
      background-position: -100% 0;
    }
  }
  @keyframes flash {
    from {
      background: #fff;
      box-shadow: 0 0 16px 3px var(--tint);
    }
  }
</style>
