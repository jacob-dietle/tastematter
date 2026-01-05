/**
 * FilesStore Tests - Unified Data Architecture (Spec 08)
 *
 * TDD: Write tests FIRST (RED), then implement (GREEN)
 *
 * Purpose: Files view store that uses shared context for filtering
 * - Reads timeRange from context
 * - Reads selectedChain from context
 * - Refetches when context changes
 */
import { describe, test, expect, vi, beforeEach } from 'vitest';

// Mock the Tauri API before importing the store
vi.mock('$lib/api/tauri', () => ({
  queryFlex: vi.fn(),
  queryChains: vi.fn()
}));

import { createFilesStore } from '$lib/stores/files.svelte';
import { createContextStore, type ContextStore } from '$lib/stores/context.svelte';
import { queryFlex } from '$lib/api/tauri';
import type { QueryResult } from '$lib/types';

const mockQueryResult: QueryResult = {
  receipt_id: 'q_123',
  timestamp: '2025-12-28T00:00:00Z',
  result_count: 3,
  results: [
    { file_path: 'src/lib/store.ts', access_count: 15, last_access: '2025-12-29T15:00:00Z', session_count: 3 },
    { file_path: 'src/App.svelte', access_count: 10, last_access: '2025-12-29T14:00:00Z', session_count: 2 },
    { file_path: 'README.md', access_count: 2, last_access: '2025-12-28T10:00:00Z', session_count: 1 }
  ],
  aggregations: {
    count: { total_files: 3, total_accesses: 27 },
    recency: { newest: '2025-12-29T15:00:00Z', oldest: '2025-12-28T10:00:00Z' }
  }
};

// Create a mock context for testing
function createMockContext(): ContextStore {
  const ctx = createContextStore();
  return ctx;
}

