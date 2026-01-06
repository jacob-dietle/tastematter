<!-- src/lib/components/WorkstreamView.svelte -->
<!-- Sessions view - single bulk fetch with client-side filtering -->
<script lang="ts">
  import { getAppContext } from '$lib/stores/context.svelte';
  import { querySessions } from '$lib/api/tauri';
  import type { SessionData, CommandError } from '$lib/types';
  import SessionCard from './SessionCard.svelte';
  import LoadingSpinner from './LoadingSpinner.svelte';
  import ErrorDisplay from './ErrorDisplay.svelte';

  const ctx = getAppContext();

  // Local state (single fetch, not per-chain)
  let sessions = $state<SessionData[]>([]);
  let loading = $state(false);
  let error = $state<CommandError | null>(null);
  let expandedSessions = $state<Set<string>>(new Set());

  // Pre-compute max access count once (O(n) instead of O(n²))
  const maxAccessCount = $derived(Math.max(...sessions.map(s => s.total_accesses), 1));

  // Single bulk fetch
  async function fetchSessions() {
    loading = true;
    error = null;
    try {
      const result = await querySessions({ time: ctx.timeRange, limit: 100 });
      sessions = result.sessions;
    } catch (e) {
      error = e as CommandError;
    } finally {
      loading = false;
    }
  }

  // Fetch on mount and when timeRange changes
  $effect(() => {
    const _ = ctx.timeRange; // dependency
    fetchSessions();
  });

  // Client-side filtering by chain
  function getFilteredSessions(): SessionData[] {
    if (!ctx.selectedChain) return sessions;
    return sessions.filter(s => s.chain_id === ctx.selectedChain);
  }

  // Compute summary stats from filtered sessions
  function getSummary() {
    const filtered = getFilteredSessions();
    const totalFiles = filtered.reduce((sum, s) => sum + s.file_count, 0);
    const totalAccesses = filtered.reduce((sum, s) => sum + s.total_accesses, 0);
    const uniqueChains = new Set(filtered.map(s => s.chain_id)).size;
    return {
      total_sessions: filtered.length,
      total_files: totalFiles,
      total_accesses: totalAccesses,
      active_chains: uniqueChains,
    };
  }

  // Color scale for visual hierarchy (uses pre-computed maxAccessCount)
  function colorScale(count: number): string {
    const intensity = Math.round((count / maxAccessCount) * 100);
    return `rgb(${100 - intensity}, ${100 + intensity}, ${150})`;
  }

  // Session expand/collapse
  function toggleSessionExpanded(sessionId: string) {
    const newSet = new Set(expandedSessions);
    if (newSet.has(sessionId)) {
      newSet.delete(sessionId);
    } else {
      newSet.add(sessionId);
    }
    expandedSessions = newSet;
  }

  function isSessionExpanded(sessionId: string): boolean {
    return expandedSessions.has(sessionId);
  }

  function handleFileClick(filePath: string) {
    console.log(`File clicked: ${filePath}`);
  }

  function handleChainClick(chainId: string) {
    ctx.setSelectedChain(ctx.selectedChain === chainId ? null : chainId);
  }
</script>

<div class="session-view" data-testid="workstream-view">
  <div class="header">
    <h3 class="title">Sessions</h3>
    <button
      class="refresh-button"
      onclick={() => fetchSessions()}
      disabled={loading}
      title="Refresh data"
    >
      ⟳
    </button>
  </div>

  {#if ctx.selectedChain}
    <div class="filter-bar">
      <span>Filtered by chain: {ctx.selectedChain.slice(0, 8)}</span>
      <button onclick={() => ctx.setSelectedChain(null)}>Clear filter</button>
    </div>
  {/if}

  {#if loading && sessions.length === 0}
    <LoadingSpinner />
  {:else if error}
    <ErrorDisplay error={error} onretry={() => fetchSessions()} />
  {:else if sessions.length > 0}
    {@const summary = getSummary()}
    {@const filteredSessions = getFilteredSessions()}

    <div class="summary" data-testid="session-summary">
      <span>{summary.total_sessions} sessions</span>
      <span>{summary.total_files} files</span>
      <span>{summary.total_accesses} accesses</span>
      {#if summary.active_chains > 0}
        <span>{summary.active_chains} chains</span>
      {/if}
    </div>

    <div class="sessions-list">
      {#each filteredSessions as session (session.session_id)}
        <SessionCard
          {session}
          expanded={isSessionExpanded(session.session_id)}
          onToggleExpand={toggleSessionExpanded}
          onFileClick={handleFileClick}
          onChainClick={handleChainClick}
          {colorScale}
        />
      {/each}

      {#if filteredSessions.length === 0}
        <div class="empty-state">
          No sessions found for this filter.
        </div>
      {/if}
    </div>
  {:else}
    <div class="empty-state">
      <div class="empty-icon">📂</div>
      <p>No sessions found</p>
      <span class="empty-hint">Sessions will appear here as you work with Claude Code</span>
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
    font-size: 1.1em;
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

  .filter-bar button:hover {
    background: var(--bg-hover, #f6f8fa);
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
    max-height: 70vh;
    overflow-y: auto;
  }

  .sessions-list::-webkit-scrollbar {
    width: 6px;
  }

  .sessions-list::-webkit-scrollbar-track {
    background: var(--bg-secondary);
    border-radius: 4px;
  }

  .sessions-list::-webkit-scrollbar-thumb {
    background: var(--border-color);
    border-radius: 4px;
  }

  .empty-state {
    padding: 2rem;
    text-align: center;
    color: var(--text-muted, #6a737d);
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
