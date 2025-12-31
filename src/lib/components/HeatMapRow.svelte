<script lang="ts">
  import { calculateIntensity } from '$lib/utils/aggregation';

  // Heat map colors - keep in sync with app.css CSS variables
  // --heat-empty, --heat-low, --heat-high
  const HEAT_EMPTY = '#e8e4d9';  // Paper
  const HEAT_LOW = '#8b4513';    // Sienna
  const HEAT_HIGH = '#1a1a2e';   // Ink

  interface Props {
    label: string;
    accessCount: number;
    maxAccessCount: number;
    lastAccess: string | null;
    isDirectory: boolean;
    onclick?: () => void;
  }

  let {
    label,
    accessCount,
    maxAccessCount,
    lastAccess,
    isDirectory,
    onclick
  }: Props = $props();

  let intensity = $derived(calculateIntensity(accessCount, maxAccessCount));

  // Color interpolation: paper -> sienna -> ink
  let backgroundColor = $derived(interpolateColor(intensity));
  let textColor = $derived(intensity > 0.5 ? HEAT_EMPTY : HEAT_HIGH);

  function interpolateColor(t: number): string {
    // 0 = paper (empty), 0.5 = sienna (low), 1 = ink (high)
    if (t < 0.5) {
      return lerpColor(HEAT_EMPTY, HEAT_LOW, t * 2);
    }
    return lerpColor(HEAT_LOW, HEAT_HIGH, (t - 0.5) * 2);
  }

  function lerpColor(a: string, b: string, t: number): string {
    const [r1, g1, b1] = hexToRgb(a);
    const [r2, g2, b2] = hexToRgb(b);
    const r = Math.round(r1 + (r2 - r1) * t);
    const g = Math.round(g1 + (g2 - g1) * t);
    const blue = Math.round(b1 + (b2 - b1) * t);
    return `rgb(${r}, ${g}, ${blue})`;
  }

  function hexToRgb(hex: string): [number, number, number] {
    const result = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex);
    return result
      ? [parseInt(result[1], 16), parseInt(result[2], 16), parseInt(result[3], 16)]
      : [0, 0, 0];
  }

  function formatRelativeTime(dateStr: string): string {
    const date = new Date(dateStr);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24));

    if (diffDays === 0) return 'Today';
    if (diffDays === 1) return 'Yesterday';
    if (diffDays < 7) return `${diffDays}d ago`;
    if (diffDays < 30) return `${Math.floor(diffDays / 7)}w ago`;
    return `${Math.floor(diffDays / 30)}mo ago`;
  }
</script>

<div
  class="heat-map-row"
  class:directory={isDirectory}
  class:clickable={!!onclick}
  style="background-color: {backgroundColor}; color: {textColor};"
  onclick={onclick}
  onkeydown={(e) => e.key === 'Enter' && onclick?.()}
  role={onclick ? 'button' : 'row'}
  tabindex={onclick ? 0 : -1}
>
  <span class="label">
    {#if isDirectory}
      <span class="icon">📁</span>
    {:else}
      <span class="icon">📄</span>
    {/if}
    {label}
  </span>
  <span class="count">{accessCount}</span>
  {#if lastAccess}
    <span class="recency">{formatRelativeTime(lastAccess)}</span>
  {/if}
</div>

<style>
  .heat-map-row {
    display: flex;
    align-items: center;
    padding: 0.5rem 1rem;
    border-bottom: 1px solid rgba(0, 0, 0, 0.1);
    transition: transform 0.1s ease;
  }

  .heat-map-row.clickable:hover {
    transform: translateX(4px);
    cursor: pointer;
  }

  .heat-map-row.clickable:focus {
    outline: 2px solid var(--focus-ring);
    outline-offset: -2px;
  }

  .label {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-family: monospace;
    font-size: 0.875rem;
  }

  .icon {
    flex-shrink: 0;
  }

  .count {
    font-weight: bold;
    min-width: 3rem;
    text-align: right;
  }

  .recency {
    min-width: 5rem;
    text-align: right;
    opacity: 0.8;
    font-size: 0.8125rem;
    margin-left: 1rem;
  }
</style>
