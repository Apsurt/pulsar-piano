<script lang="ts">
  // How a pass went, note by note: your take over the song's notes, the score and its four parts, your best
  // and kept takes to compare, and playback of your take or the song from anywhere on the roll.
  // Escape or Close goes back to practice.
  import { onDestroy, onMount, untrack } from "svelte";
  import Segmented from "./Segmented.svelte";
  import ReviewRoll from "./ReviewRoll.svelte";
  import { engine } from "./engine";
  import { NotePlayer, type Playable } from "./player";
  import type { Practice } from "./practice";
  import { expectedFor, review, type Take, type Takes } from "./review";
  import { formatTime } from "./songs";

  let {
    practice,
    takes,
    current,
    onclose,
    onagain,
    onkeep,
    onforget,
  }: {
    practice: Practice;
    takes: Takes;
    /** The pass just played, if any. */
    current: Take | null;
    onclose: () => void;
    onagain: () => void;
    onkeep: (take: Take) => void;
    /** Removes a kept take. */
    onforget: (take: Take) => void;
  } = $props();

  const barAt = (t: number) => Math.max(1, practice.bars.filter((b) => b <= t + 1e-3).length);
  const date = (at: number) =>
    new Date(at).toLocaleString(undefined, { day: "numeric", month: "short", hour: "2-digit", minute: "2-digit" });

  /** The takes you can look at: this pass (or your latest), your best, and the ones you kept (newest first). */
  const choices = $derived.by(() => {
    const list: { id: string; label: string; take: Take }[] = [];
    const first = current ?? takes.latest;
    if (first) list.push({ id: "pass", label: current ? "This pass" : "Latest", take: first });
    if (takes.best && takes.best.at !== first?.at) list.push({ id: "best", label: "Best", take: takes.best });
    for (const take of takes.kept.toReversed())
      if (take.at !== first?.at && take.at !== takes.best?.at)
        list.push({ id: `kept-${take.at}`, label: date(take.at), take });
    return list;
  });
  let chosen = $state("pass");
  const choice = $derived(choices.find((c) => c.id === chosen) ?? choices[0]);
  const take = $derived(choice?.take ?? null);

  const result = $derived(take ? review(expectedFor(practice.notes, take.recording), take.recording) : null);

  const isKept = $derived(!!take && takes.kept.some((t) => t.at === take.at));
  const isBest = $derived(!!take && takes.best?.at === take.at);
  const mode = $derived(take?.recording.mode ?? "wait");

  const ms = (s: number) => `${Math.round(Math.abs(s) * 1000)} ms`;
  const timingNote = $derived.by(() => {
    if (!result) return "";
    const avg = result.score.averageOffset;
    if (mode === "wait") return avg < 0.05 ? "no hesitation" : `${avg.toFixed(1)} s pause per chord`;
    if (Math.abs(avg) < 0.02) return "right on the beat";
    return `on average ${ms(avg)} ${avg > 0 ? "late" : "early"}`;
  });
  const lengthNote = $derived.by(() => {
    if (!result) return "";
    const short = result.notes.filter((n) => n.length === "short").length;
    const long = result.notes.filter((n) => n.length === "long").length;
    if (!short && !long) return "held as written";
    return [short && `${short} let go too soon`, long && `${long} held too long`].filter(Boolean).join(", ");
  });

  // ---- Playback: your take, the song, or both, at the speed the take was played -------------------------

  type Source = "take" | "song" | "both";
  let source = $state<Source>("take");
  let playing = $state(false);
  let playhead = $state<number | null>(null);
  const player = new NotePlayer((key, velocity) => engine.note(key, velocity).catch(() => {}));

  const songNotes = $derived(
    take ? practice.notes.filter((n) => take.recording.tracks.includes(n.track)).map(({ key, velocity, start, end }) => ({ key, velocity, start, end })) : [],
  );
  const notesFor = (s: Source): Playable[] =>
    s === "take" ? (take?.recording.played ?? []) : s === "song" ? songNotes : [...(take?.recording.played ?? []), ...songNotes];
  /** From just before the first note to just after the last, in song seconds. */
  const range = $derived.by(() => {
    const all = notesFor("both");
    return {
      start: Math.min(0, ...all.map((n) => n.start)) - 0.3,
      end: Math.max(0, ...all.map((n) => n.end)) + 0.3,
    };
  });

  // A different take starts again from the top, at its own speed. Only the take matters here: switching
  // what you hear carries on from the same place (setSource).
  $effect(() => {
    const chosenTake = take;
    untrack(() => {
      if (!chosenTake) return;
      player.pause();
      player.speed = chosenTake.recording.speed;
      player.setNotes(notesFor(source));
      player.seek(range.start);
      playing = false;
      playhead = range.start;
    });
  });

  function setSource(s: Source) {
    source = s;
    player.setNotes(notesFor(s));
  }

  function seek(time: number) {
    player.seek(time);
    playhead = time;
  }

  /** Play or pause (the ▶ pad and Space do this while the review is open). */
  export function togglePlayback() {
    if (playing) {
      player.pause();
      playing = false;
      return;
    }
    if (player.time >= range.end - 0.01) seek(range.start);
    player.play();
    playing = true;
  }

  onMount(() => {
    let last = performance.now();
    let raf = requestAnimationFrame(function frame(now) {
      player.update(Math.min(0.1, (now - last) / 1000));
      last = now;
      if (player.playing) {
        playhead = player.time;
        if (player.time >= range.end) {
          player.pause();
          playing = false;
        }
      }
      raf = requestAnimationFrame(frame);
    });
    return () => cancelAnimationFrame(raf);
  });
  onDestroy(() => player.pause());

  const clock = (t: number) => formatTime(Math.max(0, (t - range.start) / (take?.recording.speed ?? 1)));

  // Capture phase, so Escape closes the review instead of leaving the song.
  function keydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.stopPropagation();
      onclose();
    }
  }
