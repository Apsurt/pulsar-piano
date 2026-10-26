<script lang="ts">
  // Pick the instrument and set the volume. Used in sheets and in Settings.
  import { chooseInstrument, setVolume, sound } from "./app.svelte";
  import { engine } from "./engine";
</script>

<div class="panel">
  <div class="list" role="radiogroup" aria-label="Instrument">
    {#each sound.instruments as instrument (instrument.id)}
      <button
        role="radio"
        aria-checked={sound.current === instrument.id}
        class:on={sound.current === instrument.id}
        class:loading={sound.loading === instrument.id}
        disabled={sound.loading !== null && sound.loading !== instrument.id}
        onclick={() => chooseInstrument(instrument.id)}
      >
        <span class="name">{instrument.name}</span>
        <span class="kind">
          {sound.loading === instrument.id ? "Loading…" : instrument.kind === "builtin" ? "Built-in" : "SoundFont"}
        </span>
      </button>
    {/each}
  </div>
  <button class="add" onclick={() => engine.openSoundsFolder()}>Add sounds… <span>(drop .sf2 files in the folder)</span></button>

  <label class="volume">
    <span>Volume</span>
    <input
      type="range"
      min="-40"
      max="6"
      step="0.5"
      value={sound.volume}
      oninput={(e) => setVolume(Number(e.currentTarget.value))}
      style:--fill="{((sound.volume + 40) / 46) * 100}%"
    />
    <span class="mono db">{sound.volume.toFixed(1)} dB</span>
  </label>
</div>

<style>
  .panel {
    display: grid;
    gap: 12px;
  }
  .list {
    display: grid;
    gap: 4px;
  }
  .list button {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    border-radius: 12px;
    border: 1px solid transparent;
    background: rgba(255, 255, 255, 0.04);
    color: var(--snow);
    font: inherit;
    font-size: 14px;
    text-align: left;
    cursor: pointer;
  }
  .list button:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.08);
  }
  .list button.on {
    border-color: rgba(127, 212, 255, 0.55);
    background: rgba(127, 212, 255, 0.12);
  }
  .list button:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .list button.loading {
    background: linear-gradient(90deg, rgba(127, 212, 255, 0.05), rgba(127, 212, 255, 0.22), rgba(127, 212, 255, 0.05));
    background-size: 200% 100%;
    animation: shimmer 1.1s linear infinite;
  }
  .name {
    font-weight: 500;
  }
  .kind {
    font-size: 12px;
    color: var(--mist);
  }
  .add {
    justify-self: start;
    padding: 4px 2px;
    border: 0;
    background: transparent;
    color: var(--pulse);
    font: inherit;
    font-size: 13px;
    cursor: pointer;
  }
  .add span {
    color: var(--mist);
    font-size: 12px;
  }
  .volume {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 13px;
    padding-top: 10px;
    border-top: 1px solid var(--line);
  }
  .volume input {
    flex: 1;
  }
  .db {
    width: 7ch;
    text-align: right;
    color: var(--mist);
    font-size: 12px;
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
    width: 16px;
    height: 16px;
    margin-top: -6px;
    border-radius: 50%;
    background: var(--snow);
  }
  @keyframes shimmer {
    from {
      background-position: 100% 0;
    }
    to {
      background-position: -100% 0;
    }
  }
</style>
