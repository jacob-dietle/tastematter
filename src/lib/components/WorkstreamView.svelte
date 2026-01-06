<!-- src/lib/components/WorkstreamView.svelte -->
<!-- Workstream view showing Chain -> Session -> Files hierarchy -->
<script lang="ts">
  import { getAppContext } from '$lib/stores/context.svelte';
  import { createWorkstreamStore } from '$lib/stores/workstream.svelte';
  import ChainCard from './ChainCard.svelte';
  import LoadingSpinner from './LoadingSpinner.svelte';
  import ErrorDisplay from './ErrorDisplay.svelte';

  const ctx = getAppContext();
  const workstreamStore = createWorkstreamStore(ctx);

  function handleFileClick(filePath: string) {
    console.log(`File clicked: ${filePath}`);
    // TODO: Could navigate to file in Files view or show details
  }
</script>

<div class="workstream-view" data-testid="workstream-view">
  <div class="header">
    <div class="title-section">
      <h3>Workstreams</h3>
      <span class="summary">
        {ctx.totalChains} chains | {workstreamStore.totalLoadedSessions} sessions loaded
      </span>
    </div>
    <div class="controls">
      <button
        class="control-button"
        onclick={() => workstreamStore.expandAllChains()}
      >
        Expand All
      </button>
      <button
        class="control-button"
        onclick={() => workstreamStore.collapseAllChains()}
      >
        Collapse All
      </button>
    </div>
  </div>

  <div class="content">
    {#if ctx.chainsLoading && ctx.chains.length === 0}
      <div class="loading-container">
        <LoadingSpinner />
      </div>
    {:else if ctx.chainsError}
      <ErrorDisplay
        error={ctx.chainsError}
        onretry={() => ctx.refreshChains()}
      />
    {:else if ctx.chains.length > 0}
      <div class="chain-list">
        {#each ctx.chains as chain (chain.chain_id)}
          <ChainCard
            {chain}
            expanded={workstreamStore.isChainExpanded(chain.chain_id)}
            loading={workstreamStore.isChainLoading(chain.chain_id)}
            sessions={workstreamStore.getSessionsForChain(chain.chain_id)}
            error={workstreamStore.getChainError(chain.chain_id)}
            expandedSessions={workstreamStore.expandedSessions}
            onToggleExpand={workstreamStore.toggleChainExpanded}
            onToggleSession={workstreamStore.toggleSessionExpanded}
            onFileClick={handleFileClick}
            onRetry={workstreamStore.retryLoadSessions}
          />
        {/each}
      </div>
    {:else}
      <div class="empty-state">
        <div class="empty-icon">📂</div>
        <p>No chains found</p>
        <span class="empty-hint">Chains will appear here as you work with Claude Code</span>
      </div>
    {/if}
  </div>
</div>

<style>
  .workstream-view {
    padding: 1.5rem;
    background: var(--bg-card);
    border-radius: var(--radius-lg, 12px);
    border: 1px solid var(--border-color);
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1.5rem;
    padding-bottom: 1rem;
    border-bottom: 1px solid var(--border-color);
  }

  .title-section {
    display: flex;
    align-items: baseline;
    gap: 1rem;
  }

  .title-section h3 {
    margin: 0;
    font-size: 1.125rem;
    font-weight: 600;
    color: var(--text-heading);
  }

  .summary {
    font-size: 0.8rem;
    color: var(--text-muted);
    background: var(--bg-secondary);
    padding: 2px 8px;
    border-radius: var(--radius-md, 8px);
  }

  .controls {
    display: flex;
    gap: 0.5rem;
  }

  .control-button {
    padding: 0.35rem 0.75rem;
    border: 1px solid var(--border-color);
    border-radius: var(--radius-sm, 4px);
    background: transparent;
    color: var(--text-primary);
    font-size: 0.8rem;
    cursor: pointer;
    transition: background 0.15s ease;
  }

  .control-button:hover {
    background: var(--bg-hover);
  }

  .content {
    min-height: 200px;
  }

  .loading-container {
    display: flex;
    justify-content: center;
    padding: 3rem;
  }

  .chain-list {
    display: flex;
    flex-direction: column;
    gap: 12px;
    max-height: 70vh;
    overflow-y: auto;
    padding-right: 4px;
  }

  .chain-list::-webkit-scrollbar {
    width: 6px;
  }

  .chain-list::-webkit-scrollbar-track {
    background: var(--bg-secondary);
    border-radius: var(--radius-sm, 4px);
  }

  .chain-list::-webkit-scrollbar-thumb {
    background: var(--border-color);
    border-radius: var(--radius-sm, 4px);
  }

  .chain-list::-webkit-scrollbar-thumb:hover {
    background: var(--text-muted);
  }

  .empty-state {
    padding: 3rem;
    text-align: center;
    color: var(--text-muted);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.5rem;
  }

  .empty-icon {
    font-size: 2rem;
    opacity: 0.5;
  }

  .empty-state p {
    margin: 0;
    font-weight: 500;
  }

  .empty-hint {
    font-size: 0.8rem;
    opacity: 0.7;
  }
</style>
