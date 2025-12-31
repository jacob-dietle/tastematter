<script lang="ts">
  import { COLORS } from '$lib/utils/colors';
</script>

<div class="timeline-legend">
  <span class="label">Activity:</span>
  <div class="scale">
    <span class="scale-label">Low</span>
    <div class="swatches">
      <div class="swatch" style="background-color: {COLORS.empty}" title="No activity"></div>
      <div class="swatch" style="background-color: {COLORS.low}" title="Low activity"></div>
      <div class="swatch" style="background-color: {COLORS.medium}" title="Medium activity"></div>
      <div class="swatch" style="background-color: {COLORS.high}" title="High activity"></div>
    </div>
    <span class="scale-label">High</span>
  </div>
</div>

<style>
  .timeline-legend {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 0.7rem;
    color: var(--text-secondary);
    background: var(--bg-secondary);
    padding: 6px 10px;
    border-radius: var(--radius-md);
  }

  .label {
    font-weight: 500;
    color: var(--text-muted);
    text-transform: uppercase;
    font-size: 0.6rem;
    letter-spacing: 0.05em;
  }

  .scale {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .scale-label {
    font-size: 0.6rem;
    color: var(--text-muted);
  }

  .swatches {
    display: flex;
    gap: 2px;
  }

  .swatch {
    width: 14px;
    height: 14px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border-color);
    cursor: help;
    transition: transform 0.1s ease;
  }

  .swatch:hover {
    transform: scale(1.15);
  }
</style>
