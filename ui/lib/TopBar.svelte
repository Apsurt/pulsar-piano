<script lang="ts">
  // The one bar every screen shares: ‹ back, a title, and the screen's few actions.
  import type { Snippet } from "svelte";

  let {
    back,
    onback,
    title,
    subtitle,
    heading,
    actions,
  }: {
    back?: string;
    onback?: () => void;
    title?: string;
    subtitle?: string;
    /** Replaces the title, e.g. with an editable field. */
    heading?: Snippet;
    actions?: Snippet;
  } = $props();
</script>

<header class="topbar">
  {#if onback}
    <button class="back" onclick={onback}>
      <svg viewBox="0 0 24 24" width="18" height="18" aria-hidden="true"><path d="M15 6l-6 6 6 6" /></svg>
      <span>{back}</span>
    </button>
  {/if}
  <div class="heading">
    {#if heading}
      {@render heading()}
    {:else}
      {#if title}<h1>{title}</h1>{/if}
      {#if subtitle}<span class="subtitle">{subtitle}</span>{/if}
    {/if}
  </div>
  {#if actions}<div class="actions">{@render actions()}</div>{/if}
</header>

<style>
  .topbar {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 14px 18px;
    min-height: 64px;
    box-sizing: border-box;
  }
  .back {
    display: flex;
    align-items: center;
    gap: 2px;
    flex-shrink: 0;
    padding: 6px 10px 6px 4px;
    border: 0;
    border-radius: 10px;
    background: transparent;
    color: var(--pulse);
    font: inherit;
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    max-width: 220px;
  }
  .back span {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .back:hover {
    background: rgba(127, 212, 255, 0.08);
  }
  svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2.2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .heading {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: baseline;
    gap: 10px;
  }
  h1 {
    margin: 0;
    font-size: 20px;
    font-weight: 600;
    letter-spacing: -0.02em;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .subtitle {
    font-size: 13px;
    color: var(--mist);
    white-space: nowrap;
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }
</style>
