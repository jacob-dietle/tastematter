<script lang="ts">
  let {
    canPull,
    canPush,
    isPulling,
    isPushing,
    onPull,
    onPush
  }: {
    canPull: boolean;
    canPush: boolean;
    isPulling: boolean;
    isPushing: boolean;
    onPull: () => void;
    onPush: () => void;
  } = $props();

  let isDisabled = $derived(isPulling || isPushing);
</script>

<div class="git-actions" data-testid="git-actions">
  <button
    class="action-button pull"
    disabled={isDisabled || !canPull}
    onclick={onPull}
    title={canPull ? 'Pull changes from remote' : 'No changes to pull'}
  >
    {#if isPulling}
      <span class="spinner">⟳</span> Pulling...
    {:else}
      ↓ Pull
    {/if}
  </button>

  <button
    class="action-button push"
    disabled={isDisabled || !canPush}
    onclick={onPush}
    title={canPush ? 'Push changes to remote' : 'No changes to push'}
  >
    {#if isPushing}
      <span class="spinner">⟳</span> Pushing...
    {:else}
      ↑ Push
    {/if}
  </button>
</div>

<style>
  .git-actions {
    display: flex;
    gap: 0.5rem;
  }

  .action-button {
    padding: var(--button-padding-md);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-sm);
    background: var(--bg-button);
    cursor: pointer;
    font-size: 0.9em;
    display: flex;
    align-items: center;
    gap: 0.25rem;
  }

  .action-button:hover:not(:disabled) {
    background: var(--bg-button-hover);
  }

  .action-button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .action-button.pull:not(:disabled) {
    border-color: var(--color-behind);
    color: var(--color-behind);
  }

  .action-button.push:not(:disabled) {
    border-color: var(--color-ahead);
    color: var(--color-ahead);
  }

  .spinner {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }
</style>
