<script lang="ts">
  // Placeholder: only proves the webview can reach the Rust side.
  import { invoke } from "@tauri-apps/api/core";

  type Ping = { app: string; engine: string; flavor: string };

  const ping = invoke<Ping>("ping");
  const update = invoke<string | null>("check_update");
</script>

<main>
  <h1>Pulsar</h1>

  {#await ping}
    <p>connecting…</p>
  {:then p}
    <p class="ok">backend ok · app {p.app} · engine {p.engine} · {p.flavor}</p>
  {:catch e}
    <p class="err">backend error: {e}</p>
  {/await}

  {#await update then version}
    {#if version}
      <button onclick={() => invoke("install_update")}>Install update {version}</button>
    {/if}
  {:catch e}
    <p class="muted">update check failed: {e}</p>
  {/await}
</main>

<style>
  :global(body) {
    margin: 0;
    background: #0b0b0f;
    color: #e8e8ee;
    font-family: system-ui, sans-serif;
  }
  main {
    display: grid;
    place-content: center;
    min-height: 100vh;
    text-align: center;
  }
  .ok { color: #7ee2a8; }
  .err { color: #ff7a7a; }
  .muted { color: #777; font-size: 0.85rem; }
</style>
