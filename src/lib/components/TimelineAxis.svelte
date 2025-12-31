<script lang="ts">
  import type { TimeBucket } from '$lib/types';

  interface Props {
    buckets: TimeBucket[];
  }

  let { buckets }: Props = $props();

  function getDay(date: string): string {
    return date.split('-')[2];
  }

  function getMonth(date: string): string {
    const months = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun',
                    'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
    const monthNum = parseInt(date.split('-')[1], 10) - 1;
    return months[monthNum];
  }

  function isWeekend(dayOfWeek: string): boolean {
    return dayOfWeek === 'Sat' || dayOfWeek === 'Sun';
  }

  function isToday(date: string): boolean {
    const today = new Date().toISOString().split('T')[0];
    return date === today;
  }

  function isFirstOfMonth(date: string, index: number): boolean {
    return getDay(date) === '01' || index === 0;
  }
</script>

<div class="timeline-axis">
  <div class="axis-label-spacer">
    <span class="label">File</span>
  </div>
  <div class="axis-activity-spacer">
    <span class="label">Σ</span>
  </div>
  <div class="axis-cells">
    {#each buckets as bucket, i}
      <div
        class="axis-cell"
        class:weekend={isWeekend(bucket.day_of_week)}
        class:today={isToday(bucket.date)}
      >
        {#if isFirstOfMonth(bucket.date, i)}
          <span class="month">{getMonth(bucket.date)}</span>
        {:else}
          <span class="month-spacer"></span>
        {/if}
        <span class="day-of-week">{bucket.day_of_week.charAt(0)}</span>
        <span class="date">{getDay(bucket.date)}</span>
      </div>
    {/each}
  </div>
</div>

<style>
  .timeline-axis {
    display: flex;
    gap: 8px;
    padding: 0 8px;
    align-items: flex-end;
    border-bottom: 1px solid var(--border-color);
    padding-bottom: 8px;
    margin-bottom: 4px;
  }

  .axis-label-spacer {
    width: 200px;
    text-align: right;
  }

  .axis-activity-spacer {
    width: 32px;
    text-align: right;
  }

  .label {
    font-size: 0.65rem;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .axis-cells {
    display: flex;
    gap: 2px;
  }

  .axis-cell {
    width: 28px;
    display: flex;
    flex-direction: column;
    align-items: center;
    font-size: 0.65rem;
    color: var(--text-secondary);
    padding: 2px 0;
    border-radius: 3px;
  }

  .axis-cell.weekend {
    background: var(--bg-secondary);
    color: var(--text-muted);
  }

  .axis-cell.today {
    background: var(--focus-ring);
    color: var(--bg-primary);
  }

  .month {
    font-size: 0.6rem;
    font-weight: 600;
    color: var(--text-primary);
    margin-bottom: 2px;
  }

  .month-spacer {
    height: 0.6rem;
    margin-bottom: 2px;
  }

  .day-of-week {
    font-weight: 600;
    font-size: 0.6rem;
  }

  .date {
    color: var(--text-muted);
    font-size: 0.65rem;
  }
</style>
