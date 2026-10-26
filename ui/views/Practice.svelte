<script lang="ts">
  // Practice controls over the stage. The stage draws the song; this is the transport and settings.
  import { onDestroy, onMount } from "svelte";
  import Segmented from "../lib/Segmented.svelte";
  import { noteName } from "../lib/keyboard";
  import { live, type SongView } from "../lib/live";
  import { Practice, type Hands, type Mode } from "../lib/practice";
  import { formatTime, type Song } from "../lib/songs";

  let { song, onback }: { song: Song; onback: () => void } = $props();

  // The app re-creates this view per song, so the song is fixed for its lifetime.
  // svelte-ignore state_referenced_locally
  const practice = new Practice(song);
  live.practice = practice;
  onDestroy(() => {
    live.practice = null;
  });

  // Mirrors of the practice state, refreshed every frame for the controls.
  let time = $state(practice.time);
  let playing = $state(false);
  let finished = $state(false);
  let hits = $state(0);
  let wrong = $state(0);
  let total = $state(practice.total);
  let next = $state("");
  let waiting = $state(false);
  let speed = $state(1);
  let mode = $state<Mode>("wait");
  let hands = $state<Hands>("both");
  let view = $state<SongView>(live.songView);

  /** Names of the next chord to play, e.g. "C4 E4 G4". */
  function upcoming(): string {
    const chord =
      practice.waiting ??
      practice.chords.find(
        (c) => c.start >= practice.time - 0.05 && c.notes.some((n) => practice.isActive(n) && n.state === "pending"),
      );
    if (!chord) return "";
    return chord.notes
      .filter((n) => practice.isActive(n))
      .map((n) => noteName(n.key))
      .join("  ");
  }

  onMount(() => {
    let raf = requestAnimationFrame(function tick() {
      time = practice.time;
      playing = practice.playing;
      finished = practice.finished;
      hits = practice.hits;
      wrong = practice.wrong;
      next = upcoming();
      waiting = practice.waiting !== null;
      raf = requestAnimationFrame(tick);
    });
    return () => cancelAnimationFrame(raf);
  });

  function toggle() {
    if (practice.finished) practice.restart();
    practice.playing = !practice.playing;
  }

  function setMode(m: Mode) {
    mode = m;
    practice.setMode(m);
  }

  function setHands(h: Hands) {
    hands = h;
    practice.setHands(h);
    total = practice.total;
  }

  function setView(v: SongView) {
    view = v;
    live.songView = v;
  }

  const accuracy = $derived(total ? Math.round((hits / total) * 100) : 0);
</script>