describe('filesStore', () => {
  let mockContext: ContextStore;

  beforeEach(() => {
    vi.clearAllMocks();
    mockContext = createMockContext();
  });

  // Initial state tests
  describe('initial state', () => {
    test('starts with loading = false', () => {
      const store = createFilesStore(mockContext);
      expect(store.loading).toBe(false);
    });

    test('starts with data = null', () => {
      const store = createFilesStore(mockContext);
      expect(store.data).toBeNull();
    });

    test('starts with error = null', () => {
      const store = createFilesStore(mockContext);
      expect(store.error).toBeNull();
    });

    test('starts with sort = count', () => {
      const store = createFilesStore(mockContext);
      expect(store.sort).toBe('count');
    });

    test('starts with granularity = file', () => {
      const store = createFilesStore(mockContext);
      expect(store.granularity).toBe('file');
    });
  });

  // Context integration tests
  describe('context integration', () => {
    test('fetch uses context.timeRange', async () => {
      vi.mocked(queryFlex).mockResolvedValue(mockQueryResult);

      mockContext.setTimeRange('14d');
      const store = createFilesStore(mockContext);
      await store.fetch();

      expect(queryFlex).toHaveBeenCalledWith(
        expect.objectContaining({ time: '14d' })
      );
    });

    test('fetch uses context.selectedChain when set', async () => {
      vi.mocked(queryFlex).mockResolvedValue(mockQueryResult);

      mockContext.selectChain('7f389600');
      const store = createFilesStore(mockContext);
      await store.fetch();

      expect(queryFlex).toHaveBeenCalledWith(
        expect.objectContaining({ chain: '7f389600' })
      );
    });

    test('fetch omits chain when context.selectedChain is null', async () => {
      vi.mocked(queryFlex).mockResolvedValue(mockQueryResult);

      const store = createFilesStore(mockContext);
      await store.fetch();

      expect(queryFlex).toHaveBeenCalledWith(
        expect.objectContaining({ chain: undefined })
      );
    });

    test('fetch includes aggregations', async () => {
      vi.mocked(queryFlex).mockResolvedValue(mockQueryResult);

      const store = createFilesStore(mockContext);
      await store.fetch();

      expect(queryFlex).toHaveBeenCalledWith(
        expect.objectContaining({ agg: ['count', 'recency', 'sessions'] })
      );
    });
  });

  // Fetch behavior tests
  describe('fetch behavior', () => {
    test('sets loading true during fetch', async () => {
      vi.mocked(queryFlex).mockImplementation(() =>
        new Promise(resolve => setTimeout(() => resolve(mockQueryResult), 50))
      );

      const store = createFilesStore(mockContext);
      const fetchPromise = store.fetch();

      expect(store.loading).toBe(true);

      await fetchPromise;
      expect(store.loading).toBe(false);
    });

    test('populates data on success', async () => {
      vi.mocked(queryFlex).mockResolvedValue(mockQueryResult);

      const store = createFilesStore(mockContext);
      await store.fetch();

      expect(store.data).not.toBeNull();
      expect(store.data?.results).toHaveLength(3);
      expect(store.data?.results[0].file_path).toBe('src/lib/store.ts');
    });

    test('sets error on failure', async () => {
      const mockError = { code: 'INVOKE_ERROR', message: 'CLI failed' };
      vi.mocked(queryFlex).mockRejectedValue(mockError);

      const store = createFilesStore(mockContext);
      await store.fetch();

      expect(store.error).not.toBeNull();
      expect(store.error?.code).toBe('INVOKE_ERROR');
      expect(store.data).toBeNull();
    });

    test('clears error on successful fetch', async () => {
      const mockError = { code: 'INVOKE_ERROR', message: 'Failed' };

      vi.mocked(queryFlex).mockRejectedValueOnce(mockError);

      const store = createFilesStore(mockContext);
      await store.fetch();
      expect(store.error).not.toBeNull();

      vi.mocked(queryFlex).mockResolvedValueOnce(mockQueryResult);
      await store.fetch();

      expect(store.error).toBeNull();
      expect(store.data).not.toBeNull();
    });
  });

  // Sort behavior tests
  describe('sort behavior', () => {
    test('setSort updates sort to recency', () => {
      const store = createFilesStore(mockContext);
      store.setSort('recency');
      expect(store.sort).toBe('recency');
    });

    test('setSort updates sort to alpha', () => {
      const store = createFilesStore(mockContext);
      store.setSort('alpha');
      expect(store.sort).toBe('alpha');
    });

    test('fetch uses current sort value', async () => {
      vi.mocked(queryFlex).mockResolvedValue(mockQueryResult);

      const store = createFilesStore(mockContext);
      store.setSort('recency');
      await store.fetch();

      expect(queryFlex).toHaveBeenCalledWith(
        expect.objectContaining({ sort: 'recency' })
      );
    });
  });

  // Granularity tests
  describe('granularity behavior', () => {
    test('setGranularity updates to directory', () => {
      const store = createFilesStore(mockContext);
      store.setGranularity('directory');
      expect(store.granularity).toBe('directory');
    });

    test('setGranularity updates back to file', () => {
      const store = createFilesStore(mockContext);
      store.setGranularity('directory');
      store.setGranularity('file');
      expect(store.granularity).toBe('file');
    });
  });

  // Derived values tests
  describe('derived values', () => {
    test('files returns results from data', async () => {
      vi.mocked(queryFlex).mockResolvedValue(mockQueryResult);

      const store = createFilesStore(mockContext);
      await store.fetch();

      expect(store.files).toHaveLength(3);
      expect(store.files[0].access_count).toBe(15);
    });

    test('files returns empty array when no data', () => {
      const store = createFilesStore(mockContext);
      expect(store.files).toEqual([]);
    });

    test('totalFiles returns count from aggregations', async () => {
      vi.mocked(queryFlex).mockResolvedValue(mockQueryResult);

      const store = createFilesStore(mockContext);
      await store.fetch();

      expect(store.totalFiles).toBe(3);
    });

    test('totalAccesses returns count from aggregations', async () => {
      vi.mocked(queryFlex).mockResolvedValue(mockQueryResult);

      const store = createFilesStore(mockContext);
      await store.fetch();

      expect(store.totalAccesses).toBe(27);
    });

    test('maxAccessCount returns max from files', async () => {
      vi.mocked(queryFlex).mockResolvedValue(mockQueryResult);

      const store = createFilesStore(mockContext);
      await store.fetch();

      expect(store.maxAccessCount).toBe(15);
    });
  });
});
