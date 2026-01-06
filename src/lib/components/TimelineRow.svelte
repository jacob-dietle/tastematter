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

  // Calculate total activity for this file (for hierarchy/sorting context)
  let totalActivity = $derived(
    Object.values(buckets).reduce((sum, count) => sum + count, 0)
  );

  // Determine if this is a "hot" file (top activity)
  let isHot = $derived(totalActivity > 0 && totalActivity >= maxCount * 0.5);

  // Pre-compute date classifications once (O(n) instead of O(n²) on each render)
  const todayStr = new Date().toISOString().split('T')[0];
  const dateClassifications = $derived(
    dates.reduce((acc, date) => {
      const d = new Date(date);
      const day = d.getDay();
      acc[date] = {
        isWeekend: day === 0 || day === 6,
        isToday: date === todayStr
      };
      return acc;
    }, {} as Record<string, { isWeekend: boolean; isToday: boolean }>)
  );

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

  // Extract filename for display
  let fileName = $derived(filePath.split('/').pop() || filePath);
  let dirPath = $derived(filePath.split('/').slice(0, -1).join('/'));
</script>

<div class="timeline-row" class:hot={isHot}>
  <div class="file-label" title={filePath}>
    <span class="file-name">{fileName}</span>
    {#if dirPath}
      <span class="dir-path">{dirPath}/</span>
    {/if}
  </div>
  <div class="activity-indicator">
    {#if totalActivity > 0}
      <span class="activity-count">{totalActivity}</span>
    {/if}
  </div>
  <div class="cells">
    {#each dates as date}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        class="heat-cell"
        class:weekend={dateClassifications[date]?.isWeekend}
        class:today={dateClassifications[date]?.isToday}
        class:has-activity={buckets[date] > 0}
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
    padding: 4px 8px;
    border-radius: var(--radius-sm);
    transition: background-color var(--duration-fast) ease;
  }

  .timeline-row:hover {
    background: var(--bg-hover);
  }

  .timeline-row.hot {
    background: rgba(139, 69, 19, 0.08);
  }

  .timeline-row.hot:hover {
    background: rgba(139, 69, 19, 0.15);
  }

  .file-label {
    width: 200px;
    font-size: 0.75rem;
    font-family: monospace;
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 1px;
    overflow: hidden;
  }

  .file-name {
    color: var(--text-primary);
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 100%;
  }

  .dir-path {
    color: var(--text-muted);
    font-size: 0.65rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 100%;
  }

  .activity-indicator {
    width: 32px;
    text-align: right;
  }

  .activity-count {
    font-size: 0.7rem;
    font-weight: 600;
    color: var(--text-secondary);
    background: var(--bg-secondary);
    padding: 2px 6px;
    border-radius: var(--radius-md);
  }

  .cells {
    display: flex;
    gap: 2px;
  }

  .heat-cell {
    width: 28px;
    height: 20px;
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: transform 0.1s ease, box-shadow 0.1s ease;
    border: 1px solid transparent;
  }

  .heat-cell.weekend {
    opacity: 0.7;
  }

  .heat-cell.today {
    border-color: var(--focus-ring);
    box-shadow: 0 0 0 1px var(--focus-ring);
  }

  .heat-cell.has-activity:hover {
    transform: scale(1.15);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.3);
    z-index: 1;
  }

  .heat-cell:not(.has-activity) {
    opacity: 0.4;
  }
</style>
