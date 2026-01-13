import { describe, test, expect, vi, beforeEach } from 'vitest';

// Mock the API before importing the store
vi.mock('$lib/api', () => ({
  queryTimeline: vi.fn()
}));

import { createTimelineStore } from '$lib/stores/timeline.svelte';
import { queryTimeline } from '$lib/api';
import type { TimelineData } from '$lib/types';

const mockTimelineData: TimelineData = {
  time_range: '7d',
  start_date: '2025-12-24',
  end_date: '2025-12-30',
  buckets: [
    { date: '2025-12-24', day_of_week: 'Tue', access_count: 15, files_touched: 5, sessions: ['s1'] },
    { date: '2025-12-25', day_of_week: 'Wed', access_count: 0, files_touched: 0, sessions: [] },
    { date: '2025-12-26', day_of_week: 'Thu', access_count: 25, files_touched: 8, sessions: ['s2'] },
    { date: '2025-12-27', day_of_week: 'Fri', access_count: 10, files_touched: 3, sessions: ['s3'] },
    { date: '2025-12-28', day_of_week: 'Sat', access_count: 5, files_touched: 2, sessions: [] },
    { date: '2025-12-29', day_of_week: 'Sun', access_count: 20, files_touched: 6, sessions: ['s4'] },
    { date: '2025-12-30', day_of_week: 'Mon', access_count: 45, files_touched: 12, sessions: ['s5', 's6'] },
  ],
  files: [
    {
      file_path: 'src/lib/query_engine.py',
      total_accesses: 25,
      buckets: { '2025-12-24': 10, '2025-12-26': 15 },
      first_access: '2025-12-24T10:00:00Z',
      last_access: '2025-12-30T14:00:00Z'
    },
    {
      file_path: 'src/lib/commands.rs',
      total_accesses: 18,
      buckets: { '2025-12-30': 18 },
      first_access: '2025-12-30T09:00:00Z',
      last_access: '2025-12-30T16:00:00Z'
    },
  ],
  summary: {
    total_accesses: 150,
    total_files: 12,
    peak_day: '2025-12-30',
    peak_count: 45
  }
};

describe('timelineStore', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // Initial state tests (5)
  describe('initial state', () => {
    test('starts with loading false', () => {
      const store = createTimelineStore();
      expect(store.loading).toBe(false);
    });

    test('starts with no data', () => {
      const store = createTimelineStore();
      expect(store.data).toBeNull();
    });

    test('starts with 7d selected', () => {
      const store = createTimelineStore();
      expect(store.selectedRange).toBe('7d');
    });

    test('starts with no hovered cell', () => {
      const store = createTimelineStore();
      expect(store.hoveredCell).toBeNull();
    });

    test('maxAccessCount returns 0 when no data', () => {
      const store = createTimelineStore();
      expect(store.maxAccessCount).toBe(0);
    });
  });

  // Fetch behavior tests (4)
  describe('fetch behavior', () => {
    test('sets loading true during fetch', async () => {
      vi.mocked(queryTimeline).mockImplementation(() =>
        new Promise(resolve => setTimeout(() => resolve(mockTimelineData), 50))
      );

      const store = createTimelineStore();
      const fetchPromise = store.fetch();

      // Check loading is true immediately
      expect(store.loading).toBe(true);

      await fetchPromise;
      expect(store.loading).toBe(false);
    });

    test('stores data on success', async () => {
      vi.mocked(queryTimeline).mockResolvedValue(mockTimelineData);

      const store = createTimelineStore();
      await store.fetch();

      expect(store.data).not.toBeNull();
      expect(store.data?.time_range).toBe('7d');
      expect(store.data?.files.length).toBe(2);
      expect(store.error).toBeNull();
    });

    test('stores error on failure', async () => {
      const mockError = {
        code: 'TIMELINE_ERROR',
        message: 'Failed to fetch timeline data'
      };

      vi.mocked(queryTimeline).mockRejectedValue(mockError);

      const store = createTimelineStore();
      await store.fetch();

      expect(store.error).not.toBeNull();
      expect(store.error?.code).toBe('TIMELINE_ERROR');
      expect(store.data).toBeNull();
    });

    test('sets loading false after fetch', async () => {
      vi.mocked(queryTimeline).mockResolvedValue(mockTimelineData);

      const store = createTimelineStore();
      await store.fetch();

      expect(store.loading).toBe(false);
    });
  });

  // Range selection tests (2)
  describe('range selection', () => {
    test('setRange updates selectedRange', () => {
      const store = createTimelineStore();

      store.setRange('14d');
      expect(store.selectedRange).toBe('14d');

      store.setRange('30d');
      expect(store.selectedRange).toBe('30d');
    });

    test('setRange triggers fetch', async () => {
      vi.mocked(queryTimeline).mockResolvedValue(mockTimelineData);

      const store = createTimelineStore();
      await store.setRange('14d');

      expect(queryTimeline).toHaveBeenCalledWith({
        time: '14d',
        limit: 30
      });
    });
  });

  // Hover state tests (2)
  describe('hover state', () => {
    test('setHoveredCell updates state', () => {
      const store = createTimelineStore();

      store.setHoveredCell('src/file.ts', '2025-12-30');

      expect(store.hoveredCell).toEqual({
        file: 'src/file.ts',
        date: '2025-12-30'
      });
    });

    test('clearHover resets to null', () => {
      const store = createTimelineStore();

      store.setHoveredCell('src/file.ts', '2025-12-30');
      expect(store.hoveredCell).not.toBeNull();

      store.clearHover();
      expect(store.hoveredCell).toBeNull();
    });
  });

  // Derived values tests (2)
  describe('derived values', () => {
    test('maxAccessCount returns max from files', async () => {
      vi.mocked(queryTimeline).mockResolvedValue(mockTimelineData);

      const store = createTimelineStore();
      await store.fetch();

      // Max access count is 25 (from query_engine.py)
      expect(store.maxAccessCount).toBe(25);
    });

    test('getColorForCell returns correct intensity', async () => {
      vi.mocked(queryTimeline).mockResolvedValue(mockTimelineData);

      const store = createTimelineStore();
      await store.fetch();

      // High intensity file (query_engine.py on 2025-12-26 has 15 accesses, max is 25)
      const intensity = store.getIntensity('src/lib/query_engine.py', '2025-12-26');
      expect(intensity).toBeCloseTo(15 / 25, 2);

      // File with no accesses on that date
      const zeroIntensity = store.getIntensity('src/lib/commands.rs', '2025-12-24');
      expect(zeroIntensity).toBe(0);
    });
  });
});
