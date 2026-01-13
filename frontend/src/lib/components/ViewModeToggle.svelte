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
