<script lang="ts">
  import type { QueryResult, ViewMode, Granularity } from '$lib/types';
  import ViewModeToggle from './ViewModeToggle.svelte';
  import GranularityToggle from './GranularityToggle.svelte';
  import TableView from './TableView.svelte';
  import HeatMap from './HeatMap.svelte';

  interface Props {
    data: QueryResult;
  }

  let { data }: Props = $props();

  let viewMode = $state<ViewMode>('heatmap');
  let granularity = $state<Granularity>('directory');
</script>

<div class="query-results" data-testid="query-results">
  <div class="controls">
    <ViewModeToggle bind:mode={viewMode} />
    {#if viewMode === 'heatmap'}
      <GranularityToggle bind:granularity />
    {/if}
  </div>

  <p class="count" data-testid="result-count">
    {data.result_count} files
  </p>

  {#if viewMode === 'table'}
    <TableView results={data.results} />
  {:else}
    <HeatMap results={data.results} {granularity} />
  {/if}
</div>

<style>
  .query-results {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    width: 100%;
  }

  .controls {
    display: flex;
    gap: 1rem;
    flex-wrap: wrap;
  }

  .count {
    font-size: 0.875rem;
    color: #666;
    margin: 0;
  }
</style>
