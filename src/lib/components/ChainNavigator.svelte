<!-- src/lib/components/ChainNavigator.svelte -->
<script lang="ts">
  import { onMount } from 'svelte';
  import { createChainStore } from '$lib/stores/chain.svelte';
  import ChainBadge from './ChainBadge.svelte';
  import LoadingSpinner from './LoadingSpinner.svelte';
  import ErrorDisplay from './ErrorDisplay.svelte';

  let {
    onSelect,
    initialFetch = true,
    limit = 20
  }: {
    onSelect?: (chainId: string | null) => void;
    initialFetch?: boolean;
    limit?: number;
  } = $props();

  const chainStore = createChainStore();

  onMount(() => {
    if (initialFetch) {
      chainStore.fetch(limit);
    }
  });

  function handleClick(chainId: string) {
    chainStore.toggleChain(chainId);
    onSelect?.(chainStore.selectedChain);
  }

  function formatTimeRange(start: string, end: string): string {
    const startDate = new Date(start);
    const endDate = new Date(end);
    const format = (d: Date) => `${(d.getMonth() + 1).toString().padStart(2, '0')}/${d.getDate().toString().padStart(2, '0')}`;
    return `${format(startDate)} - ${format(endDate)}`;
  }
</script>

<div class="chain-navigator" data-testid="chain-navigator">
  <div class="header">
    <h4 class="title">Chains</h4>
    <span class="count">{chainStore.totalChains} total</span>
    <button
      class="refresh-button"
      onclick={() => chainStore.fetch(limit)}
      disabled={chainStore.loading}
      title="Refresh chains"
    >
      ⟳
    </button>
  </div>

  {#if chainStore.loading && !chainStore.data}
    <LoadingSpinner />
  {:else if chainStore.error}
    <ErrorDisplay error={chainStore.error} />
  {:else if chainStore.chains.length > 0}
    <div class="chain-list">
      {#each chainStore.chains as chain (chain.chain_id)}
        <button
          class="chain-item"
          class:selected={chainStore.selectedChain === chain.chain_id}
          onclick={() => handleClick(chain.chain_id)}
          data-testid="chain-item"
        >
          <div class="chain-header">
            <ChainBadge chainId={chain.chain_id} />
          </div>
          <div class="chain-stats">
            <span class="stat">{chain.session_count} sessions</span>
            <span class="stat">{chain.file_count} files</span>
          </div>
          {#if chain.time_range}
            <div class="chain-time">
              {formatTimeRange(chain.time_range.start, chain.time_range.end)}
            </div>
          {/if}
        </button>
      {/each}
    </div>
  {:else}
    <div class="empty-state">
      No chains found
    </div>
  {/if}
</div>

<style>
  .chain-navigator {
    border: 1px solid var(--border-color, #e1e4e8);
    border-radius: 8px;
    padding: 0.75rem;
    background: var(--bg-panel, white);
  }

  .header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.75rem;
  }

  .title {
    margin: 0;
    font-size: 1em;
  }

  .count {
    font-size: 0.8em;
    color: var(--text-muted, #6a737d);
  }

  .refresh-button {
    margin-left: auto;
    padding: 0.15rem 0.35rem;
    font-size: 0.9em;
    border: 1px solid var(--border-color, #e1e4e8);
    border-radius: 4px;
    background: transparent;
    cursor: pointer;
  }

  .refresh-button:hover:not(:disabled) {
    background: var(--bg-hover, #f6f8fa);
  }

  .refresh-button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .chain-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
    max-height: 300px;
    overflow-y: auto;
  }

  .chain-item {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 8px;
    border: 1px solid var(--border-color, #e1e4e8);
    border-radius: 6px;
    background: transparent;
    cursor: pointer;
    text-align: left;
    transition: all 0.15s ease;
  }

  .chain-item:hover {
    background: var(--bg-hover, #f6f8fa);
  }

  .chain-item.selected {
    border-color: var(--color-primary, #0366d6);
    background: var(--bg-selected, #f1f8ff);
  }

  .chain-header {
    display: flex;
    align-items: center;
  }

  .chain-stats {
    display: flex;
    gap: 0.75rem;
  }

  .stat {
    font-size: 0.75em;
    color: var(--text-muted, #6a737d);
  }

  .chain-time {
    font-size: 0.7em;
    color: var(--text-muted, #6a737d);
    font-family: monospace;
  }

  .empty-state {
    padding: 1rem;
    text-align: center;
    color: var(--text-muted, #6a737d);
    font-size: 0.9em;
  }
</style>
