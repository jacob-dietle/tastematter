<script lang="ts">
  import type { ViewMode } from '$lib/types';

  interface Props {
    mode: ViewMode;
  }

  let { mode = $bindable<ViewMode>('heatmap') }: Props = $props();
</script>

<div class="view-mode-toggle" role="tablist">
  <button
    role="tab"
    aria-selected={mode === 'table'}
    class:active={mode === 'table'}
    onclick={() => mode = 'table'}
  >
    Table
  </button>
  <button
    role="tab"
    aria-selected={mode === 'heatmap'}
    class:active={mode === 'heatmap'}
    onclick={() => mode = 'heatmap'}
  >
    Heat Map
  </button>
</div>

<style>
  .view-mode-toggle {
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
