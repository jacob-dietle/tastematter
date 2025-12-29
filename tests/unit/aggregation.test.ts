/**
 * Aggregation Utility Tests - Phase 2 Heat Map
 *
 * TDD: Write tests FIRST (RED), then implement (GREEN)
 */
import { describe, test, expect } from 'vitest';
import {
  aggregateByDirectory,
  getParentDirectory,
  maxDate,
  calculateIntensity
} from '$lib/utils/aggregation';
import type { FileResult } from '$lib/types';

describe('getParentDirectory', () => {
  test('extracts parent from Unix path', () => {
    expect(getParentDirectory('src/lib/components/HeatMap.svelte'))
      .toBe('src/lib/components');
  });

  test('extracts parent from Windows path', () => {
    expect(getParentDirectory('src\\lib\\components\\HeatMap.svelte'))
      .toBe('src/lib/components');
  });

  test('returns root for top-level file', () => {
    expect(getParentDirectory('README.md')).toBe('/');
  });

  test('handles deeply nested paths', () => {
    expect(getParentDirectory('a/b/c/d/e/f.txt')).toBe('a/b/c/d/e');
  });
});

describe('maxDate', () => {
  test('returns later date when both provided', () => {
    expect(maxDate('2025-12-25', '2025-12-27')).toBe('2025-12-27');
    expect(maxDate('2025-12-27', '2025-12-25')).toBe('2025-12-27');
  });

  test('returns non-null when one is null', () => {
    expect(maxDate(null, '2025-12-27')).toBe('2025-12-27');
    expect(maxDate('2025-12-27', null)).toBe('2025-12-27');
  });

  test('returns null when both null', () => {
    expect(maxDate(null, null)).toBeNull();
  });
});

describe('calculateIntensity', () => {
  test('returns 0 for zero count', () => {
    expect(calculateIntensity(0, 100)).toBe(0);
  });

  test('returns 1 for max count', () => {
    expect(calculateIntensity(100, 100)).toBe(1);
  });

  test('returns proportional value', () => {
    expect(calculateIntensity(50, 100)).toBe(0.5);
    expect(calculateIntensity(25, 100)).toBe(0.25);
  });

  test('caps at 1 for values exceeding max', () => {
    expect(calculateIntensity(150, 100)).toBe(1);
  });

  test('handles zero maxCount', () => {
    expect(calculateIntensity(10, 0)).toBe(0);
  });
});

describe('aggregateByDirectory', () => {
  const testFiles: FileResult[] = [
    { file_path: 'src/lib/a.ts', access_count: 10, last_access: '2025-12-27' },
    { file_path: 'src/lib/b.ts', access_count: 5, last_access: '2025-12-25' },
    { file_path: 'src/routes/c.svelte', access_count: 3, last_access: '2025-12-26' },
    { file_path: 'README.md', access_count: 1, last_access: '2025-12-20' },
  ];

  test('groups files by parent directory', () => {
    const result = aggregateByDirectory(testFiles);
    expect(result).toHaveLength(3); // src/lib, src/routes, /
  });

  test('calculates total access count per directory', () => {
    const result = aggregateByDirectory(testFiles);
    const srcLib = result.find(d => d.directory_path === 'src/lib');
    expect(srcLib?.total_access_count).toBe(15); // 10 + 5
  });

  test('tracks max access count for intensity scaling', () => {
    const result = aggregateByDirectory(testFiles);
    const srcLib = result.find(d => d.directory_path === 'src/lib');
    expect(srcLib?.max_access_count).toBe(10);
  });

  test('counts files per directory', () => {
    const result = aggregateByDirectory(testFiles);
    const srcLib = result.find(d => d.directory_path === 'src/lib');
    expect(srcLib?.file_count).toBe(2);
  });

  test('preserves child files for drill-down', () => {
    const result = aggregateByDirectory(testFiles);
    const srcLib = result.find(d => d.directory_path === 'src/lib');
    expect(srcLib?.files).toHaveLength(2);
  });

  test('calculates latest access date per directory', () => {
    const result = aggregateByDirectory(testFiles);
    const srcLib = result.find(d => d.directory_path === 'src/lib');
    expect(srcLib?.last_access).toBe('2025-12-27');
  });

  test('handles empty input', () => {
    expect(aggregateByDirectory([])).toEqual([]);
  });
});
