<script lang="ts">
  // A small floating panel for secondary controls, opened from a button and closed by clicking
  // outside or pressing Escape. Put it inside a `position: relative` wrapper next to its button.
  import type { Snippet } from "svelte";
  import { cubicOut } from "svelte/easing";
  import { scale } from "svelte/transition";

  let {
    open,
    onclose,
    label,
    align = "right",
    width = 280,
    children,
  }: {
    open: boolean;
    onclose: () => void;
    label: string;
    align?: "left" | "right";
    width?: number;
    children: Snippet;
  } = $props();

  // Capture phase, so Escape closes the sheet before anything else (like "go back") sees it.
  function keydown(e: KeyboardEvent) {
    if (open && e.key === "Escape") {
      e.stopPropagation();
      onclose();
    }
  }
</script>

<svelte:window onkeydowncapture={keydown} />

{#if open}
  <button class="backdrop" tabindex="-1" aria-label="Close {label}" onclick={onclose}></button>
  <div
    class="sheet"
    class:left={align === "left"}
    style:width="{width}px"
    role="dialog"
    aria-label={label}
    transition:scale={{ start: 0.96, duration: 160, easing: cubicOut }}
  >
    {@render children()}
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 20;
    border: 0;
    background: transparent;
    cursor: default;
  }
  .sheet {
    position: absolute;
    top: calc(100% + 8px);
    right: 0;
    z-index: 21;
    max-width: calc(100vw - 32px);
    padding: 14px;
    border-radius: 18px;
    background: rgba(22, 22, 29, 0.96);
    border: 1px solid var(--line-strong);
    box-shadow: 0 18px 50px rgba(0, 0, 0, 0.5);
    backdrop-filter: blur(24px);
    transform-origin: top right;
  }
  .sheet.left {
    right: auto;
    left: 0;
    transform-origin: top left;
  }
</style>
