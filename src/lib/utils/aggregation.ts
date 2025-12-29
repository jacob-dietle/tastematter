/**
 * Aggregation utilities for Heat Map View
 *
 * Handles directory grouping, date comparison, and intensity calculation.
 */
import type { FileResult, DirectoryResult } from '$lib/types';

/**
 * Extract parent directory from a file path.
 * Handles both Unix (/) and Windows (\) separators.
 * Returns '/' for root-level files.
 */
export function getParentDirectory(filePath: string): string {
  // Normalize to forward slashes
  const normalized = filePath.replace(/\\/g, '/');
  const parts = normalized.split('/');

  if (parts.length <= 1) {
    return '/';
  }

  return parts.slice(0, -1).join('/');
}

/**
 * Return the later of two dates, handling nulls.
 * Returns null only if both inputs are null.
 */
export function maxDate(a: string | null, b: string | null): string | null {
  if (!a) return b;
  if (!b) return a;
  return a > b ? a : b;
}

/**
 * Calculate intensity as 0-1 scale for color interpolation.
 * Handles edge cases: zero maxCount returns 0, values > max cap at 1.
 */
export function calculateIntensity(count: number, maxCount: number): number {
  if (maxCount === 0) return 0;
  return Math.min(count / maxCount, 1);
}

/**
 * Aggregate files by their parent directory.
 * Used for directory-level heat map view.
 */
export function aggregateByDirectory(files: FileResult[]): DirectoryResult[] {
  if (files.length === 0) return [];

  const dirMap = new Map<string, DirectoryResult>();

  for (const file of files) {
    const dir = getParentDirectory(file.file_path);

    if (!dirMap.has(dir)) {
      dirMap.set(dir, {
        directory_path: dir,
        file_count: 0,
        total_access_count: 0,
        max_access_count: 0,
        last_access: null,
        files: []
      });
    }

    const entry = dirMap.get(dir)!;
    entry.file_count++;
    entry.total_access_count += file.access_count;
    entry.max_access_count = Math.max(entry.max_access_count, file.access_count);
    entry.last_access = maxDate(entry.last_access, file.last_access);
    entry.files.push(file);
  }

  return Array.from(dirMap.values());
}
