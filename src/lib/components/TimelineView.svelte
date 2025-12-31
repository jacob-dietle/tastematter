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
    <h3>File Activity Timeline</h3>
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
    <div class="loading">Loading timeline...</div>
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
        <strong>{store.hoveredCell.file}</strong>
        <span>{store.hoveredCell.date}</span>
      </div>
    {/if}
  {:else}
    <div class="empty">No timeline data available</div>
  {/if}
</div>

<style>
  .timeline-view {
    padding: 1rem;
    background: white;
    border-radius: 8px;
    border: 1px solid #e0e0e0;
  }

  .timeline-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1rem;
  }

  .timeline-header h3 {
    margin: 0;
    font-size: 1rem;
    font-weight: 600;
    color: #1a1a2e;
  }

  .controls {
    display: flex;
    align-items: center;
    gap: 1rem;
  }

  .timeline-content {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .timeline-rows {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .loading,
  .error,
  .empty {
    padding: 2rem;
    text-align: center;
    color: #666;
  }

  .error {
    color: #d32f2f;
  }

  .tooltip {
    position: fixed;
    bottom: 1rem;
    left: 50%;
    transform: translateX(-50%);
    background: #1a1a2e;
    color: white;
    padding: 0.5rem 1rem;
    border-radius: 4px;
    font-size: 0.75rem;
    display: flex;
    gap: 1rem;
    z-index: 100;
  }
</style>
