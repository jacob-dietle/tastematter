/**
 * TimelineStore (Refactored) Tests - Unified Data Architecture (Spec 08)
 *
 * TDD: Write tests FIRST (RED), then implement (GREEN)
 *
 * Purpose: Timeline view store refactored to use shared context
 * - Uses context.timeRange instead of own selectedRange
 * - Uses context.selectedChain for filtering
 * - Refetches when context changes
 */
import { describe, test, expect, vi, beforeEach } from 'vitest';

// Mock the API before importing the store
vi.mock('$lib/api', () => ({
  queryTimeline: vi.fn(),
  queryChains: vi.fn()
}));

import { createTimelineStore } from '$lib/stores/timeline.svelte';
import { createContextStore, type ContextStore } from '$lib/stores/context.svelte';
import { queryTimeline } from '$lib/api';
import type { TimelineData } from '$lib/types';

const mockTimelineData: TimelineData = {
  time_range: '7d',
  start_date: '2025-12-22',
  end_date: '2025-12-29',
  buckets: [
    { date: '2025-12-29', day_of_week: 'Sun', access_count: 50, files_touched: 12, sessions: ['s1', 's2'] },
    { date: '2025-12-28', day_of_week: 'Sat', access_count: 30, files_touched: 8, sessions: ['s3'] },
    { date: '2025-12-27', day_of_week: 'Fri', access_count: 80, files_touched: 20, sessions: ['s4', 's5', 's6'] }
  ],
  files: [
    {
      file_path: 'src/lib/store.ts',
      total_accesses: 45,
      buckets: { '2025-12-29': 20, '2025-12-28': 15, '2025-12-27': 10 },
      first_access: '2025-12-27T10:00:00Z',
      last_access: '2025-12-29T18:00:00Z'
    },
    {
      file_path: 'src/App.svelte',
      total_accesses: 30,
      buckets: { '2025-12-29': 15, '2025-12-27': 15 },
      first_access: '2025-12-27T09:00:00Z',
      last_access: '2025-12-29T17:00:00Z'
    }
  ],
  summary: {
    total_accesses: 160,
    total_files: 25,
    peak_day: '2025-12-27',
    peak_count: 80
  }
};

// Create a mock context for testing
function createMockContext(): ContextStore {
  const ctx = createContextStore();
  return ctx;
}

