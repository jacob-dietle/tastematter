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
    padding: 0.5rem 1rem;
    border: 1px solid var(--border-color, #ccc);
    background: var(--bg-secondary, #f5f5f5);
    cursor: pointer;
    border-radius: 4px;
    font-size: 0.875rem;
    transition: all 0.15s ease;
  }

  button:hover:not(.active) {
    background: var(--bg-hover, #e8e8e8);
  }

  button.active {
    background: var(--bg-active, #1a1a2e);
    color: var(--text-inverse, white);
    border-color: var(--bg-active, #1a1a2e);
  }
</style>
