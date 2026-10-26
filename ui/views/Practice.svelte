<script lang="ts">
  // Practicing a song. The stage draws the notes and the progress; this screen keeps just the song title,
  // a play prompt and the score in view. Mode, tracks, view and speed live in the Options sheet.
  import { onDestroy, onMount } from "svelte";
  import Segmented from "../lib/Segmented.svelte";
  import Sheet from "../lib/Sheet.svelte";
  import TopBar from "../lib/TopBar.svelte";
  import { nav, openEditor, practicePrefs, savePracticePrefs } from "../lib/app.svelte";
  import { onAction, onFader } from "../lib/controls";
  import { live, type SongView } from "../lib/live";
  import { Practice, type Mode } from "../lib/practice";
  import { trackCss, type Song } from "../lib/songs";

  let { song, songId }: { song: Song; songId: string } = $props();

  // A new screen is created per song, so the song is fixed for its lifetime.
  // svelte-ignore state_referenced_locally
  const practice = new Practice(song, practicePrefs.leadIn);
  practice.loop = practicePrefs.loop;
  // svelte-ignore state_referenced_locally
  practice.setPlayed(practicePrefs.tracks[songId] ?? []);
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
  let speed = $state(1);
  let mode = $state<Mode>("wait");
  let played = $state([...practice.played].sort((a, b) => a - b));
  let view = $state<SongView>(live.songView);
  let optionsOpen = $state(false);
  let loop = $state(practice.loop);
  let leadIn = $state(practice.leadIn);
  let passes = $state(0);
  let lastScore = $state<number | null>(null);

  onMount(() => {
    const stopPads = onAction((action) => {
      if (action === "play") toggle();
      else if (action === "stop") practice.restart();
      else if (action === "loop") setLoop(!loop);
    });
    // Fader 1: speed, fader 2: lead-in.
    const stopFaders = onFader((fader, value) => {
      if (fader === 1) {
        setSpeed(0.25 + value * 1.25);
        show(`Speed ${Math.round(speed * 100)}%`);
      } else if (fader === 2) {
        setLeadIn(value * 10);
        show(`Lead-in ${leadIn} s`);
      }
    });
    let raf = requestAnimationFrame(function tick() {
      time = practice.time;
      playing = practice.playing;
      finished = practice.finished;
      hits = practice.hits;
      wrong = practice.wrong;
      passes = practice.passes;
      lastScore = practice.lastScore;
      raf = requestAnimationFrame(tick);
    });
    return () => {
      stopPads();
      stopFaders();
      cancelAnimationFrame(raf);
    };
  });

  /** A value being adjusted from the keyboard, shown briefly in the corner. */
  let adjusting = $state<string | null>(null);
  let adjustingTimer: ReturnType<typeof setTimeout>;
  function show(text: string) {
    adjusting = text;
    clearTimeout(adjustingTimer);
    adjustingTimer = setTimeout(() => (adjusting = null), 1200);
  }

  function toggle() {
    if (practice.finished) practice.restart();
    practice.playing = !practice.playing;
  }

  /** 25–150%, in 5% steps. */
  function setSpeed(value: number) {
    speed = Math.min(1.5, Math.max(0.25, Math.round(value * 20) / 20));
    practice.setSpeed(speed);
  }

  function setLoop(on: boolean) {
    loop = practice.loop = practicePrefs.loop = on;
    savePracticePrefs();
  }

  /** 0–10 whole seconds. */
  function setLeadIn(value: number) {
    const next = Math.min(10, Math.max(0, Math.round(value)));
    if (next === leadIn) return;
    leadIn = next;
    practice.setLeadIn(leadIn);
    practicePrefs.leadIn = leadIn;
    savePracticePrefs();
  }

  function setMode(m: Mode) {
    mode = m;
    practice.setMode(m);
  }

  /** Tracks with notes; empty ones aren't worth choosing. */
  const playable = practice.tracks.flatMap((t, i) => (t.notes ? [i] : []));
  const playingAny = (tracks: number[]) => tracks.some((t) => playable.includes(t));

  /** Turns a track on or off; the last one with notes stays on. */
  function toggleTrack(track: number) {
    const next = played.includes(track) ? played.filter((t) => t !== track) : [...played, track];
    if (!playingAny(next)) return;
    practice.setPlayed(next);
    played = [...practice.played].sort((a, b) => a - b);
    total = practice.total;
    if (played.length === practice.tracks.length) delete practicePrefs.tracks[songId];
    else practicePrefs.tracks[songId] = played;
    savePracticePrefs();
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
            {#if playable.length > 1}
              <div class="field">
                <span class="label">Tracks you play</span>
                <div class="tracks" role="group" aria-label="Tracks you play">
                  {#each playable as i (i)}
                    {@const track = practice.tracks[i]}
                    {@const on = played.includes(i)}
                    {@const last = on && !playingAny(played.filter((t) => t !== i))}
                    <button
                      class="track"
                      class:on
                      aria-pressed={on}
                      disabled={last}
                      title={last ? "At least one track stays on" : undefined}
                      onclick={() => toggleTrack(i)}
                    >
                      <span class="swatch" style:--color={trackCss(i)} aria-hidden="true"></span>
                      <span class="name">{track.name}</span>
                      <span class="count dim">{track.notes}</span>
                    </button>
                  {/each}
                </div>
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
              <span class="label">Speed <span class="dim">(fader 1)</span></span>
              <div class="stepper" role="group" aria-label="Speed">
                <button onclick={() => setSpeed(speed - 0.05)} aria-label="Slower">−</button>
                <span class="mono">{Math.round(speed * 100)}%</span>
                <button onclick={() => setSpeed(speed + 0.05)} aria-label="Faster">+</button>
              </div>
            </div>
            <div class="field">
              <span class="label">Time before the first note <span class="dim">(fader 2)</span></span>
              <div class="stepper" role="group" aria-label="Lead-in">
                <button onclick={() => setLeadIn(leadIn - 1)} aria-label="Shorter lead-in">−</button>
                <span class="mono">{leadIn} s</span>
                <button onclick={() => setLeadIn(leadIn + 1)} aria-label="Longer lead-in">+</button>
              </div>
            </div>
            <div class="field">
              <span class="label">Loop <span class="dim">(or the loop pad)</span></span>
              <Segmented
                label="Loop"
                value={loop ? "on" : "off"}
                onchange={(v) => setLoop(v === "on")}
                options={[
                  { value: "off", label: "Play once" },
                  { value: "on", label: "Loop" },
                ]}
              />
            </div>
            <button class="edit" onclick={() => openEditor(songId)}>Edit notes</button>
          </div>
        </Sheet>
      </div>
    {/snippet}
  </TopBar>
</div>

<!-- Everything sits in this corner; the middle of the screen is only for the notes. -->
<div class="corner">
  <button class="round" onclick={toggle} aria-label={playing ? "Pause song" : "Play song"}>
    {#if playing}
      <svg viewBox="0 0 24 24" width="14" height="14" aria-hidden="true"><path d="M8 5v14M16 5v14" /></svg>
    {:else}
      <svg viewBox="0 0 24 24" width="14" height="14" aria-hidden="true"><path class="fill" d="M8 5l11 7-11 7z" /></svg>
    {/if}
  </button>
  {#if finished}
    <span class="chip done">Done · {accuracy}%</span>
    <button class="link" onclick={() => practice.restart()}>Again</button>
  {:else if !playing}
    <span class="paused dim">Space or ▶ pad to play</span>
    {#if time > practice.startTime + 0.01}
      <button class="link" onclick={() => practice.restart()}>Start over</button>
    {/if}
  {/if}
  <span class="chip mono" title="Notes hit / notes to play">
    {hits}/{total}{#if wrong}<span class="wrong">{` · ${wrong}`}</span>{/if}
  </span>
  {#if adjusting}
    <span class="chip mono adjusting">{adjusting}</span>
  {:else if speed !== 1}
    <span class="chip mono">{Math.round(speed * 100)}%</span>
  {/if}
  {#if loop}
    <span class="chip loop" title="Starts over at the end">
      Loop{passes ? ` · pass ${passes + 1}` : ""}{lastScore !== null ? ` · last ${lastScore}%` : ""}
    </span>
  {/if}
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
  .tracks {
    display: grid;
    gap: 4px;
    width: 100%;
  }
  .track {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 7px 10px;
    border-radius: 10px;
    border: 1px solid var(--line);
    background: transparent;
    color: var(--mist);
    font-size: 13px;
    text-align: left;
    transition:
      background 120ms,
      color 120ms;
  }
  .track:hover {
    color: var(--snow);
  }
  .track.on {
    color: var(--snow);
    background: rgba(255, 255, 255, 0.06);
    border-color: var(--line-strong);
  }
  .track:disabled {
    cursor: default;
  }
  .swatch {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    flex: none;
    background: var(--color);
    opacity: 0.3;
  }
  .track.on .swatch {
    opacity: 1;
    box-shadow: 0 0 8px var(--color);
  }
  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .count {
    font-family: var(--mono);
    font-size: 11px;
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

  .paused {
    font-size: 12px;
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
  .done,
  .loop,
  .adjusting {
    color: var(--tint);
    border-color: rgba(127, 212, 255, 0.4);
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
