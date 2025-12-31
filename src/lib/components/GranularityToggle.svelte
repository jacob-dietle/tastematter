<script lang="ts">
  import type { Granularity } from '$lib/types';

  interface Props {
    granularity: Granularity;
  }

  let { granularity = $bindable<Granularity>('directory') }: Props = $props();
</script>

<div class="granularity-toggle" role="tablist">
  <button
    role="tab"
    aria-selected={granularity === 'directory'}
    class:active={granularity === 'directory'}
    onclick={() => granularity = 'directory'}
  >
    Directories
  </button>
  <button
    role="tab"
    aria-selected={granularity === 'file'}
    class:active={granularity === 'file'}
    onclick={() => granularity = 'file'}
  >
    Files
  </button>
</div>

<style>
  .granularity-toggle {
    display: flex;
    gap: 0.5rem;
  }

  button {
    padding: var(--button-padding-md);
    border: 1px solid var(--border-color);
    background: var(--bg-secondary);
    cursor: pointer;
    border-radius: var(--radius-sm);
    font-size: 0.875rem;
    transition: all 0.15s ease;
  }

  button:hover:not(.active) {
    background: var(--bg-hover);
  }

  button.active {
    background: var(--bg-active);
    color: var(--text-inverse);
    border-color: var(--bg-active);
  }
</style>
