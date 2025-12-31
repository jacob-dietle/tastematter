<script lang="ts">
  import type { TimeBucket } from '$lib/types';

  interface Props {
    buckets: TimeBucket[];
  }

  let { buckets }: Props = $props();

  function getDay(date: string): string {
    return date.split('-')[2];
  }
</script>

<div class="timeline-axis">
  {#each buckets as bucket}
    <div class="axis-cell">
      <span class="day-of-week">{bucket.day_of_week}</span>
      <span class="date">{getDay(bucket.date)}</span>
    </div>
  {/each}
</div>

<style>
  .timeline-axis {
    display: flex;
    gap: 2px;
    padding-left: 200px; /* Space for file labels */
  }

  .axis-cell {
    width: 24px;
    display: flex;
    flex-direction: column;
    align-items: center;
    font-size: 0.7rem;
    color: #666;
  }

  .day-of-week {
    font-weight: 500;
  }

  .date {
    color: #999;
  }
</style>
