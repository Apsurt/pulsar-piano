<script lang="ts">
  // Everything that used to crowd the top bar: sound, MIDI, audio output, updates.
  import { onMount } from "svelte";
  import { getVersion } from "@tauri-apps/api/app";
  import SoundPanel from "../lib/SoundPanel.svelte";
  import TopBar from "../lib/TopBar.svelte";
  import { nav, status } from "../lib/app.svelte";
  import { engine } from "../lib/engine";

  let version = $state("");
  onMount(() => {
    getVersion()
      .then((v) => (version = v))
      .catch(() => {});
  });
</script>

<div class="settings">
  <TopBar back={nav.backLabel} onback={() => nav.back()} title="Settings" />

  <div class="content">
    <section>
      <h2>Sound</h2>
      <div class="card"><SoundPanel /></div>
    </section>

    <section>
      <h2>MIDI</h2>
      <div class="card">
        {#each status.devices as device (device.name)}
          <div class="row">
            <span class="led" class:on={device.connected}></span>
            <span class="grow">{device.name}</span>
            <span class="dim">{device.connected ? "Connected" : "In use by another app"}</span>
          </div>
        {:else}
          <div class="dim">No MIDI devices found. Plug one in and Pulsar picks it up within a second.</div>
        {/each}
      </div>
    </section>

    <section>
      <h2>Audio output</h2>
      <div class="card">
        <div class="row">
          <span class="grow">{status.output || "—"}</span>
          <span class="dim mono">
            {status.sampleRate ? `${status.sampleRate / 1000} kHz` : ""}{status.latency !== null
              ? ` · ${status.latency.toFixed(1)} ms`
              : ""}
          </span>
        </div>
        <div class="dim small">Latency is time from playing to hearing; wired headphones are usually the lowest.</div>
      </div>
    </section>

    <section>
      <h2>About</h2>
      <div class="card">
        <div class="row">
          <span class="grow">Pulsar {version}</span>
          {#if status.update}
            <button class="primary" onclick={() => engine.installUpdate()}>Install update {status.update}</button>
          {:else}
            <span class="dim">Up to date</span>
          {/if}
        </div>
      </div>
    </section>
  </div>
</div>

<style>
  .settings {
    position: absolute;
    inset: 0;
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    background: rgba(10, 10, 15, 0.9);
    backdrop-filter: blur(18px);
  }
  .content {
    overflow-y: auto;
    padding: 8px max(24px, calc((100% - 560px) / 2)) 32px;
    display: grid;
    gap: 24px;
    align-content: start;
  }
  h2 {
    margin: 0 0 8px 4px;
    font-size: 13px;
    font-weight: 600;
    color: var(--mist);
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }
  .card {
    display: grid;
    gap: 10px;
    padding: 14px;
    border-radius: 18px;
    background: rgba(22, 22, 29, 0.85);
    border: 1px solid var(--line);
    font-size: 14px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .grow {
    flex: 1;
    min-width: 0;
  }
  .dim {
    color: var(--mist);
    font-size: 13px;
  }
  .small {
    font-size: 12px;
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
  .primary {
    padding: 8px 16px;
    border-radius: 999px;
    border: 0;
    background: var(--pulse);
    color: var(--void);
    font: inherit;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
  }
</style>
