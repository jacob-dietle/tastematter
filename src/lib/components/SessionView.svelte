<!-- src/lib/components/SessionView.svelte -->
<script lang="ts">
  import { onMount } from 'svelte';
  import { createSessionStore } from '$lib/stores/session.svelte';
  import TimeRangeToggle from './TimeRangeToggle.svelte';
  import SessionCard from './SessionCard.svelte';
  import LoadingSpinner from './LoadingSpinner.svelte';
  import ErrorDisplay from './ErrorDisplay.svelte';

  let {
    initialFetch = true,
    defaultRange = '7d'
  }: {
    initialFetch?: boolean;
    defaultRange?: '7d' | '14d' | '30d';
  } = $props();

  const sessionStore = createSessionStore();

  onMount(() => {
    if (initialFetch) {
      sessionStore.setRange(defaultRange);
    }
  });

  function handleFileClick(filePath: string) {
    console.log(`File clicked: ${filePath}`);
  }

  function handleChainClick(chainId: string) {
    sessionStore.setChainFilter(
      sessionStore.selectedChain === chainId ? null : chainId
    );
  }
</script>

<div class="session-view" data-testid="session-view">
  <div class="header">
    <h3 class="title">Sessions</h3>
    <TimeRangeToggle
      selected={sessionStore.selectedRange}
      options={['7d', '14d', '30d']}
      onchange={(range) => sessionStore.setRange(range)}
      disabled={sessionStore.loading}
    />
    <button
      class="refresh-button"
      onclick={() => sessionStore.fetch()}
      disabled={sessionStore.loading}
      title="Refresh data"
    >
      ⟳
    </button>
  </div>

  {#if sessionStore.selectedChain}
    <div class="filter-bar">
      <span>Filtered by chain: {sessionStore.selectedChain.slice(0, 8)}</span>
      <button onclick={() => sessionStore.setChainFilter(null)}>Clear filter</button>
    </div>
  {/if}

  {#if sessionStore.loading && !sessionStore.data}
    <LoadingSpinner />
  {:else if sessionStore.error}
    <ErrorDisplay error={sessionStore.error} />
  {:else if sessionStore.data}
    <div class="summary" data-testid="session-summary">
      <span>{sessionStore.data.summary.total_sessions} sessions</span>
      <span>{sessionStore.data.summary.total_files} files</span>
      <span>{sessionStore.data.summary.total_accesses} accesses</span>
      {#if sessionStore.data.summary.active_chains > 0}
        <span>{sessionStore.data.summary.active_chains} chains</span>
      {/if}
    </div>

    <div class="sessions-list">
      {#each sessionStore.filteredSessions as session (session.session_id)}
        <SessionCard
          {session}
          expanded={sessionStore.isExpanded(session.session_id)}
          onToggleExpand={sessionStore.toggleSessionExpanded}
          onFileClick={handleFileClick}
          onChainClick={handleChainClick}
          colorScale={sessionStore.colorScale}
        />
      {/each}

      {#if sessionStore.filteredSessions.length === 0}
        <div class="empty-state">
          No sessions found for this time range.
        </div>
      {/if}
    </div>
  {:else}
    <div class="empty-state">
      Select a time range to view sessions.
    </div>
  {/if}
</div>

<style>
  .session-view {
    border: 1px solid var(--border-color, #e1e4e8);
    border-radius: 8px;
    padding: 1rem;
    background: var(--bg-panel, white);
  }

  .header {
    display: flex;
    align-items: center;
    gap: 1rem;
    margin-bottom: 1rem;
  }

  .title {
    margin: 0;
    font-size: 1.1em;
  }

  .refresh-button {
    margin-left: auto;
    padding: 0.25rem 0.5rem;
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

  .filter-bar {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 0.5rem;
    margin-bottom: 1rem;
    background: var(--bg-secondary, #f6f8fa);
    border-radius: 4px;
    font-size: 0.85em;
  }

  .filter-bar button {
    padding: 2px 8px;
    font-size: 0.9em;
    border: 1px solid var(--border-color);
    border-radius: 4px;
    background: white;
    cursor: pointer;
  }

  .summary {
    display: flex;
    gap: 1rem;
    font-size: 0.9em;
    color: var(--text-muted, #6a737d);
    margin-bottom: 1rem;
    padding: 0.5rem;
    background: var(--bg-secondary, #f6f8fa);
    border-radius: 4px;
  }

  .sessions-list {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .empty-state {
    padding: 2rem;
    text-align: center;
    color: var(--text-muted, #6a737d);
  }
</style>
