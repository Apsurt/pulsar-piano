<script lang="ts">
  // Practicing a song. The stage draws the notes and the progress; this screen keeps just the song title,
  // a play prompt and the score in view. Mode, hands, view and speed live in the Options sheet.
  import { onDestroy, onMount } from "svelte";
  import Segmented from "../lib/Segmented.svelte";
  import Sheet from "../lib/Sheet.svelte";
  import TopBar from "../lib/TopBar.svelte";
  import { nav, openEditor } from "../lib/app.svelte";
  import { onAction } from "../lib/controls";
  import { noteName } from "../lib/keyboard";
  import { live, type SongView } from "../lib/live";
  import { Practice, type Hands, type Mode } from "../lib/practice";
  import type { Song } from "../lib/songs";

  let { song, songId }: { song: Song; songId: string } = $props();

  // A new screen is created per song, so the song is fixed for its lifetime.
  // svelte-ignore state_referenced_locally
  const practice = new Practice(song);
  live.practice = practice;
  onDestroy(() => {
    // The next screen may already have its own practice running.
    if (live.practice === practice) live.practice = null;
  });

  // Mirrors of the practice state, refreshed every frame for the controls.
  let time = $state(practice.time);
  let playing = $state(false);
  let finished = $state(false);
  let hits = $state(0);
  let wrong = $state(0);
  let total = $state(practice.total);
  let next = $state("");
  let speed = $state(1);
  let mode = $state<Mode>("wait");
  let hands = $state<Hands>("both");
  let view = $state<SongView>(live.songView);
  let optionsOpen = $state(false);

  /** Names of the first chord still to play, e.g. "C4 E4". */
  function upcoming(): string {
    const chord = practice.chords.find(
      (c) => c.start >= practice.time - 0.05 && c.notes.some((n) => practice.isActive(n) && n.state === "pending"),
    );
    return chord
      ? chord.notes
          .filter((n) => practice.isActive(n))
          .map((n) => noteName(n.key))
          .join(" ")
      : "";
  }

  onMount(() => {
    const stopPads = onAction((action) => {
      if (action === "play") toggle();
      else if (action === "stop") practice.restart();
    });
    let raf = requestAnimationFrame(function tick() {
      time = practice.time;
      playing = practice.playing;
      finished = practice.finished;
      hits = practice.hits;
      wrong = practice.wrong;
      if (!playing) next = upcoming();
      raf = requestAnimationFrame(tick);
    });
    return () => {
      stopPads();
      cancelAnimationFrame(raf);
    };
  });

  function toggle() {
    if (practice.finished) practice.restart();
    practice.playing = !practice.playing;
  }

  function nudgeSpeed(delta: number) {
    speed = Math.min(1.5, Math.max(0.25, Math.round((speed + delta) * 20) / 20));
    practice.speed = speed;
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

  // Seeking: drag along the keyboard's top edge, where the progress light runs.
  let seekBar: HTMLDivElement;
  function seekTo(e: PointerEvent) {
    const box = seekBar.getBoundingClientRect();
    const f = Math.min(1, Math.max(0, (e.clientX - box.left) / box.width));
    practice.seek(practice.startTime + f * (practice.duration - practice.startTime));
  }

  function keydown(e: KeyboardEvent) {
    if ((e.target as HTMLElement).closest?.("input, textarea")) return;
    if (e.key === "Enter") toggle();
    else if (e.key === "Home") practice.restart();
  }

  const accuracy = $derived(total ? Math.round((hits / total) * 100) : 0);
  const progress = $derived(
    Math.round(((time - practice.startTime) / (practice.duration - practice.startTime || 1)) * 100),
  );
</script>

<svelte:window onkeydown={keydown} />

<div class="top">
  <TopBar back={nav.backLabel} onback={() => nav.back()} title={song.title} subtitle={song.artist || undefined}>
    {#snippet actions()}
      <div class="anchor">
        <button class="options" aria-expanded={optionsOpen} onclick={() => (optionsOpen = !optionsOpen)}>
          <svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true">
            <path d="M4 7h10M18 7h2M4 17h4M12 17h8" /><circle cx="16" cy="7" r="2" /><circle cx="10" cy="17" r="2" />
          </svg>
          Options
        </button>
        <Sheet label="Practice options" open={optionsOpen} onclose={() => (optionsOpen = false)} width={300}>
          <div class="sheet">
            <div class="field">
              <span class="label">Mode</span>
              <Segmented
                label="Practice mode"
                value={mode}
                onchange={setMode}
                options={[
                  { value: "wait", label: "Wait for me" },
                  { value: "tempo", label: "Keep tempo" },
                ]}
              />
            </div>
            {#if practice.hasHands}
              <div class="field">
                <span class="label">Hands</span>
                <Segmented
                  label="Hands"
                  value={hands}
                  onchange={setHands}
                  options={[
                    { value: "left", label: "Left" },
                    { value: "both", label: "Both" },
                    { value: "right", label: "Right" },
                  ]}
                />
              </div>
            {/if}
            <div class="field">
              <span class="label">Show</span>
              <Segmented
                label="View"
                value={view}
                onchange={setView}
                options={[
                  { value: "falling", label: "Falling notes" },
                  { value: "tape", label: "Note names" },
                ]}
              />
            </div>
            <div class="field">
              <span class="label">Speed</span>
              <div class="stepper" role="group" aria-label="Speed">
                <button onclick={() => nudgeSpeed(-0.05)} aria-label="Slower">−</button>
                <span class="mono">{Math.round(speed * 100)}%</span>
                <button onclick={() => nudgeSpeed(0.05)} aria-label="Faster">+</button>
              </div>
            </div>
            <button class="edit" onclick={() => openEditor(songId)}>Edit notes</button>
          </div>
        </Sheet>
      </div>
    {/snippet}
  </TopBar>
</div>

{#if !playing && !finished}
  <div class="center">
    <button class="big-play" onclick={toggle} aria-label="Play song">
      <svg viewBox="0 0 24 24" width="30" height="30" aria-hidden="true"><path class="fill" d="M8 5l11 7-11 7z" /></svg>
    </button>
    <div class="hint">
      {#if mode === "wait" && next}
        Play <span class="mono notes">{next}</span> to start
      {:else}
        Press play
      {/if}
      <span class="dim">· or the ▶ pad</span>
    </div>
    {#if time > practice.startTime + 0.01}
      <button class="link" onclick={() => practice.restart()}>Start over</button>
    {/if}
  </div>
{/if}

{#if finished}
  <div class="center">
    <div class="done">
      <div class="score">{accuracy}%</div>
      <div class="dim">{hits} of {total} notes{wrong ? ` · ${wrong} wrong` : ""}</div>
      <div class="row">
        <button class="primary" onclick={() => practice.restart()}>Again</button>
        <button class="link" onclick={() => nav.back()}>Back</button>
      </div>
    </div>
  </div>
{/if}

<div class="corner">
  {#if playing}
    <button class="round" onclick={toggle} aria-label="Pause song">
      <svg viewBox="0 0 24 24" width="14" height="14" aria-hidden="true"><path d="M8 5v14M16 5v14" /></svg>
    </button>
  {/if}
  <span class="chip mono" title="Notes hit / notes to play">
    {hits}/{total}{#if wrong}<span class="wrong">{` · ${wrong}`}</span>{/if}
  </span>
  {#if speed !== 1}<span class="chip mono">{Math.round(speed * 100)}%</span>{/if}
</div>

<div
  class="seek"
  bind:this={seekBar}
  role="slider"
  tabindex="0"
  aria-label="Song position"
  aria-valuemin={0}
  aria-valuemax={100}
  aria-valuenow={progress}
  onpointerdown={(e) => {
    seekBar.setPointerCapture(e.pointerId);
    seekTo(e);
  }}
  onpointermove={(e) => {
    if (e.buttons) seekTo(e);
  }}
  onkeydown={(e) => {
    if (e.key === "ArrowRight") practice.seek(practice.time + 2);
    if (e.key === "ArrowLeft") practice.seek(practice.time - 2);
  }}
></div>

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
    stroke: none;
  }
  .dim {
    color: var(--mist);
  }
  button {
    font: inherit;
    cursor: pointer;
    color: var(--snow);
  }
  .top {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    z-index: 2;
  }
  .anchor {
    position: relative;
  }
  .options {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 14px;
    border-radius: 999px;
    border: 1px solid var(--line-strong);
    background: rgba(22, 22, 29, 0.6);
    font-size: 13px;
    font-weight: 500;
  }
  .sheet {
    display: grid;
    gap: 14px;
  }
  .field {
    display: grid;
    gap: 6px;
    justify-items: start;
  }
  .label {
    font-size: 12px;
    color: var(--mist);
  }
  .stepper {
    display: flex;
    align-items: center;
    border-radius: 999px;
    border: 1px solid var(--line);
    background: rgba(255, 255, 255, 0.04);
    font-size: 13px;
  }
  .stepper button {
    width: 34px;
    height: 30px;
    border: 0;
    background: transparent;
    color: var(--mist);
    font-size: 16px;
  }
  .stepper button:hover {
    color: var(--snow);
  }
  .stepper .mono {
    min-width: 5ch;
    text-align: center;
  }
  .edit {
    justify-self: start;
    padding: 8px 14px;
    border-radius: 999px;
    border: 1px solid var(--line);
    background: transparent;
    font-size: 13px;
    font-weight: 500;
  }
  .edit:hover {
    border-color: var(--line-strong);
  }

  .center {
    position: absolute;
    inset: 0;
    display: grid;
    place-content: center;
    justify-items: center;
    gap: 14px;
    pointer-events: none;
  }
  .center > * {
    pointer-events: auto;
  }
  .big-play {
    width: 84px;
    height: 84px;
    display: grid;
    place-items: center;
    padding-left: 4px;
    border-radius: 50%;
    border: 0;
    background: var(--pulse);
    color: var(--void);
    box-shadow:
      0 0 0 10px rgba(127, 212, 255, 0.08),
      0 0 48px rgba(127, 212, 255, 0.45);
    transition: transform 180ms cubic-bezier(0.3, 1.6, 0.5, 1);
  }
  .big-play:hover {
    transform: scale(1.06);
  }
  .big-play:active {
    transform: scale(0.96);
  }
  .hint {
    font-size: 14px;
  }
  .notes {
    color: var(--tint);
    font-size: 15px;
  }
  .link {
    font-size: 13px;
    border: 0;
    background: transparent;
    color: var(--mist);
    text-decoration: underline;
    text-underline-offset: 3px;
  }
  .link:hover {
    color: var(--snow);
  }
  .done {
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
    gap: 12px;
    align-items: center;
    margin-top: 8px;
  }
  .primary {
    padding: 9px 20px;
    border-radius: 999px;
    border: 0;
    background: var(--pulse);
    color: var(--void);
    font-weight: 500;
  }

  .corner {
    position: absolute;
    left: 20px;
    bottom: 14px;
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .round {
    width: 30px;
    height: 30px;
    display: grid;
    place-items: center;
    border-radius: 50%;
    border: 1px solid var(--line-strong);
    background: rgba(22, 22, 29, 0.7);
  }
  .chip {
    font-size: 12px;
    padding: 6px 12px;
    border-radius: 999px;
    border: 1px solid var(--line);
    background: rgba(22, 22, 29, 0.7);
  }
  .wrong {
    color: var(--coral);
  }
  .seek {
    position: absolute;
    left: 0;
    right: 0;
    bottom: -9px;
    height: 14px;
    cursor: pointer;
    touch-action: none;
  }
  .seek:focus-visible {
    outline-offset: -2px;
  }
</style>