</script>

<svelte:window onkeydowncapture={keydown} />

<section class="review" aria-label="How that pass went">
  {#if take && result}
    <header>
      <div class="headline">
        <span class="overall mono">{result.score.overall}<span class="pct">%</span></span>
        <div class="about">
          <span class="title">
            {choice?.label}
            {#if isBest}<span class="badge">Best</span>{/if}
          </span>
          <span class="dim">
            {mode === "wait" ? "Wait for me" : "Keep tempo"} · {Math.round(take.recording.speed * 100)}% speed
            {#if take.recording.section}
              · bars {barAt(take.recording.section.from)}–{barAt(take.recording.section.to - 0.001)}
            {/if}
            {#if takes.best && !isBest} · best {takes.best.score.overall}%{/if}
          </span>
        </div>
      </div>
      {#if choices.length > 1}
        <div class="choices" role="group" aria-label="Takes">
          {#each choices as c (c.id)}
            <button class:on={c.id === choice?.id} aria-pressed={c.id === choice?.id} onclick={() => (chosen = c.id)}>
              {c.label} <span class="mono dim">{c.take.score.overall}%</span>
            </button>
          {/each}
        </div>
      {/if}
    </header>

    <div class="tiles">
      <div class="tile">
        <span class="label">Notes</span>
        <span class="value mono">{result.score.notes.hit}/{result.score.notes.total}</span>
        <span class="dim">
          {result.score.notes.total - result.score.notes.hit ? `${result.score.notes.total - result.score.notes.hit} missed` : "none missed"}
        </span>
      </div>
      <div class="tile">
        <span class="label">Timing</span>
        <span class="value mono">{result.score.timing}%</span>
        <span class="dim">{timingNote}</span>
      </div>
      <div class="tile">
        <span class="label">Note length</span>
        <span class="value mono">{result.score.length}%</span>
        <span class="dim">{lengthNote}</span>
      </div>
      <div class="tile">
        <span class="label">Wrong keys</span>
        <span class="value mono">{result.score.extra}</span>
        <span class="dim">{result.score.extra === 1 ? "extra press" : "extra presses"}</span>
      </div>
    </div>

    <ReviewRoll review={result} bars={practice.bars} {mode} {playhead} onseek={seek} />

    <div class="transport">
      <button class="round" onclick={togglePlayback} aria-label={playing ? "Pause playback" : "Play back"} title="Space or ▶ pad">
        {#if playing}
          <svg viewBox="0 0 24 24" width="14" height="14" aria-hidden="true"><path d="M8 5v14M16 5v14" /></svg>
        {:else}
          <svg viewBox="0 0 24 24" width="14" height="14" aria-hidden="true"><path class="fill" d="M8 5l11 7-11 7z" /></svg>
        {/if}
      </button>
      <span class="mono dim time">{clock(playhead ?? range.start)}</span>
      <input
        class="scrub"
        type="range"
        min={range.start}
        max={range.end}
        step="0.01"
        value={playhead ?? range.start}
        oninput={(e) => seek(Number(e.currentTarget.value))}
        aria-label="Playback position"
      />
      <span class="mono dim time">{clock(range.end)}</span>
      <Segmented
        label="What to hear"
        value={source}
        onchange={setSource}
        options={[
          { value: "take", label: "Your take" },
          { value: "song", label: "The song" },
          { value: "both", label: "Both" },
        ]}
      />
    </div>

    <footer>
      <div class="legend dim">
        <span><i class="good"></i>On time</span>
        <span><i class="off"></i>{mode === "wait" ? "Hesitated" : "Early or late"}</span>
        <span><i class="missed"></i>Missed</span>
        <span><i class="extra"></i>Wrong key</span>
        <span><i class="song"></i>The song</span>
      </div>
      <div class="actions">
        <button class="ghost" onclick={onclose}>Close</button>
        {#if isKept}
          <button class="ghost" onclick={() => onforget(take)}>Remove from kept</button>
        {:else}
          <button class="ghost keep" onclick={() => onkeep(take)} title="Or tap the record pad">
            <span class="dot" aria-hidden="true"></span>Keep this take
          </button>
        {/if}
        <button class="primary" onclick={onagain}>Play again</button>
      </div>
    </footer>
  {/if}
</section>

<style>
  .review {
    display: grid;
    grid-template-rows: auto auto minmax(120px, 1fr) auto auto;
    gap: 14px;
    padding: 18px;
    border-radius: 22px;
    background: rgba(18, 18, 25, 0.94);
    border: 1px solid var(--line-strong);
    box-shadow: 0 24px 70px rgba(0, 0, 0, 0.55);
    backdrop-filter: blur(24px);
  }
  .dim {
    color: var(--mist);
  }
  header {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .headline {
    display: flex;
    align-items: center;
    gap: 16px;
  }
  .overall {
    font-size: 44px;
    font-weight: 600;
    letter-spacing: -0.04em;
    line-height: 1;
    color: var(--tint);
    text-shadow: 0 0 24px rgba(127, 212, 255, 0.35);
  }
  .pct {
    font-size: 22px;
    margin-left: 2px;
  }
  .about {
    display: grid;
    gap: 3px;
    font-size: 12px;
  }
  .title {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 16px;
    font-weight: 600;
  }
  .badge {
    padding: 2px 8px;
    border-radius: 999px;
    font-size: 11px;
    font-weight: 600;
    color: var(--void);
    background: rgb(110, 231, 183);
  }
  .choices {
    display: flex;
    gap: 4px;
    max-width: 100%;
    overflow-x: auto;
  }
  .choices button {
    flex: none;
    padding: 6px 12px;
    border-radius: 999px;
    border: 1px solid var(--line);
    background: transparent;
    color: var(--mist);
    font-size: 12px;
  }
  .choices button.on {
    color: var(--snow);
    background: rgba(255, 255, 255, 0.08);
    border-color: var(--line-strong);
  }
  .tiles {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 10px;
  }
  .tile {
    display: grid;
    gap: 2px;
    padding: 10px 12px;
    border-radius: 14px;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid var(--line);
    font-size: 12px;
    min-width: 0;
  }
  .tile .label {
    color: var(--mist);
  }
  .tile .value {
    font-size: 22px;
    font-weight: 600;
  }
  .tile .dim {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  footer {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }
  .legend {
    display: flex;
    flex-wrap: wrap;
    gap: 14px;
    font-size: 12px;
  }
  .legend span {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .legend i {
    width: 14px;
    height: 8px;
    border-radius: 3px;
  }
  .legend .good {
    background: rgb(110, 231, 183);
  }
  .legend .off {
    background: rgb(255, 181, 71);
  }
  .legend .missed {
    border: 1.5px dashed rgb(255, 122, 89);
  }
  .legend .extra {
    background: rgb(255, 122, 89);
  }
  .legend .song {
    border: 1.5px solid rgba(255, 255, 255, 0.45);
  }
  .actions {
    display: flex;
    gap: 8px;
  }
  .transport {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .round {
    flex: none;
    width: 32px;
    height: 32px;
    display: grid;
    place-items: center;
    border-radius: 50%;
    border: 1px solid var(--line-strong);
    background: rgba(255, 255, 255, 0.06);
  }
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
  .time {
    font-size: 12px;
    min-width: 4ch;
  }
  .scrub {
    flex: 1;
    min-width: 80px;
    accent-color: var(--pulse);
  }
  button {
    font: inherit;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    color: var(--snow);
  }
  button:disabled {
    opacity: 0.55;
    cursor: default;
  }
  .ghost {
    padding: 8px 14px;
    border-radius: 999px;
    border: 1px solid var(--line);
    background: transparent;
    color: var(--mist);
  }
  .ghost:hover:not(:disabled) {
    color: var(--snow);
  }
  .keep {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .keep .dot {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: var(--coral);
  }
  .primary {
    padding: 8px 18px;
    border-radius: 999px;
    border: 0;
    background: var(--pulse);
    color: var(--void);
    box-shadow: 0 0 20px rgba(127, 212, 255, 0.3);
  }
</style>
