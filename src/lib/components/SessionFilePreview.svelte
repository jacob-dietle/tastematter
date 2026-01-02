<!-- src/lib/components/SessionFilePreview.svelte -->
<script lang="ts">
  import type { SessionFile } from '$lib/types';

  let {
    files,
    totalCount,
    onShowMore,
    colorScale
  }: {
    files: SessionFile[];
    totalCount: number;
    onShowMore: () => void;
    colorScale: (count: number) => string;
  } = $props();

  function getFileName(path: string): string {
    return path.split('/').pop() || path;
  }

  const remainingCount = $derived(totalCount - files.length);
</script>

<div class="file-preview" data-testid="file-preview">
  <div class="top-files">
    {#each files as file (file.file_path)}
      <div class="file-item" title={file.file_path}>
        <span
          class="count-dot"
          style="background: {colorScale(file.access_count)}"
        ></span>
        <span class="file-name">{getFileName(file.file_path)}</span>
        <span class="count">({file.access_count})</span>
      </div>
    {/each}
  </div>

  {#if remainingCount > 0}
    <button class="show-more" onclick={onShowMore}>
      + {remainingCount} more file{remainingCount === 1 ? '' : 's'}
    </button>
  {/if}
</div>

<style>
  .file-preview {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .top-files {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .file-item {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 0.85em;
  }

  .count-dot {
    width: 8px;
    height: 8px;
    border-radius: 2px;
    flex-shrink: 0;
  }

  .file-name {
    font-family: monospace;
    color: var(--text-primary, #24292e);
  }

  .count {
    color: var(--text-muted, #6a737d);
    font-size: 0.9em;
  }

  .show-more {
    align-self: flex-start;
    padding: 4px 8px;
    font-size: 0.8em;
    color: var(--color-link, #0366d6);
    background: none;
    border: none;
    cursor: pointer;
  }

  .show-more:hover {
    text-decoration: underline;
  }
</style>
