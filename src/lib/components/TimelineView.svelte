<script lang="ts">
  import { onMount } from 'svelte';
  import { createTimelineStore } from '$lib/stores/timeline.svelte';
  import TimeRangeToggle from './TimeRangeToggle.svelte';
  import TimelineAxis from './TimelineAxis.svelte';
  import TimelineRow from './TimelineRow.svelte';
  import TimelineLegend from './TimelineLegend.svelte';

  const store = createTimelineStore();

  // Extract dates from buckets for row rendering
  $effect(() => {
    if (store.data?.buckets) {
      dates = store.data.buckets.map(b => b.date);
    }
  });

  let dates = $state<string[]>([]);

  function handleRangeChange(range: string) {
    store.setRange(range as '7d' | '14d' | '30d');
  }

  onMount(() => {
    store.fetch();
  });
</script>

<div class="timeline-view">
  <div class="timeline-header">
    <div class="title-section">
      <h3>File Activity</h3>
      {#if store.data}
        <span class="file-count">{store.data.files.length} files</span>
      {/if}
    </div>
    <div class="controls">
      <TimeRangeToggle
        selected={store.selectedRange}
        onchange={handleRangeChange}
        disabled={store.loading}
      />
      <TimelineLegend />
    </div>
  </div>

  {#if store.loading}
    <div class="loading">
      <div class="loading-spinner"></div>
      <span>Loading timeline...</span>
    </div>
  {:else if store.error}
    <div class="error">
      <p>Error loading timeline: {store.error.message}</p>
    </div>
  {:else if store.data}
    <div class="timeline-content">
      <TimelineAxis buckets={store.data.buckets} />
      <div class="timeline-rows">
        {#each store.data.files as file}
          <TimelineRow
            filePath={file.file_path}
            {dates}
            buckets={file.buckets}
            maxCount={store.maxAccessCount}
            onHover={(f, d) => store.setHoveredCell(f, d)}
            onLeave={() => store.clearHover()}
          />
        {/each}
      </div>
    </div>

    {#if store.hoveredCell}
      <div class="tooltip">
        <div class="tooltip-file">{store.hoveredCell.file}</div>
        <div class="tooltip-meta">
          <span class="tooltip-date">{store.hoveredCell.date}</span>
        </div>
      </div>
    {/if}
  {:else}
    <div class="empty">
      <div class="empty-icon">📊</div>
      <p>No timeline data available</p>
      <span class="empty-hint">Select a time range to view file activity</span>
    </div>
  {/if}
</div>

<style>
  .timeline-view {
    padding: 1.5rem;
    background: var(--bg-card);
    border-radius: var(--radius-lg);
    border: 1px solid var(--border-color);
    position: relative;
  }

  .timeline-header {
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
    gap: 0.75rem;
  }

  .timeline-header h3 {
    margin: 0;
    font-size: 1.125rem;
    font-weight: 600;
    color: var(--text-heading);
  }

  .file-count {
    font-size: 0.8rem;
    color: var(--text-muted);
    background: var(--bg-secondary);
    padding: 2px 8px;
    border-radius: var(--radius-md);
  }

  .controls {
    display: flex;
    align-items: center;
    gap: 1rem;
  }

  .timeline-content {
    display: flex;
    flex-direction: column;
    gap: 0;
  }

  .timeline-rows {
    display: flex;
    flex-direction: column;
    gap: 0;
    max-height: 60vh;
    overflow-y: auto;
    padding-right: 4px;
  }

  .timeline-rows::-webkit-scrollbar {
    width: 6px;
  }

  .timeline-rows::-webkit-scrollbar-track {
    background: var(--bg-secondary);
    border-radius: var(--radius-sm);
  }

  .timeline-rows::-webkit-scrollbar-thumb {
    background: var(--border-color);
    border-radius: var(--radius-sm);
  }

  .timeline-rows::-webkit-scrollbar-thumb:hover {
    background: var(--text-muted);
  }

  .loading {
    padding: 3rem;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1rem;
    color: var(--text-muted);
  }

  .loading-spinner {
    width: 24px;
    height: 24px;
    border: 2px solid var(--border-color);
    border-top-color: var(--focus-ring);
    border-radius: var(--radius-full);
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .error {
    padding: 2rem;
    text-align: center;
    color: var(--border-error);
    background: var(--bg-error);
    border-radius: var(--radius-md);
  }

  .empty {
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

  .empty p {
    margin: 0;
    font-weight: 500;
  }

  .empty-hint {
    font-size: 0.8rem;
    opacity: 0.7;
  }

  .tooltip {
    position: fixed;
    bottom: 1.5rem;
    left: 50%;
    transform: translateX(-50%);
    background: var(--text-heading);
    color: var(--text-inverse);
    padding: 0.75rem 1.25rem;
    border-radius: var(--radius-md);
    font-size: 0.8rem;
    z-index: var(--z-tooltip);
    box-shadow: var(--shadow-lg);
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    min-width: 200px;
  }

  .tooltip-file {
    font-family: monospace;
    font-weight: 500;
    font-size: 0.75rem;
  }

  .tooltip-meta {
    display: flex;
    gap: 1rem;
    font-size: 0.7rem;
    opacity: 0.8;
  }
</style>
