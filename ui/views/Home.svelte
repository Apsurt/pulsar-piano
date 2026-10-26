<script lang="ts">
  // The calm top level: play freely, or go to your songs. Settings sit behind the gear.
  import { onMount } from "svelte";
  import { lastSongId, nav, openSong, status } from "../lib/app.svelte";
  import { songs, type SongSummary } from "../lib/songs";

  let list = $state<SongSummary[]>([]);

  onMount(() => {
    songs
      .list()
      .then((l) => (list = l))
      .catch(() => {});
  });

  const last = $derived(list.find((s) => s.id === lastSongId()));
  const keyboard = $derived(status.devices.find((d) => d.connected));
</script>

<section class="home">
  <div class="corner">
    <span class="device" title={keyboard ? `${keyboard.name} connected` : "No MIDI keyboard connected"}>
      <span class="led" class:on={!!keyboard}></span>
      {keyboard?.name ?? "No keyboard"}
    </span>
    <button class="gear" aria-label="Settings" onclick={() => nav.push({ name: "settings" })}>
      <svg viewBox="0 0 24 24" width="18" height="18" aria-hidden="true">
        <circle cx="12" cy="12" r="3" />
        <path
          d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z"
        />
      </svg>
    </button>
  </div>

  <div class="center">
    <div class="wordmark"><span class="mark" aria-hidden="true"></span>Pulsar</div>
    <div class="tiles">
      <button class="tile" onclick={() => nav.push({ name: "play" })}>
        <span class="tile-title">Play freely</span>
        <span class="dim">Just you and the keys</span>
      </button>
      <button class="tile" onclick={() => nav.push({ name: "songs" })}>
        <span class="tile-title">Songs</span>
        <span class="dim">{list.length ? `${list.length} ${list.length === 1 ? "song" : "songs"}` : "Learn something new"}</span>
      </button>
    </div>
    {#if last}
      <button class="continue" onclick={() => openSong(last.id)}>
        Continue <b>{last.title}</b>
        <svg viewBox="0 0 24 24" width="14" height="14" aria-hidden="true"><path d="M9 6l6 6-6 6" /></svg>
      </button>
    {/if}
  </div>
</section>

<style>
  .home {
    position: absolute;
    inset: 0;
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
  .corner {
    position: absolute;
    top: 16px;
    right: 18px;
    display: flex;
    align-items: center;
    gap: 14px;
    font-size: 13px;
    color: var(--mist);
  }
  .device {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .led {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--slate);
  }
  .led.on {
    background: var(--pulse);
    box-shadow: 0 0 8px var(--pulse);
  }
  .gear {
    width: 38px;
    height: 38px;
    display: grid;
    place-items: center;
    border-radius: 50%;
    border: 1px solid var(--line);
    background: rgba(22, 22, 29, 0.6);
    color: var(--snow);
    cursor: pointer;
  }
  .gear:hover {
    border-color: var(--line-strong);
  }

  .center {
    position: absolute;
    inset: 0;
    display: grid;
    place-content: center;
    justify-items: center;
    gap: 36px;
  }
  .wordmark {
    display: flex;
    align-items: center;
    gap: 16px;
    font-size: 44px;
    font-weight: 600;
    letter-spacing: -0.04em;
  }
  .mark {
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: var(--tint);
    box-shadow:
      0 0 10px var(--pulse),
      0 0 34px var(--pulse);
  }
  .tiles {
    display: flex;
    gap: 16px;
  }
  .tile {
    width: 230px;
    height: 132px;
    display: grid;
    align-content: end;
    gap: 4px;
    padding: 20px 22px;
    text-align: left;
    border-radius: 24px;
    border: 1px solid var(--line);
    background: linear-gradient(160deg, rgba(36, 36, 46, 0.85), rgba(22, 22, 29, 0.85));
    color: var(--snow);
    font: inherit;
    cursor: pointer;
    transition:
      transform 220ms cubic-bezier(0.3, 1.5, 0.5, 1),
      border-color 200ms ease,
      box-shadow 260ms ease;
  }
  .tile:hover {
    transform: translateY(-3px);
    border-color: rgba(127, 212, 255, 0.45);
    box-shadow: 0 0 40px rgba(127, 212, 255, 0.16);
  }
  .tile:active {
    transform: scale(0.98);
  }
  .tile-title {
    font-size: 22px;
    font-weight: 600;
    letter-spacing: -0.02em;
  }
  .tile .dim {
    font-size: 13px;
  }
  .continue {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 14px;
    border-radius: 999px;
    border: 0;
    background: transparent;
    color: var(--mist);
    font: inherit;
    font-size: 14px;
    cursor: pointer;
  }
  .continue b {
    color: var(--snow);
    font-weight: 600;
  }
  .continue:hover {
    background: rgba(255, 255, 255, 0.05);
  }
</style>
