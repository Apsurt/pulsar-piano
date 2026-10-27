<script lang="ts">
  // Practicing a song. The stage draws the notes and the progress; this screen keeps just the song title,
  // a play prompt and the score in view. Mode, tracks, view and speed live in the Options sheet.
  import { onDestroy, onMount } from "svelte";
  import { cubicOut } from "svelte/easing";
  import { fly } from "svelte/transition";
  import ReviewPanel from "../lib/ReviewPanel.svelte";
  import Segmented from "../lib/Segmented.svelte";
  import Sheet from "../lib/Sheet.svelte";
  import TopBar from "../lib/TopBar.svelte";
  import { nav, openEditor, practicePrefs, savePracticePrefs } from "../lib/app.svelte";
  import { onAction, onFader, onStrip } from "../lib/controls";
  import { engine } from "../lib/engine";
  import { live, type SongView } from "../lib/live";
  import { Practice, type Mode } from "../lib/practice";
  import { addPass, expectedFor, keepTake, noTakes, review, type Take, type Takes } from "../lib/review";
  import { noteName } from "../lib/keyboard";
  import { KEYBOARD, octaveLabel, planOctaves } from "../lib/octave";
  import { songs, trackCss, type Song } from "../lib/songs";

  let { song, songId }: { song: Song; songId: string } = $props();

  // A new screen is created per song, so the song is fixed for its lifetime.
  // svelte-ignore state_referenced_locally
  const practice = new Practice(song, practicePrefs.leadIn);
  practice.loop = practicePrefs.loop;
  // svelte-ignore state_referenced_locally
  practice.setPlayed(practicePrefs.tracks[songId] ?? []);
  // svelte-ignore state_referenced_locally
  practice.setHeard(practicePrefs.heard[songId] ?? []);
  practice.accompany = practicePrefs.accompany;
  // Notes Pulsar plays (listen mode, accompaniment) go to the synth like any other.
  practice.sound = (key, velocity) => engine.note(key, velocity);
  live.practice = practice;
  onDestroy(() => {
    practice.silence();
    engine.setKeyFilter(null).catch(() => {});
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
  let heard = $state([...practice.heard].sort((a, b) => a - b));
  /** The track choice on screen: what you play, or when listening, what you hear. */
  const selection = $derived(mode === "listen" ? heard : played);

  // Where to set your keyboard's octave buttons for the tracks you play, and any changes on the way.
  const octaves = $derived(
    planOctaves(
      practice.notes.filter((n) => played.includes(n.track) && n.start >= sectionFrom - 0.001 && n.start < sectionTo),
      practice.bars,
    ),
  );
  const keysFor = (shift: number) => `${noteName(KEYBOARD.low + 12 * shift)}–${noteName(KEYBOARD.high + 12 * shift)}`;
  /** While playing: an octave change coming up in the next few seconds (or just now). */
  const nextChange = $derived(
    octaves.changes.find((c) => c.at - time > -speed && c.at - time < 4 * speed) ?? null,
  );
  let view = $state<SongView>(live.songView);
  let optionsOpen = $state(false);
  let loop = $state(practice.loop);
  let accompany = $state(practice.accompany);
  let muteWrong = $state(practicePrefs.muteWrong);
  /** The key filter last sent to the engine ("all" when off), so it's only sent when it changes. */
  let sentFilter = "all";
  let leadIn = $state(practice.leadIn);
  let passes = $state(0);
  let lastScore = $state<number | null>(null);

  // ---- Takes: every full pass is recorded and scored; the latest, the best and the ones you keep are saved.
  let takes = $state<Takes>(noTakes());
  let lastTake = $state<Take | null>(null);
  let lastTakeAt = 0;
  /** Which pass the last take was: a pass that jumped around isn't recorded, so it has none. */
  let lastTakePass = $state(-1);
  let reviewing = $state(false);
  /** The record pad was tapped before or during this pass: keep it when it ends. */
  let keepNext = $state(false);

  function saveTakes() {
    songs.saveTakes(songId, $state.snapshot(takes) as Takes).catch(() => {});
  }

  practice.onPass = (recording) => {
    const take: Take = { at: Date.now(), recording, score: review(expectedFor(practice.notes, recording), recording).score };
    const best = addPass(takes, take);
    if (keepNext) keepTake(takes, take);
    lastTake = take;
    lastTakeAt = performance.now();
    lastTakePass = practice.passes;
    lastScore = take.score.overall;
    saveTakes();
    if (keepNext || best) show(keepNext ? "Take kept" : "New best");
    keepNext = false;
    if (!practice.loop) reviewing = true;
  };

  // ---- Part of the song: the left touch strip moves the start, the right one sets the end -----------------

  let sectionFrom = $state(practice.from);
  let sectionTo = $state(practice.to);
  let hasSection = $state(false);
  /** Left strip: -1 to 1, springing back to 0. Held away from the middle, it moves the start. */
  let jog = 0;
  let jogging = false;
  let lastJogAt = 0;

  /** Bar number (from 1) at song time `t`. */
  const barAt = (t: number) => Math.max(1, practice.bars.filter((b) => b <= t + 1e-3).length);
  /** Where notes start on the tracks on screen (yours, or the ones you hear): the points start and end snap to. */
  const noteStarts = () =>
    practice.chords.filter((c) => c.notes.some((n) => practice.shown.has(n.track))).map((c) => c.start);
  const nearestNote = (t: number, after = -Infinity) =>
    noteStarts()
      .filter((s) => s > after)
      .reduce<number | null>((best, s) => (best === null || Math.abs(s - t) < Math.abs(best - t) ? s : best), null);

  /** Called every frame: while the left strip is held, the start point travels (faster the further it's
   *  pushed); let go and it settles on the nearest note. */
  function updateJog(dt: number, now: number) {
    if (Math.abs(jog) > 0.04 && !reviewing) {
      if (practice.playing) practice.playing = false;
      const from = practice.from + Math.sign(jog) * jog * jog * 10 * dt;
      // Moving the start past the end lets the end go to the end of the song.
      const to = from >= practice.to - 0.05 ? practice.duration : practice.to;
      practice.setSection(from, to);
      show(`Start: bar ${barAt(practice.from)}`);
      jogging = true;
      lastJogAt = now;
    } else if (jogging && now - lastJogAt > 300) {
      jogging = false;
      practice.setSection(nearestNote(practice.from) ?? practice.firstNote, practice.to);
      show(`Start: bar ${barAt(practice.from)}`);
    }
  }

  /** Right strip, 0 (bottom) to 1 (top): where the pass stops, just before a note; the top is the end of the song. */
  function setEnd(value: number) {
    const raw = practice.from + value * (practice.duration - practice.from);
    const to = value >= 0.98 ? practice.duration : (nearestNote(raw, practice.from + 0.01) ?? practice.duration);
    practice.setEnd(to);
    show(practice.to >= practice.duration - 0.001 ? "To the end" : `Up to bar ${barAt(practice.to - 0.001)}`);
  }

  function wholeSong() {
    practice.setSection(practice.firstNote, practice.duration);
  }

  function keep(take: Take) {
    if (!keepTake(takes, take)) return;
    saveTakes();
    show("Take kept");
  }

  function forget(take: Take) {
    takes.kept = takes.kept.filter((t) => t.at !== take.at);
    saveTakes();
  }

  let reviewPanel = $state<ReturnType<typeof ReviewPanel> | null>(null);
  const hasTakes = $derived(!!(takes.latest || takes.best || takes.kept.length));

  /** The record pad: keeps the pass you just finished, or else the one you're about to play or playing. */
  function recordPad() {
    const justDone = lastTake && (reviewing || finished || performance.now() - lastTakeAt < 4000);
    if (justDone && !keepNext) return keep(lastTake!);
    if (mode === "listen") return;
    keepNext = !keepNext;
    show(keepNext ? "Keeping this take" : "Not keeping this take");
  }

  function again() {
    reviewing = false;
    practice.restart();
    practice.playing = true;
  }

  onMount(() => {
    songs
      .takes(songId)
      .then((saved) => {
        // A pass that already ended while loading wins over what was saved.
        if (saved && !lastTake) takes = saved;
      })
      .catch(() => {});
    const stopPads = onAction((action) => {
      if (action === "play") {
        // In the review, ▶ (and Space) play the take back; "Play again" there starts practicing.
        if (reviewing) reviewPanel?.togglePlayback();
        else toggle();
      } else if (action === "stop") {
        if (reviewing) reviewing = false;
        else practice.restart();
      } else if (action === "loop") setLoop(!loop);
      else if (action === "record") recordPad();
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
    // On this screen the touch strips move through the song instead of bending the sound.
    engine.setStripsAsControls(true).catch(() => {});
    const stopStrips = onStrip((strip, value) => {
      if (strip === "left") jog = value;
      else if (!reviewing) setEnd(value);
    });
    let lastFrame = performance.now();
    let raf = requestAnimationFrame(function tick(now) {
      updateJog(Math.min(0.1, (now - lastFrame) / 1000), now);
      lastFrame = now;
      sectionFrom = practice.from;
      sectionTo = practice.to;
      hasSection = practice.hasSection;
      time = practice.time;
      playing = practice.playing;
      finished = practice.finished;
      hits = practice.hits;
      wrong = practice.wrong;
      passes = practice.passes;
      // Wrong keys silent: the engine only lets the keys due now sound.
      const keys = muteWrong ? practice.soundKeys() : null;
      const filter = keys ? keys.join(",") : "all";
      if (filter !== sentFilter) {
        sentFilter = filter;
        engine.setKeyFilter(keys).catch(() => {});
      }
      raf = requestAnimationFrame(tick);
    });
    return () => {
      stopPads();
      stopFaders();
      stopStrips();
      engine.setStripsAsControls(false).catch(() => {});
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
    total = practice.total;
  }

  function setMuteWrong(on: boolean) {
    muteWrong = practicePrefs.muteWrong = on;
    savePracticePrefs();
  }

  function setAccompany(on: boolean) {
    accompany = practice.accompany = practicePrefs.accompany = on;
    savePracticePrefs();
  }

  /** Tracks with notes; empty ones aren't worth choosing. */
  const playable = practice.tracks.flatMap((t, i) => (t.notes ? [i] : []));
  const playingAny = (tracks: number[]) => tracks.some((t) => playable.includes(t));

  /** Turns a track on or off (to play, or to hear when listening); the last one with notes stays on. */
  function toggleTrack(track: number) {
    const next = selection.includes(track) ? selection.filter((t) => t !== track) : [...selection, track];
    if (!playingAny(next)) return;
    const listening = mode === "listen";
    if (listening) practice.setHeard(next);
    else practice.setPlayed(next);
    const chosen = [...(listening ? practice.heard : practice.played)].sort((a, b) => a - b);
    if (listening) heard = chosen;
    else played = chosen;
    total = practice.total;
    // Remembered per song; every track on is the default, so it isn't stored.
    const prefs = listening ? practicePrefs.heard : practicePrefs.tracks;
    if (chosen.length === practice.tracks.length) delete prefs[songId];
    else prefs[songId] = chosen;
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
    if (reviewing || (e.target as HTMLElement).closest?.("input, textarea")) return;
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
      {#if hasTakes}
        <button class="options takes" aria-pressed={reviewing} onclick={() => (reviewing = !reviewing)}>
          <svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true">
            <circle cx="12" cy="12" r="8" /><circle class="fill" cx="12" cy="12" r="3.5" />
          </svg>
          Takes
        </button>
      {/if}
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
                  { value: "listen", label: "Listen" },
                ]}
              />
              {#if mode === "listen"}
                <span class="hint dim">Pulsar plays the song for you</span>
              {/if}
            </div>
            {#if playable.length > 1}
              {@const label = mode === "listen" ? "Tracks to hear" : "Tracks you play"}
              <div class="field">
                <span class="label">{label}</span>
                <div class="tracks" role="group" aria-label={label}>
                  {#each playable as i (i)}
                    {@const track = practice.tracks[i]}
                    {@const on = selection.includes(i)}
                    {@const last = on && !playingAny(selection.filter((t) => t !== i))}
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
              {#if mode !== "listen" && playable.some((t) => !played.includes(t))}
                <div class="field">
                  <span class="label">The tracks you don't play</span>
                  <Segmented
                    label="The tracks you don't play"
                    value={accompany ? "play" : "show"}
                    onchange={(v) => setAccompany(v === "play")}
                    options={[
                      { value: "show", label: "Leave them out" },
                      { value: "play", label: "Pulsar plays them" },
                    ]}
                  />
                </div>
              {/if}
            {/if}
            <div class="field">
              <span class="label">Part of the song</span>
              <span class="plan">
                {#if hasSection}
                  Bars <b>{barAt(sectionFrom)}–{barAt(sectionTo - 0.001)}</b>
                  <button class="edit inline" onclick={wholeSong}>Whole song</button>
                {:else}
                  The whole song
                {/if}
                <br /><span class="dim">Left strip moves the start, right strip sets the end</span>
              </span>
            </div>
            {#if mode !== "listen"}
              <div class="field">
                <span class="label">Your keyboard</span>
                <span class="plan">
                  Start at <b>{octaveLabel(octaves.shift)}</b> <span class="dim">({keysFor(octaves.shift)})</span>
                  {#each octaves.changes as change (change.at)}
                    <br />then <b>{octaveLabel(change.shift)}</b> at bar {change.bar}
                  {/each}
                  {#if octaves.outside}
                    <br /><span class="dim">
                      {octaves.outside} note{octaves.outside === 1 ? "" : "s"} still off your keys
                    </span>
                  {/if}
                </span>
              </div>
              <div class="field">
                <span class="label">Wrong keys <span class="dim">(on your keyboard)</span></span>
                <Segmented
                  label="Wrong keys"
                  value={muteWrong ? "silent" : "sound"}
                  onchange={(v) => setMuteWrong(v === "silent")}
                  options={[
                    { value: "sound", label: "Make a sound" },
                    { value: "silent", label: "Silent" },
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
    <span class="chip done">
      Done{mode === "listen" ? "" : ` · ${lastTakePass === passes && lastScore !== null ? lastScore : accuracy}%`}
    </span>
    {#if lastTake}<button class="link" onclick={() => (reviewing = true)}>Review</button>{/if}
    <button class="link" onclick={() => practice.restart()}>Again</button>
  {:else if !playing}
    <span class="paused dim">Space or ▶ pad to {mode === "listen" ? "listen" : "play"}</span>
    {#if time > practice.startTime + 0.01}
      <button class="link" onclick={() => practice.restart()}>Start over</button>
    {/if}
  {/if}
  {#if mode === "listen"}
    <span class="chip listening">Listening</span>
  {:else}
    <span class="chip mono" title="Notes hit / notes to play">
      {hits}/{total}{#if wrong}<span class="wrong">{` · ${wrong}`}</span>{/if}
    </span>
  {/if}
  {#if adjusting}
    <span class="chip mono adjusting">{adjusting}</span>
  {:else if speed !== 1}
    <span class="chip mono">{Math.round(speed * 100)}%</span>
  {/if}
  {#if loop}
    <span class="chip loop" title="Starts over at the end">
      Loop{passes ? ` · pass ${passes + 1}` : ""}{lastScore !== null ? ` · last ${lastScore}%` : ""}
    </span>
    {#if lastTake && !finished}<button class="link" onclick={() => (reviewing = true)}>Review</button>{/if}
  {/if}
  {#if mode !== "listen"}
    {#if !playing && !finished && time <= practice.startTime + 0.01 && (octaves.shift || octaves.changes.length)}
      <span class="chip octave" title="Your keyboard's octave buttons, from its normal {keysFor(0)}">
        Set keyboard to {octaveLabel(octaves.shift)}{octaves.changes.length
          ? ` · ${octaves.changes.length} change${octaves.changes.length > 1 ? "s" : ""} later`
          : ""}
      </span>
    {:else if playing && nextChange}
      <span class="chip octave soon">{octaveLabel(nextChange.shift)} at bar {nextChange.bar}</span>
    {/if}
  {/if}
  {#if hasSection}
    <span class="chip section" title="Left strip moves the start, right strip the end">
      Bars {barAt(sectionFrom)}–{barAt(sectionTo - 0.001)}
      <button class="clear" aria-label="Practice the whole song" onclick={wholeSong}>×</button>
    </span>
  {/if}
  {#if keepNext}
    <span class="chip keeping" title="Tap the record pad again to undo">
      <span class="rec" aria-hidden="true"></span>Keeping this take
    </span>
  {/if}
</div>

{#if reviewing}
  <div class="review-layer" transition:fly={{ y: 18, duration: 240, easing: cubicOut }}>
    <ReviewPanel
      bind:this={reviewPanel}
      {practice}
      {takes}
      current={lastTake}
      onclose={() => (reviewing = false)}
      onagain={again}
      onkeep={keep}
      onforget={forget}
    />
  </div>
{/if}

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
  .hint {
    font-size: 12px;
  }
  .done,
  .loop,
  .listening,
  .adjusting {
    color: var(--tint);
    border-color: rgba(127, 212, 255, 0.4);
  }

  .section {
    display: flex;
    align-items: center;
    gap: 6px;
    padding-right: 6px;
    color: rgb(255, 181, 71);
    border-color: rgba(255, 181, 71, 0.45);
  }
  .clear {
    width: 18px;
    height: 18px;
    display: grid;
    place-items: center;
    padding: 0;
    border: 0;
    border-radius: 50%;
    background: rgba(255, 255, 255, 0.08);
    color: inherit;
    font-size: 13px;
    line-height: 1;
  }
  .edit.inline {
    margin-left: 8px;
    padding: 3px 10px;
    font-size: 12px;
  }
  .takes svg .fill {
    fill: var(--coral);
  }
  .takes[aria-pressed="true"] {
    background: rgba(255, 255, 255, 0.12);
  }
  .plan {
    font-size: 13px;
    line-height: 1.5;
  }
  .plan b {
    font-weight: 600;
    color: var(--tint);
  }
  .octave {
    color: var(--tint);
    border-color: rgba(127, 212, 255, 0.4);
  }
  .octave.soon {
    color: rgb(255, 181, 71);
    border-color: rgba(255, 181, 71, 0.5);
  }
  .keeping {
    display: flex;
    align-items: center;
    gap: 7px;
    border-color: rgba(255, 122, 89, 0.45);
  }
  .rec {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--coral);
    box-shadow: 0 0 8px var(--coral);
  }
  .review-layer {
    position: absolute;
    top: 72px;
    bottom: 58px;
    left: 16px;
    right: 16px;
    max-width: 1100px;
    margin: 0 auto;
    display: grid;
    z-index: 3;
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
