<script lang="ts">
  // Free play: nothing but the keys, the light, and the note you just played. Sound is one tap away.
  import { onMount } from "svelte";
  import { Spring } from "svelte/motion";
  import Sheet from "../lib/Sheet.svelte";
  import SoundPanel from "../lib/SoundPanel.svelte";
  import TopBar from "../lib/TopBar.svelte";
  import { computerKeys, nav, played, sound } from "../lib/app.svelte";
  import { noteName } from "../lib/keyboard";

  let soundOpen = $state(false);
  const pop = new Spring(1, { stiffness: 0.18, damping: 0.32 });

  $effect(() => {
    if (played.activity) {
      pop.set(1.14, { instant: true });
      pop.target = 1;
    }
  });

  const instrument = $derived(sound.instruments.find((i) => i.id === sound.current)?.name ?? "Sound");

  // The computer-keys hint shows only for a few seconds after those keys are used.
  let now = $state(Date.now());
  onMount(() => {
    const timer = setInterval(() => (now = Date.now()), 500);
    return () => clearInterval(timer);
  });
  const showKeys = $derived(now - computerKeys.usedAt < 3000);
</script>

<TopBar back={nav.backLabel} onback={() => nav.back()}>
  {#snippet actions()}
    <div class="anchor">
      <button class="sound" aria-expanded={soundOpen} onclick={() => (soundOpen = !soundOpen)}>
        {instrument}
        <svg viewBox="0 0 24 24" width="14" height="14" aria-hidden="true"><path d="M6 9l6 6 6-6" /></svg>
      </button>
      <Sheet label="Sound" open={soundOpen} onclose={() => (soundOpen = false)} width={300}>
        <SoundPanel />
      </Sheet>
    </div>
  {/snippet}
</TopBar>

<div class="readout">
  {#if played.lastNote}
    <div class="big" style:transform="scale({pop.current})">{noteName(played.lastNote.key)}</div>
    <div class="mono line">
      vel {played.lastNote.velocity}{played.held > 1 ? ` · ${played.held} keys` : ""}
    </div>
  {:else}
    <div class="hint">
      <div class="hint-title">Play anything.</div>
      <div class="dim">Your MIDI keyboard, the computer keys A – ' , or click the keys below.</div>
    </div>
  {/if}
</div>

{#if showKeys}
  <div class="keys mono">Computer keys: {noteName((computerKeys.octave + 1) * 12)} up · Z / X shifts the octave</div>
{/if}

<style>
  .anchor {
    position: relative;
  }
  .sound {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 12px 8px 16px;
    border-radius: 999px;
    border: 1px solid var(--line-strong);
    background: rgba(22, 22, 29, 0.6);
    color: var(--snow);
    font: inherit;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
  }
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
  .keys {
    position: absolute;
    left: 50%;
    bottom: 16px;
    transform: translateX(-50%);
    font-size: 12px;
    padding: 6px 12px;
    border-radius: 999px;
    background: rgba(22, 22, 29, 0.7);
    border: 1px solid var(--line);
    color: var(--mist);
    pointer-events: none;
  }
</style>
