<script lang="ts">
  // The shell: the instrument stage is always there, and one screen at a time sits on top of it.
  // Screens stack: you go into things and come back out (‹ or Escape).
  import { onMount } from "svelte";
  import { cubicOut } from "svelte/easing";
  import { fade, fly } from "svelte/transition";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import Stage from "./lib/Stage.svelte";
  import { engine } from "./lib/engine";
  import { live, press, release, heldCount } from "./lib/live";
  import { COMPUTER_KEYS } from "./lib/keyboard";
  import { songs } from "./lib/songs";
  import { MOD_WHEEL, actionFor, faderFor, fire, moveFader, moveStrip } from "./lib/controls";
  import {
    computerKeys,
    library,
    nav,
    played,
    refreshInstruments,
    showError,
    sound,
    status,
    toast,
  } from "./lib/app.svelte";
  import Editor from "./views/Editor.svelte";
  import FreePlay from "./views/FreePlay.svelte";
  import Home from "./views/Home.svelte";
  import Library from "./views/Library.svelte";
  import Practice from "./views/Practice.svelte";
  import Settings from "./views/Settings.svelte";

  let keyboardHeight = $state(0);
  let dropping = $state(false);

  async function importDropped(paths: string[]) {
    const midi = paths.filter((p) => /\.midi?$/i.test(p));
    if (!midi.length) return showError("Only .mid / .midi files can be imported for now.");
    try {
      const result = await songs.importFiles(midi);
      if (result.errors.length) showError(result.errors.join("\n"));
      if (result.songs.length) {
        library.version++;
        if (nav.route.name !== "songs") nav.push({ name: "songs" });
      }
    } catch (e) {
      showError(String(e));
    }
  }

  // Computer keyboard: A–' plays an octave and a half, Z/X shift octave, Space is the sustain pedal.
  // Escape goes back (the editor uses Escape itself).
  const down = new Map<string, number>();

  function keydown(e: KeyboardEvent) {
    if (e.repeat || e.ctrlKey || e.metaKey || e.altKey) return;
    if ((e.target as HTMLElement).closest?.("input:not([type=range]), textarea")) return;
    if (e.key === "Escape") {
      if (nav.route.name !== "edit") nav.back();
    } else if (e.code === "Space") {
      e.preventDefault();
      (document.activeElement as HTMLElement | null)?.blur();
      // While practicing, Space is play/pause (like the ▶ pad); everywhere else it's the sustain pedal.
      if (nav.route.name === "practice") fire("play");
      else engine.pedal(true);
    } else if (e.code === "KeyZ" || e.code === "KeyX") {
      computerKeys.octave = Math.min(7, Math.max(0, computerKeys.octave + (e.code === "KeyX" ? 1 : -1)));
      computerKeys.usedAt = Date.now();
    } else if (e.code in COMPUTER_KEYS && !down.has(e.code)) {
      const key = (computerKeys.octave + 1) * 12 + COMPUTER_KEYS[e.code];
      down.set(e.code, key);
      computerKeys.usedAt = Date.now();
      engine.note(key, 96);
    }
  }

  function keyup(e: KeyboardEvent) {
    if (e.code === "Space" && nav.route.name !== "practice") engine.pedal(false);
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
      engine.onNote(({ key, velocity, device }) => {
        if (velocity > 0) {
          press(key, velocity);
          // Notes Pulsar plays itself come back here too; they aren't yours.
          if (device || !live.practice?.isAutoKey(key)) live.practice?.press(key, velocity);
          played.lastNote = { key, velocity };
          played.activity++;
        } else {
          release(key);
          live.practice?.release(key);
        }
        played.held = heldCount();
        live.noteListener?.(key, velocity);
      }),
      engine.onControl(({ controller, value, port }) => {
        if (port === "control") {
          const action = actionFor(controller, value);
          if (action) fire(action);
          return;
        }
        const fader = faderFor(controller);
        if (fader) moveFader(fader, value / 127);
        else if (controller === MOD_WHEEL) moveStrip("right", value / 127);
      }),
      engine.onBend((amount) => moveStrip("left", amount)),
      engine.onPedal((isDown) => (live.sustain = played.sustain = isDown)),
      engine.onDevices((devices) => (status.devices = devices)),
      engine.onInstrument((s) => {
        if (s.state === "loading") sound.loading = s.instrument.id;
        if (s.state === "ready") sound.current = s.instrument.id;
        if (s.state !== "loading") sound.loading = null;
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
        status.devices = s.devices;
        status.output = s.output_device;
        status.sampleRate = s.sample_rate;
        status.latency = s.latency_ms;
        sound.current ??= s.instrument?.id ?? null;
        sound.volume = s.volume_db;
        sound.touch = s.touch;
      })
      .catch((e) => (status.fatal = String(e)));
    refreshInstruments();
    engine
      .checkUpdate()
      .then((v) => (status.update = v))
      .catch(() => {});
    const poll = setInterval(
      () =>
        engine
          .status()
          .then((s) => (status.latency = s.latency_ms))
          .catch(() => {}),
      2000,
    );

    return () => {
      unlisten.forEach((u) => u.then((off) => off()));
      clearInterval(poll);
    };
  });
</script>

<svelte:window onkeydown={keydown} onkeyup={keyup} onblur={releaseAll} onfocus={refreshInstruments} />

<main class="stage">
  <Stage bind:keyboardHeight onpress={(k, v) => engine.note(k, v)} onrelease={(k) => engine.note(k, 0)} />

  {#key nav.stack}
    {@const route = nav.route}
    <div
      class="view"
      style:bottom="{keyboardHeight}px"
      in:fly={{ x: 28 * nav.dir, duration: 260, easing: cubicOut }}
      out:fade={{ duration: 110 }}
    >
      {#if route.name === "home"}
        <Home />
      {:else if route.name === "play"}
        <FreePlay />
      {:else if route.name === "songs"}
        <Library />
      {:else if route.name === "settings"}
        <Settings />
      {:else if route.name === "practice"}
        <Practice song={route.song} songId={route.id} />
      {:else if route.name === "edit"}
        <Editor id={route.id} song={route.song} />
      {/if}
    </div>
  {/key}

  {#if dropping}
    <div class="drop" style:bottom="{keyboardHeight + 16}px">Drop MIDI files to add them to your songs</div>
  {/if}

  {#if toast.message || status.fatal}
    <div class="toast" role="alert">{status.fatal ?? toast.message}</div>
  {/if}
</main>

<style>
  .stage {
    position: relative;
    height: 100%;
    overflow: hidden;
    background: radial-gradient(120% 80% at 50% 110%, #12121a 0%, var(--void) 60%);
  }
  .view {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    pointer-events: none;
  }
  /* Screens take clicks again, but with zero specificity so a screen's own `pointer-events: none`
     (like the free-play readout covering everything) always wins. */
  :global(:where(.view > *)) {
    pointer-events: auto;
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
    z-index: 30;
  }
  .toast {
    position: absolute;
    top: 16px;
    left: 50%;
    transform: translateX(-50%);
    max-width: min(640px, 90%);
    padding: 10px 16px;
    border-radius: 12px;
    background: rgba(22, 22, 29, 0.95);
    border: 1px solid rgba(255, 122, 89, 0.4);
    color: var(--coral);
    font-size: 13px;
    white-space: pre-line;
    z-index: 40;
  }
</style>