describe('timelineStore (refactored)', () => {
  let mockContext: ContextStore;

  beforeEach(() => {
    vi.clearAllMocks();
    mockContext = createMockContext();
  });

  // Initial state tests
  describe('initial state', () => {
    test('starts with loading = false', () => {
      const store = createTimelineStore(mockContext);
      expect(store.loading).toBe(false);
    });

    test('starts with data = null', () => {
      const store = createTimelineStore(mockContext);
      expect(store.data).toBeNull();
    });

    test('starts with error = null', () => {
      const store = createTimelineStore(mockContext);
      expect(store.error).toBeNull();
    });

    test('starts with hoveredCell = null', () => {
      const store = createTimelineStore(mockContext);
      expect(store.hoveredCell).toBeNull();
    });
  });

  // Context integration tests
  describe('context integration', () => {
    test('fetch uses context.timeRange', async () => {
      vi.mocked(queryTimeline).mockResolvedValue(mockTimelineData);

      mockContext.setTimeRange('30d');
      const store = createTimelineStore(mockContext);
      await store.fetch();

      expect(queryTimeline).toHaveBeenCalledWith(
        expect.objectContaining({ time: '30d' })
      );
    });

    test('timeRange getter returns context.timeRange', () => {
      mockContext.setTimeRange('14d');
      const store = createTimelineStore(mockContext);

      expect(store.timeRange).toBe('14d');
    });

    test('fetch includes limit parameter', async () => {
      vi.mocked(queryTimeline).mockResolvedValue(mockTimelineData);

      const store = createTimelineStore(mockContext);
      await store.fetch();

      expect(queryTimeline).toHaveBeenCalledWith(
        expect.objectContaining({ limit: 30 })
      );
    });
  });

  // Fetch behavior tests
  describe('fetch behavior', () => {
    test('sets loading true during fetch', async () => {
      vi.mocked(queryTimeline).mockImplementation(() =>
        new Promise(resolve => setTimeout(() => resolve(mockTimelineData), 50))
      );

      const store = createTimelineStore(mockContext);
      const fetchPromise = store.fetch();

      expect(store.loading).toBe(true);

      await fetchPromise;
      expect(store.loading).toBe(false);
    });

    test('populates data on success', async () => {
      vi.mocked(queryTimeline).mockResolvedValue(mockTimelineData);

      const store = createTimelineStore(mockContext);
      await store.fetch();

      expect(store.data).not.toBeNull();
      expect(store.data?.time_range).toBe('7d');
      expect(store.data?.files).toHaveLength(2);
    });

    test('sets error on failure', async () => {
      const mockError = { code: 'TIMELINE_ERROR', message: 'Query failed' };
      vi.mocked(queryTimeline).mockRejectedValue(mockError);

      const store = createTimelineStore(mockContext);
      await store.fetch();

      expect(store.error).not.toBeNull();
      expect(store.error?.code).toBe('TIMELINE_ERROR');
      expect(store.data).toBeNull();
    });

    test('clears error on successful fetch', async () => {
      const mockError = { code: 'TIMELINE_ERROR', message: 'Failed' };

      vi.mocked(queryTimeline).mockRejectedValueOnce(mockError);

      const store = createTimelineStore(mockContext);
      await store.fetch();
      expect(store.error).not.toBeNull();

      vi.mocked(queryTimeline).mockResolvedValueOnce(mockTimelineData);
      await store.fetch();

      expect(store.error).toBeNull();
      expect(store.data).not.toBeNull();
    });
  });

  // Hover state tests
  describe('hover state', () => {
    test('setHoveredCell updates hover state', () => {
      const store = createTimelineStore(mockContext);
      store.setHoveredCell('src/lib/store.ts', '2025-12-29');

      expect(store.hoveredCell).toEqual({
        file: 'src/lib/store.ts',
        date: '2025-12-29'
      });
    });

    test('clearHover sets hoveredCell to null', () => {
      const store = createTimelineStore(mockContext);
      store.setHoveredCell('src/lib/store.ts', '2025-12-29');
      store.clearHover();

      expect(store.hoveredCell).toBeNull();
    });
  });

  // Derived values tests
  describe('derived values', () => {
    test('maxAccessCount returns max from files', async () => {
      vi.mocked(queryTimeline).mockResolvedValue(mockTimelineData);

      const store = createTimelineStore(mockContext);
      await store.fetch();

      expect(store.maxAccessCount).toBe(45); // src/lib/store.ts has 45
    });

    test('maxAccessCount returns 0 when no data', () => {
      const store = createTimelineStore(mockContext);
      expect(store.maxAccessCount).toBe(0);
    });

    test('getIntensity returns 0 for non-existent file', async () => {
      vi.mocked(queryTimeline).mockResolvedValue(mockTimelineData);

      const store = createTimelineStore(mockContext);
      await store.fetch();

      expect(store.getIntensity('non-existent.ts', '2025-12-29')).toBe(0);
    });

    test('getIntensity returns normalized value', async () => {
      vi.mocked(queryTimeline).mockResolvedValue(mockTimelineData);

      const store = createTimelineStore(mockContext);
      await store.fetch();

      // src/lib/store.ts has 45 total, max is 45, so file intensity relative
      // But getIntensity should return bucket count / max
      // 2025-12-29 has 20 accesses, max total is 45
      const intensity = store.getIntensity('src/lib/store.ts', '2025-12-29');
      expect(intensity).toBeCloseTo(20 / 45);
    });

    test('files returns data.files', async () => {
      vi.mocked(queryTimeline).mockResolvedValue(mockTimelineData);

      const store = createTimelineStore(mockContext);
      await store.fetch();

      expect(store.files).toHaveLength(2);
      expect(store.files[0].file_path).toBe('src/lib/store.ts');
    });

    test('buckets returns data.buckets', async () => {
      vi.mocked(queryTimeline).mockResolvedValue(mockTimelineData);

      const store = createTimelineStore(mockContext);
      await store.fetch();

      expect(store.buckets).toHaveLength(3);
      expect(store.buckets[0].date).toBe('2025-12-29');
    });

    test('dates returns array of bucket dates', async () => {
      vi.mocked(queryTimeline).mockResolvedValue(mockTimelineData);

      const store = createTimelineStore(mockContext);
      await store.fetch();

      expect(store.dates).toEqual(['2025-12-29', '2025-12-28', '2025-12-27']);
    });
  });
});