<div class="top">
  <button class="back" onclick={onback} aria-label="Back to songs">
    <svg viewBox="0 0 24 24" width="18" height="18" aria-hidden="true"><path d="M15 6l-6 6 6 6" /></svg>
  </button>
  <div class="heading">
    <div class="title">{song.title}</div>
    {#if song.artist}<div class="dim">{song.artist}</div>{/if}
  </div>
  <div class="controls">
    <Segmented
      label="Practice mode"
      value={mode}
      onchange={setMode}
      options={[
        { value: "wait", label: "Wait for me" },
        { value: "tempo", label: "Tempo" },
      ]}
    />
    <Segmented
      label="Hands"
      value={hands}
      onchange={setHands}
      disabled={!practice.hasHands}
      options={[
        { value: "left", label: "Left" },
        { value: "both", label: "Both" },
        { value: "right", label: "Right" },
      ]}
    />
    <Segmented
      label="View"
      value={view}
      onchange={setView}
      options={[
        { value: "falling", label: "Falling" },
        { value: "tape", label: "Notes" },
      ]}
    />
  </div>
</div>

{#if next && !finished}
  <div class="next" aria-live="polite">
    <span class="dim">{waiting ? "Play" : "Next"}</span>
    <span class="mono">{next}</span>
  </div>
{/if}

{#if finished}
  <div class="done">
    <div class="score">{accuracy}%</div>
    <div class="dim">{hits} of {total} notes{wrong ? ` · ${wrong} wrong` : ""}</div>
    <div class="row">
      <button class="primary" onclick={() => practice.restart()}>Again</button>
      <button class="ghost" onclick={onback}>Songs</button>
    </div>
  </div>
{/if}

<div class="transport">
  <button class="play" onclick={toggle} aria-label={playing ? "Pause song" : "Play song"}>
    {#if playing}
      <svg viewBox="0 0 24 24" width="18" height="18" aria-hidden="true"><path d="M8 5v14M16 5v14" /></svg>
    {:else}
      <svg viewBox="0 0 24 24" width="18" height="18" aria-hidden="true"><path class="fill" d="M8 5l11 7-11 7z" /></svg>
    {/if}
  </button>
  <button class="icon" onclick={() => practice.restart()} aria-label="Restart">
    <svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true"><path d="M4 12a8 8 0 1 0 2.3-5.7M4 4v4h4" /></svg>
  </button>
  <label class="scrub">
    <span class="sr-only">Position</span>
    <input
      type="range"
      min={practice.startTime}
      max={practice.duration}
      step="0.01"
      value={time}
      oninput={(e) => practice.seek(Number(e.currentTarget.value))}
      style:--fill="{((time - practice.startTime) / (practice.duration - practice.startTime || 1)) * 100}%"
    />
  </label>
  <span class="mono dim">{formatTime(time)} / {formatTime(practice.duration)}</span>
  <label class="speed">
    <span class="dim">Speed</span>
    <input
      type="range"
      min="0.25"
      max="1.5"
      step="0.05"
      bind:value={speed}
      oninput={() => (practice.speed = speed)}
      style:--fill="{((speed - 0.25) / 1.25) * 100}%"
    />
    <span class="mono">{Math.round(speed * 100)}%</span>
  </label>
  <span class="mono stats" title="Notes hit / notes in this hand selection">
    {hits}/{total}{#if wrong}<span class="wrong">{` · ${wrong} wrong`}</span>{/if}
  </span>
</div>

<style>
  svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  svg .fill {
    fill: currentColor;
  }
  .dim {
    color: var(--mist);
  }
  button {
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    color: var(--snow);
  }

  .top {
    position: absolute;
    top: 14px;
    left: 16px;
    right: 16px;
    display: flex;
    align-items: center;
    gap: 14px;
    flex-wrap: wrap;
  }
  .back {
    width: 38px;
    height: 38px;
    display: grid;
    place-items: center;
    border-radius: 50%;
    border: 1px solid var(--line-strong);
    background: rgba(22, 22, 29, 0.7);
  }
  .heading {
    min-width: 0;
    flex: 1;
  }
  .title {
    font-size: 17px;
    font-weight: 600;
    letter-spacing: -0.02em;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .heading .dim {
    font-size: 12px;
  }
  .controls {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .next {
    position: absolute;
    top: 124px;
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    gap: 10px;
    align-items: baseline;
    padding: 6px 14px;
    border-radius: 999px;
    background: rgba(22, 22, 29, 0.75);
    border: 1px solid var(--line);
    font-size: 13px;
    pointer-events: none;
  }
  .next .mono {
    font-size: 15px;
    color: var(--tint);
  }

  .done {
    position: absolute;
    top: 38%;
    left: 50%;
    transform: translate(-50%, -50%);
    display: grid;
    justify-items: center;
    gap: 8px;
    padding: 28px 40px;
    border-radius: 24px;
    background: rgba(22, 22, 29, 0.9);
    border: 1px solid var(--line-strong);
    backdrop-filter: blur(20px);
  }
  .score {
    font-size: 64px;
    font-weight: 600;
    letter-spacing: -0.04em;
    line-height: 1;
  }
  .row {
    display: flex;
    gap: 8px;
    margin-top: 8px;
  }
  .primary,
  .ghost {
    padding: 9px 18px;
    border-radius: 999px;
    border: 0;
  }
  .primary {
    background: var(--pulse);
    color: var(--void);
  }
  .ghost {
    background: transparent;
    color: var(--mist);
  }

  /* At the top, so the space above the keyboard stays clear for arriving notes. */
  .transport {
    position: absolute;
    top: 64px;
    left: 16px;
    right: 16px;
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 8px 16px 8px 8px;
    border-radius: 999px;
    background: rgba(22, 22, 29, 0.78);
    border: 1px solid var(--line);
    backdrop-filter: blur(20px);
    font-size: 12px;
  }
  .play {
    width: 42px;
    height: 42px;
    flex-shrink: 0;
    display: grid;
    place-items: center;
    border-radius: 50%;
    border: 0;
    background: var(--pulse);
    color: var(--void);
    box-shadow: 0 0 22px rgba(127, 212, 255, 0.35);
  }
  .icon {
    width: 34px;
    height: 34px;
    flex-shrink: 0;
    display: grid;
    place-items: center;
    border-radius: 50%;
    border: 0;
    background: transparent;
    color: var(--mist);
  }
  .icon:hover {
    color: var(--snow);
  }
  .scrub {
    flex: 1;
    display: flex;
  }
  .scrub input {
    width: 100%;
  }
  .speed {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .speed input {
    width: 90px;
  }
  .speed .mono {
    width: 4ch;
    text-align: right;
  }
  .stats {
    white-space: nowrap;
  }
  .wrong {
    color: var(--coral);
  }

  input[type="range"] {
    appearance: none;
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
    width: 14px;
    height: 14px;
    margin-top: -5px;
    border-radius: 50%;
    background: var(--snow);
  }
</style>
