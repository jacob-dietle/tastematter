<script lang="ts">
  import { COLORS } from '$lib/utils/colors';
</script>

<div class="timeline-legend">
  <span class="label">Less</span>
  <div class="swatches">
    <div class="swatch" style="background-color: {COLORS.empty}"></div>
    <div class="swatch" style="background-color: {COLORS.low}"></div>
    <div class="swatch" style="background-color: {COLORS.medium}"></div>
    <div class="swatch" style="background-color: {COLORS.high}"></div>
  </div>
  <span class="label">More</span>
</div>

<style>
  .timeline-legend {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 0.7rem;
    color: #666;
  }

  .swatches {
    display: flex;
    gap: 2px;
  }

  .swatch {
    width: 12px;
    height: 12px;
    border-radius: 2px;
  }

  .label {
    color: #999;
  }
</style>
