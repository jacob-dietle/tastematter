<script lang="ts">
  import type { FileResult, DirectoryResult, Granularity } from '$lib/types';
  import { aggregateByDirectory } from '$lib/utils/aggregation';
  import HeatMapRow from './HeatMapRow.svelte';

  interface Props {
    results: FileResult[];
    granularity: Granularity;
    onDirectoryClick?: (dir: DirectoryResult) => void;
  }

  let {
    results,
    granularity,
    onDirectoryClick
  }: Props = $props();

  // Drill-down state
  let currentDirectory = $state<DirectoryResult | null>(null);

  // Derived data based on granularity and drill-down
  let displayData = $derived.by(() => {
    if (currentDirectory) {
      // Showing files within a directory
      return {
        type: 'files' as const,
        items: currentDirectory.files,
        maxCount: Math.max(...currentDirectory.files.map(f => f.access_count), 0)
      };
    }

    if (granularity === 'directory') {
      const dirs = aggregateByDirectory(results);
      return {
        type: 'directories' as const,
        items: dirs,
        maxCount: Math.max(...dirs.map(d => d.total_access_count), 0)
      };
    }

    return {
      type: 'files' as const,
      items: results,
      maxCount: Math.max(...results.map(f => f.access_count), 0)
    };
  });

  function handleDirectoryClick(dir: DirectoryResult) {
    currentDirectory = dir;
    onDirectoryClick?.(dir);
  }

  function handleBackClick() {
    currentDirectory = null;
  }
</script>

<div class="heat-map" data-testid="heat-map">
  {#if currentDirectory}
    <button class="back-button" onclick={handleBackClick}>
      ← Back to directories
    </button>
    <h3 class="current-directory">{currentDirectory.directory_path}</h3>
  {/if}

  <div class="heat-map-rows">
    {#if displayData.type === 'directories'}
      {#each displayData.items as dir (dir.directory_path)}
        <HeatMapRow
          label={dir.directory_path}
          accessCount={dir.total_access_count}
          maxAccessCount={displayData.maxCount}
          lastAccess={dir.last_access}
          isDirectory={true}
          onclick={() => handleDirectoryClick(dir)}
        />
      {/each}
    {:else}
      {#each displayData.items as file (file.file_path)}
        <HeatMapRow
          label={file.file_path}
          accessCount={file.access_count}
          maxAccessCount={displayData.maxCount}
          lastAccess={file.last_access}
          isDirectory={false}
        />
      {/each}
    {/if}
  </div>

  {#if displayData.items.length === 0}
    <div class="empty-state" data-testid="empty-state">
      No files found for the selected time range.
    </div>
  {/if}
</div>

<style>
  .heat-map {
    border: 1px solid var(--border-color);
    border-radius: var(--radius-md);
    overflow: hidden;
  }

  .back-button {
    padding: var(--button-padding-md);
    background: var(--bg-secondary);
    border: none;
    border-bottom: 1px solid var(--border-color);
    cursor: pointer;
    width: 100%;
    text-align: left;
    font-size: 0.875rem;
  }

  .back-button:hover {
    background: var(--bg-hover);
  }

  .current-directory {
    padding: 0.5rem 1rem;
    margin: 0;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border-color);
    font-size: 0.875rem;
    font-family: monospace;
  }

  .heat-map-rows {
    max-height: 60vh;
    overflow-y: auto;
  }

  .empty-state {
    padding: 2rem;
    text-align: center;
    color: var(--text-muted);
  }
</style>
