<script lang="ts" generics="T extends string">
  // A pill-shaped segmented control: one choice out of a few.
  let {
    label,
    options,
    value,
    onchange,
    disabled = false,
  }: {
    label: string;
    options: { value: T; label: string }[];
    value: T;
    onchange: (value: T) => void;
    disabled?: boolean;
  } = $props();
</script>

<div class="segmented" role="radiogroup" aria-label={label}>
  {#each options as option (option.value)}
    <button
      role="radio"
      aria-checked={value === option.value}
      class:on={value === option.value}
      {disabled}
      onclick={() => onchange(option.value)}
    >
      {option.label}
    </button>
  {/each}
</div>

<style>
  .segmented {
    display: inline-flex;
    padding: 3px;
    gap: 2px;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid var(--line);
  }
  button {
    font-size: 12px;
    font-weight: 500;
    padding: 6px 12px;
    border: 0;
    border-radius: 999px;
    background: transparent;
    color: var(--mist);
    cursor: pointer;
    transition:
      background 160ms ease,
      color 160ms ease;
  }
  button:hover:not(:disabled):not(.on) {
    color: var(--snow);
  }
  button.on {
    background: var(--snow);
    color: var(--void);
  }
  button:disabled {
    opacity: 0.4;
    cursor: default;
  }
</style>
