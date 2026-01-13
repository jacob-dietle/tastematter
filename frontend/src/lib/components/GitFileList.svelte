<script lang="ts">
  let {
    title,
    files,
    icon,
    collapsible = true
  }: {
    title: string;
    files: string[];
    icon: string;
    collapsible?: boolean;
  } = $props();

  let isExpanded = $state(true);

  function truncatePath(path: string, maxLength = 50): string {
    if (path.length <= maxLength) return path;
    const parts = path.split('/');
    const filename = parts.pop() || '';
    if (filename.length >= maxLength - 3) {
      return '...' + filename.slice(-(maxLength - 3));
    }
    return '.../' + filename;
  }
</script>

{#if files.length > 0}
  <div class="git-file-list" data-testid="git-file-list">
    <button
      class="header"
      class:collapsible
      onclick={() => collapsible && (isExpanded = !isExpanded)}
      aria-expanded={isExpanded}
    >
      <span class="icon">{icon}</span>
      <span class="title">{title}</span>
      <span class="count">({files.length})</span>
      {#if collapsible}
        <span class="chevron">{isExpanded ? '▼' : '▶'}</span>
      {/if}
    </button>

    {#if isExpanded}
      <ul class="file-list">
        {#each files as file (file)}
          <li class="file-item" title={file}>
            {truncatePath(file)}
          </li>
        {/each}
      </ul>
    {/if}
  </div>
{/if}

<style>
  .git-file-list {
    border: 1px solid var(--border-color);
    border-radius: var(--radius-sm);
    margin-bottom: 0.5rem;
  }

  .header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem;
    width: 100%;
    background: var(--bg-secondary);
    border: none;
    cursor: default;
    text-align: left;
    font-size: inherit;
    font-family: inherit;
  }

  .header.collapsible {
    cursor: pointer;
  }

  .header.collapsible:hover {
    background: var(--bg-hover);
  }

  .count {
    color: var(--text-muted);
  }

  .chevron {
    margin-left: auto;
    color: var(--text-muted);
  }

  .file-list {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .file-item {
    padding: 0.25rem 0.5rem 0.25rem 1.5rem;
    font-family: monospace;
    font-size: 0.85em;
    border-top: 1px solid var(--border-color);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
