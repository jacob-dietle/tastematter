<script lang="ts">
  let { ahead, behind }: { ahead: number; behind: number } = $props();
</script>

<div class="git-status-badge" data-testid="git-status-badge">
  {#if ahead > 0}
    <span class="ahead" title="{ahead} commits ahead">
      ↑{ahead}
    </span>
  {/if}
  {#if behind > 0}
    <span class="behind" title="{behind} commits behind">
      ↓{behind}
    </span>
  {/if}
  {#if ahead === 0 && behind === 0}
    <span class="synced" title="Up to date">✓</span>
  {/if}
</div>

<style>
  .git-status-badge {
    display: flex;
    gap: 0.5rem;
    font-family: monospace;
    font-size: 0.9em;
  }

  .ahead {
    color: var(--color-ahead, #22863a);
  }

  .behind {
    color: var(--color-behind, #cb2431);
  }

  .synced {
    color: var(--color-synced, #6a737d);
  }
</style>
