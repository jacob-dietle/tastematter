<script lang="ts">
  import { onMount } from 'svelte';
  import { createGitStore } from '$lib/stores/git.svelte';
  import GitStatusBadge from './GitStatusBadge.svelte';
  import GitFileList from './GitFileList.svelte';
  import GitActions from './GitActions.svelte';
  import LoadingSpinner from './LoadingSpinner.svelte';
  import ErrorDisplay from './ErrorDisplay.svelte';

  let { initialFetch = true }: { initialFetch?: boolean } = $props();

  const gitStore = createGitStore();

  onMount(() => {
    if (initialFetch) {
      gitStore.fetchStatus();
    }
  });

  // Auto-dismiss operation result after 5 seconds
  $effect(() => {
    if (gitStore.lastOperation) {
      const timer = setTimeout(() => {
        gitStore.clearLastOperation();
      }, 5000);
      return () => clearTimeout(timer);
    }
  });
</script>

<div class="git-panel" data-testid="git-panel">
  <div class="header">
    <h3 class="title">Git Status</h3>
    {#if gitStore.data}
      <GitStatusBadge
        ahead={gitStore.data.ahead}
        behind={gitStore.data.behind}
      />
    {/if}
    <button
      class="refresh-button"
      onclick={() => gitStore.fetchStatus()}
      disabled={gitStore.loading}
      title="Refresh status"
    >
      ⟳
    </button>
  </div>

  {#if gitStore.loading && !gitStore.data}
    <LoadingSpinner />
  {:else if gitStore.error}
    <ErrorDisplay error={gitStore.error} />
  {:else if gitStore.data}
    <div class="branch" data-testid="git-branch">
      <span class="branch-icon">⎇</span>
      {gitStore.data.branch}
    </div>

    {#if gitStore.data.has_conflicts}
      <div class="conflict-warning" data-testid="conflict-warning">
        ⚠ Merge conflicts detected
      </div>
    {/if}

    <div class="file-lists">
      <GitFileList
        title="Staged"
        files={gitStore.data.staged}
        icon="✓"
      />
      <GitFileList
        title="Modified"
        files={gitStore.data.modified}
        icon="M"
      />
      <GitFileList
        title="Untracked"
        files={gitStore.data.untracked}
        icon="?"
      />
    </div>

    {#if !gitStore.hasChanges && gitStore.data.ahead === 0 && gitStore.data.behind === 0}
      <div class="clean-state" data-testid="clean-state">
        ✓ Working tree clean, in sync with remote
      </div>
    {/if}

    <GitActions
      canPull={gitStore.canPull}
      canPush={gitStore.canPush}
      isPulling={gitStore.isPulling}
      isPushing={gitStore.isPushing}
      onPull={() => gitStore.pull()}
      onPush={() => gitStore.push()}
    />

    {#if gitStore.lastOperation}
      <div
        class="operation-result"
        class:success={gitStore.lastOperation.success}
        class:error={!gitStore.lastOperation.success}
        data-testid="operation-result"
      >
        {gitStore.lastOperation.message}
        {#if gitStore.lastOperation.error}
          <details>
            <summary>Details</summary>
            <pre>{gitStore.lastOperation.error}</pre>
          </details>
        {/if}
      </div>
    {/if}
  {:else}
    <div class="empty-state">
      Click refresh to load git status.
    </div>
  {/if}
</div>

<style>
  .git-panel {
    border: 1px solid var(--border-color);
    border-radius: var(--radius-md);
    padding: 1rem;
    background: var(--bg-panel);
  }

  .header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 1rem;
  }

  .title {
    margin: 0;
    font-size: 1.1em;
  }

  .refresh-button {
    margin-left: auto;
    padding: var(--button-padding-sm);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-sm);
    background: transparent;
    cursor: pointer;
  }

  .refresh-button:hover:not(:disabled) {
    background: var(--bg-hover);
  }

  .refresh-button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .branch {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-family: monospace;
    font-size: 0.9em;
    margin-bottom: 1rem;
    padding: 0.5rem;
    background: var(--bg-secondary);
    border-radius: var(--radius-sm);
  }

  .conflict-warning {
    padding: 0.5rem;
    background: var(--bg-warning);
    border: 1px solid var(--border-warning);
    border-radius: var(--radius-sm);
    margin-bottom: 1rem;
  }

  .file-lists {
    margin-bottom: 1rem;
  }

  .clean-state {
    padding: 1rem;
    text-align: center;
    color: var(--color-synced);
    margin-bottom: 1rem;
  }

  .empty-state {
    padding: 1rem;
    text-align: center;
    color: var(--text-muted);
  }

  .operation-result {
    padding: 0.5rem;
    border-radius: var(--radius-sm);
    margin-top: 1rem;
  }

  .operation-result.success {
    background: var(--bg-success);
    border: 1px solid var(--border-success);
  }

  .operation-result.error {
    background: var(--bg-error);
    border: 1px solid var(--border-error);
  }

  .operation-result pre {
    margin: 0.5rem 0 0;
    padding: 0.5rem;
    background: rgba(0, 0, 0, 0.05);
    border-radius: var(--radius-sm);
    font-size: 0.8em;
    overflow-x: auto;
  }
</style>
