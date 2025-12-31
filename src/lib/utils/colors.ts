/**
 * Ink & Paper Color Palette
 * Inspired by Tufte/Rams minimalism - aged document aesthetic
 */
export const COLORS = {
  empty: '#f6f8fa',    // Paper white (0 activity)
  low: '#d4c9b8',      // Faded ink (low activity)
  medium: '#8b4513',   // Aged ink (medium activity)
  high: '#1a1a2e',     // Deep ink (high activity)
} as const;

/**
 * Get heat color based on intensity (0-1)
 */
export function getHeatColor(intensity: number): string {
  if (intensity === 0) return COLORS.empty;
  if (intensity < 0.33) return COLORS.low;
  if (intensity < 0.66) return COLORS.medium;
  return COLORS.high;
}

/**
 * Calculate intensity from access count relative to max
 */
export function calculateIntensity(count: number, maxCount: number): number {
  if (maxCount === 0) return 0;
  return Math.min(count / maxCount, 1);
}
