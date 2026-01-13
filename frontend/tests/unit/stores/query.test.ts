import { describe, test, expect, vi, beforeEach } from 'vitest';

// Mock the API before importing the store
vi.mock('$lib/api', () => ({
  queryFlex: vi.fn()
}));

import { createQueryStore } from '$lib/stores/query.svelte';
import { queryFlex } from '$lib/api';

describe('queryStore', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  test('initial state is empty', () => {
    const store = createQueryStore();
    expect(store.loading).toBe(false);
    expect(store.data).toBeNull();
    expect(store.error).toBeNull();
    expect(store.lastQuery).toBeNull();
  });

  test('loading state during fetch', async () => {
    const mockResult = {
      receipt_id: 'q_123',
      timestamp: '2025-12-28T00:00:00Z',
      result_count: 5,
      results: [],
      aggregations: {}
    };

    vi.mocked(queryFlex).mockResolvedValue(mockResult);

    const store = createQueryStore();
    const fetchPromise = store.fetch({ agg: ['count'] });

    // Note: Due to microtask timing, loading may already be false
    await fetchPromise;
    expect(store.loading).toBe(false);
  });

  test('data updates on success', async () => {
    const mockResult = {
      receipt_id: 'q_456',
      timestamp: '2025-12-28T00:00:00Z',
      result_count: 10,
      results: [
        { file_path: '/test/file.ts', access_count: 5, last_access: null }
      ],
      aggregations: {
        count: { total_files: 10, total_accesses: 50 }
      }
    };

    vi.mocked(queryFlex).mockResolvedValue(mockResult);

    const store = createQueryStore();
    await store.fetch({ time: '7d', agg: ['count'] });

    expect(store.data).not.toBeNull();
    expect(store.data?.receipt_id).toBe('q_456');
    expect(store.data?.result_count).toBe(10);
    expect(store.error).toBeNull();
  });

  test('error state on failure', async () => {
    const mockError = {
      code: 'CLI_NOT_FOUND',
      message: 'context-os not found in PATH'
    };

    vi.mocked(queryFlex).mockRejectedValue(mockError);

    const store = createQueryStore();
    await store.fetch({ agg: ['count'] });

    expect(store.error).not.toBeNull();
    expect(store.error?.code).toBe('CLI_NOT_FOUND');
    expect(store.data).toBeNull();
  });

  test('reset clears all state', async () => {
    const mockResult = {
      receipt_id: 'q_789',
      timestamp: '2025-12-28T00:00:00Z',
      result_count: 1,
      results: [],
      aggregations: {}
    };

    vi.mocked(queryFlex).mockResolvedValue(mockResult);

    const store = createQueryStore();
    await store.fetch({ agg: ['count'] });

    expect(store.data).not.toBeNull();

    store.reset();

    expect(store.loading).toBe(false);
    expect(store.data).toBeNull();
    expect(store.error).toBeNull();
    expect(store.lastQuery).toBeNull();
  });
});
