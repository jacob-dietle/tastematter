<script lang="ts">
  import { getHeatColor, calculateIntensity } from '$lib/utils/colors';

  interface Props {
    filePath: string;
    dates: string[];
    buckets: Record<string, number>;
    maxCount: number;
    onHover?: (file: string, date: string) => void;
    onLeave?: () => void;
  }

  let { filePath, dates, buckets, maxCount, onHover, onLeave }: Props = $props();

  function getColor(date: string): string {
    const count = buckets[date] ?? 0;
    const intensity = calculateIntensity(count, maxCount);
    return getHeatColor(intensity);
  }

  function handleMouseEnter(date: string) {
    onHover?.(filePath, date);
  }

  function handleMouseLeave() {
    onLeave?.();
  }
</script>

<div class="timeline-row">
  <div class="file-label" title={filePath}>
    {filePath}
  </div>
  <div class="cells">
    {#each dates as date}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        class="heat-cell"
        style="background-color: {getColor(date)}"
        onmouseenter={() => handleMouseEnter(date)}
        onmouseleave={handleMouseLeave}
        title="{buckets[date] ?? 0} accesses on {date}"
      ></div>
    {/each}
  </div>
</div>

<style>
  .timeline-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .file-label {
    width: 192px;
    font-size: 0.75rem;
    font-family: monospace;
    color: #333;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    text-align: right;
  }

  .cells {
    display: flex;
    gap: 2px;
  }

  .heat-cell {
    width: 24px;
    height: 16px;
    border-radius: 2px;
    cursor: pointer;
    transition: transform 0.1s ease;
  }

  .heat-cell:hover {
    transform: scale(1.2);
  }
</style>
