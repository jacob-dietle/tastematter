<!-- src/lib/components/ChainCard.svelte -->
<!-- Chain card for WorkstreamView - displays chain with expandable sessions -->
<script lang="ts">
  import type { ChainData, SessionData, CommandError } from '$lib/types';
  import ChainBadge from './ChainBadge.svelte';
  import SessionCard from './SessionCard.svelte';
  import LoadingSpinner from './LoadingSpinner.svelte';
  import ErrorDisplay from './ErrorDisplay.svelte';

  let {
    chain,
    expanded,
    loading,
    sessions,
    error,
    expandedSessions,
    onToggleExpand,
    onToggleSession,
    onFileClick,
    onRetry,
  }: {
    chain: ChainData;
    expanded: boolean;
    loading: boolean;
    sessions: SessionData[];
    error: CommandError | null;
    expandedSessions: Set<string>;
    onToggleExpand: (chainId: string) => void;
    onToggleSession: (sessionId: string) => void;
    onFileClick: (filePath: string) => void;
    onRetry: (chainId: string) => void;
  } = $props();

  function formatTimeRange(start: string, end: string): string {
    const startDate = new Date(start);
    const endDate = new Date(end);
    const format = (d: Date) => `${(d.getMonth() + 1).toString().padStart(2, '0')}/${d.getDate().toString().padStart(2, '0')}`;
    return `${format(startDate)} - ${format(endDate)}`;
  }

  function colorScale(count: number): string {
    const maxCount = Math.max(...sessions.map(s => s.total_accesses), 1);
    const intensity = Math.round((count / maxCount) * 100);
    return `rgb(${100 - intensity}, ${100 + intensity}, ${150})`;
  }

  function handleExpandClick() {
    onToggleExpand(chain.chain_id);
  }

  function handleRetryClick() {
    onRetry(chain.chain_id);
  }
</script>

<div class="chain-card" class:expanded data-testid="chain-card">
  <div class="card-header">
    <div class="chain-info">
      <ChainBadge chainId={chain.chain_id} />
      <div class="chain-stats">
        <span class="stat">{chain.session_count} sessions</span>
        <span class="stat">{chain.file_count} files</span>
      </div>
      {#if chain.time_range}
        <div class="chain-time">
          {formatTimeRange(chain.time_range.start, chain.time_range.end)}
        </div>
      {/if}
    </div>
    <button
      class="expand-button"
      data-testid="expand-button"
      onclick={handleExpandClick}
      title={expanded ? 'Collapse' : 'Expand'}
    >
      {expanded ? '▲' : '▼'}
    </button>
  </div>

  {#if expanded}
    <div class="card-content">
      {#if loading}
        <LoadingSpinner />
      {:else if error}
        <div class="error-section">
          <ErrorDisplay error={error} />
          <button class="retry-button" onclick={handleRetryClick}>Retry</button>
        </div>
      {:else if sessions.length > 0}
        <div class="sessions-list">
          {#each sessions as session (session.session_id)}
            <SessionCard
              {session}
              expanded={expandedSessions.has(session.session_id)}
              onToggleExpand={onToggleSession}
              onFileClick={onFileClick}
              {colorScale}
            />
          {/each}
        </div>
      {:else}
        <div class="empty-state">
          No sessions found for this chain.
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .chain-card {
    border: 1px solid var(--border-color, #e1e4e8);
    border-radius: 8px;
    background: var(--bg-card, white);
    overflow: hidden;
    transition: all 0.15s ease;
  }

  .chain-card:hover {
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
  }

  .chain-card.expanded {
    border-color: var(--color-primary, #0366d6);
  }

  .card-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    padding: 12px 16px;
    cursor: pointer;
  }

  .card-header:hover {
    background: var(--bg-hover, #f6f8fa);
  }

  .chain-info {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .chain-stats {
    display: flex;
    gap: 12px;
  }

  .stat {
    font-size: 0.8em;
    color: var(--text-muted, #6a737d);
  }

  .chain-time {
    font-size: 0.75em;
    color: var(--text-muted, #6a737d);
    font-family: monospace;
  }

  .expand-button {
    padding: 4px 8px;
    border: 1px solid var(--border-color, #e1e4e8);
    border-radius: 4px;
    background: transparent;
    cursor: pointer;
    font-size: 0.9em;
    color: var(--text-muted, #6a737d);
  }

  .expand-button:hover {
    background: var(--bg-hover, #f6f8fa);
  }

  .card-content {
    border-top: 1px solid var(--border-color, #e1e4e8);
    padding: 12px 16px;
  }

  .sessions-list {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .error-section {
    display: flex;
    flex-direction: column;
    gap: 8px;
    align-items: flex-start;
  }

  .retry-button {
    padding: 4px 12px;
    border: 1px solid var(--border-color, #e1e4e8);
    border-radius: 4px;
    background: var(--bg-button, #f6f8fa);
    cursor: pointer;
    font-size: 0.85em;
  }

  .retry-button:hover {
    background: var(--bg-button-hover, #e1e4e8);
  }

  .empty-state {
    padding: 1rem;
    text-align: center;
    color: var(--text-muted, #6a737d);
    font-size: 0.9em;
  }
</style>
