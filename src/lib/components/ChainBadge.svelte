<!-- src/lib/components/ChainBadge.svelte -->
<script lang="ts">
  let {
    chainId,
    onClick
  }: {
    chainId: string | null;
    onClick?: (chainId: string) => void;
  } = $props();

  // Generate consistent color from chain ID
  function getChainColor(id: string | null): string {
    if (!id) return 'var(--color-muted, #6a737d)';
    // Simple hash to color
    let hash = 0;
    for (let i = 0; i < id.length; i++) {
      hash = id.charCodeAt(i) + ((hash << 5) - hash);
    }
    const hue = hash % 360;
    return `hsl(${hue}, 40%, 45%)`;
  }

  function truncateId(id: string | null): string {
    if (!id) return 'No chain';
    return id.slice(0, 8);
  }
</script>

<button
  class="chain-badge"
  class:clickable={onClick && chainId}
  style="--badge-color: {getChainColor(chainId)}"
  onclick={() => chainId && onClick?.(chainId)}
  disabled={!onClick || !chainId}
  data-testid="chain-badge"
>
  <span class="dot"></span>
  {truncateId(chainId)}
</button>

<style>
  .chain-badge {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 8px;
    font-size: 0.75em;
    font-family: monospace;
    border: 1px solid var(--badge-color);
    border-radius: 12px;
    background: transparent;
    color: var(--badge-color);
  }

  .chain-badge.clickable {
    cursor: pointer;
  }

  .chain-badge.clickable:hover {
    background: var(--badge-color);
    color: white;
  }

  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--badge-color);
  }
</style>
