<!-- src/lib/components/SessionCard.svelte -->
<script lang="ts">
  import type { SessionData } from '$lib/types';
  import ChainBadge from './ChainBadge.svelte';
  import SessionFilePreview from './SessionFilePreview.svelte';
  import SessionFileTree from './SessionFileTree.svelte';

  let {
    session,
    expanded,
    onToggleExpand,
    onFileClick,
    onChainClick,
    colorScale
  }: {
    session: SessionData;
    expanded: boolean;
    onToggleExpand: (sessionId: string) => void;
    onFileClick: (filePath: string) => void;
    onChainClick?: (chainId: string) => void;
    colorScale: (count: number) => string;
  } = $props();

  function formatDate(iso: string): string {
    const date = new Date(iso);
    return date.toLocaleDateString('en-US', {
      month: 'short',
      day: 'numeric',
      hour: 'numeric',
      minute: '2-digit'
    });
  }

  function formatDuration(seconds: number | null): string {
    if (!seconds) return '';
    if (seconds < 60) return `${seconds}s`;
    if (seconds < 3600) return `${Math.round(seconds / 60)}min`;
    const hours = Math.floor(seconds / 3600);
    const mins = Math.round((seconds % 3600) / 60);
    return `${hours}h ${mins}m`;
  }

  function truncateSessionId(id: string): string {
    return id.slice(0, 8);
  }
</script>

<div class="session-card" class:expanded data-testid="session-card">
  <div class="card-header">
    <div class="session-info">
      <span class="session-id">{truncateSessionId(session.session_id)}</span>
      <ChainBadge chainId={session.chain_id} onClick={onChainClick} />
    </div>
    <div class="session-meta">
      <span class="date">{formatDate(session.started_at)}</span>
      {#if session.duration_seconds}
        <span class="duration">{formatDuration(session.duration_seconds)}</span>
      {/if}
    </div>
  </div>

  <div class="card-stats">
    <span>{session.file_count} file{session.file_count === 1 ? '' : 's'}</span>
    <span class="dot">·</span>
    <span>{session.total_accesses} access{session.total_accesses === 1 ? '' : 'es'}</span>
  </div>

  <div class="card-content">
    {#if expanded}
      <SessionFileTree
        files={session.files}
        {colorScale}
        {onFileClick}
      />
      <button class="collapse-btn" onclick={() => onToggleExpand(session.session_id)}>
        ▲ Collapse
      </button>
    {:else}
      <SessionFilePreview
        files={session.top_files}
        totalCount={session.file_count}
        onShowMore={() => onToggleExpand(session.session_id)}
        {colorScale}
      />
    {/if}
  </div>
</div>

<style>
  .session-card {
    border: 1px solid var(--border-color, #e1e4e8);
    border-radius: 8px;
    padding: 12px 16px;
    background: var(--bg-card, white);
    transition: box-shadow 0.15s ease;
  }

  .session-card:hover {
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
  }

  .session-card.expanded {
    border-color: var(--color-primary, #0366d6);
  }

  .card-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: 8px;
  }

  .session-info {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .session-id {
    font-family: monospace;
    font-weight: 600;
    color: var(--text-primary, #24292e);
  }

  .session-meta {
    display: flex;
    gap: 8px;
    font-size: 0.85em;
    color: var(--text-muted, #6a737d);
  }

  .card-stats {
    font-size: 0.85em;
    color: var(--text-muted, #6a737d);
    margin-bottom: 12px;
  }

  .dot {
    margin: 0 4px;
  }

  .card-content {
    border-top: 1px solid var(--border-light, #f0f0f0);
    padding-top: 12px;
  }

  .collapse-btn {
    margin-top: 12px;
    padding: 4px 8px;
    font-size: 0.8em;
    color: var(--text-muted, #6a737d);
    background: none;
    border: none;
    cursor: pointer;
  }

  .collapse-btn:hover {
    color: var(--text-primary, #24292e);
  }
</style>
