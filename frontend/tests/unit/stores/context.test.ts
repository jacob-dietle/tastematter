/**
 * ContextProvider Store Tests - Unified Data Architecture (Spec 08)
 *
 * TDD: Write tests FIRST (RED), then implement (GREEN)
 *
 * Purpose: Shared global state for all views
 * - timeRange: Global time filter ('7d' | '14d' | '30d')
 * - selectedChain: Global chain filter (string | null)
 * - chains: Always-loaded chain list for navigation
 */
import { describe, test, expect, vi, beforeEach } from 'vitest';

// Mock the API before importing the store
vi.mock('$lib/api', () => ({
  queryChains: vi.fn()
}));

import { createContextStore } from '$lib/stores/context.svelte';
import { queryChains } from '$lib/api';
import type { ChainQueryResult } from '$lib/types';

const mockChainData: ChainQueryResult = {
  chains: [
    {
      chain_id: '7f389600',
      session_count: 103,
      file_count: 669,
      time_range: {
        start: '2025-12-11T04:31:05.507000+00:00',
        end: '2026-01-05T01:03:16.750000+00:00'
      }
    },
    {
      chain_id: 'fa6b4bf6',
      session_count: 21,
      file_count: 193,
      time_range: {
        start: '2025-12-07T23:32:17.397000+00:00',
        end: '2025-12-19T23:01:24.564000+00:00'
      }
    },
    {
      chain_id: 'abc12345',
      session_count: 5,
      file_count: 42,
      time_range: null
    }
  ],
  total_chains: 622
};

describe('contextStore', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // Initial state tests
  describe('initial state', () => {
    test('starts with timeRange = 7d', () => {
      const store = createContextStore();
      expect(store.timeRange).toBe('7d');
    });

    test('starts with selectedChain = null', () => {
      const store = createContextStore();
      expect(store.selectedChain).toBeNull();
    });

    test('starts with empty chains array', () => {
      const store = createContextStore();
      expect(store.chains).toEqual([]);
    });

    test('starts with chainsLoading = false', () => {
      const store = createContextStore();
      expect(store.chainsLoading).toBe(false);
    });

    test('starts with chainsError = null', () => {
      const store = createContextStore();
      expect(store.chainsError).toBeNull();
    });
  });

  // Time range management tests
  describe('time range management', () => {
    test('setTimeRange updates timeRange to 14d', () => {
      const store = createContextStore();
      store.setTimeRange('14d');
      expect(store.timeRange).toBe('14d');
    });

    test('setTimeRange updates timeRange to 30d', () => {
      const store = createContextStore();
      store.setTimeRange('30d');
      expect(store.timeRange).toBe('30d');
    });

    test('setTimeRange can change back to 7d', () => {
      const store = createContextStore();
      store.setTimeRange('30d');
      store.setTimeRange('7d');
      expect(store.timeRange).toBe('7d');
    });
  });

  // Chain selection tests
  describe('chain selection', () => {
    test('selectChain sets selectedChain', () => {
      const store = createContextStore();
      store.selectChain('7f389600');
      expect(store.selectedChain).toBe('7f389600');
    });

    test('selectChain with null clears selection', () => {
      const store = createContextStore();
      store.selectChain('7f389600');
      store.selectChain(null);
      expect(store.selectedChain).toBeNull();
    });

    test('clearChain sets selectedChain to null', () => {
      const store = createContextStore();
      store.selectChain('7f389600');
      store.clearChain();
      expect(store.selectedChain).toBeNull();
    });

    test('selecting a different chain updates selection', () => {
      const store = createContextStore();
      store.selectChain('7f389600');
      store.selectChain('fa6b4bf6');
      expect(store.selectedChain).toBe('fa6b4bf6');
    });
  });

  // Chain data fetching tests
  describe('chain data fetching', () => {
    test('refreshChains sets loading true during fetch', async () => {
      vi.mocked(queryChains).mockImplementation(() =>
        new Promise(resolve => setTimeout(() => resolve(mockChainData), 50))
      );

      const store = createContextStore();
      const fetchPromise = store.refreshChains();

      expect(store.chainsLoading).toBe(true);

      await fetchPromise;
      expect(store.chainsLoading).toBe(false);
    });

    test('refreshChains populates chains on success', async () => {
      vi.mocked(queryChains).mockResolvedValue(mockChainData);

      const store = createContextStore();
      await store.refreshChains();

      expect(store.chains).toHaveLength(3);
      expect(store.chains[0].chain_id).toBe('7f389600');
      expect(store.chains[0].session_count).toBe(103);
    });

    test('refreshChains sets error on failure', async () => {
      const mockError = {
        code: 'CHAIN_ERROR',
        message: 'Failed to fetch chains'
      };

      vi.mocked(queryChains).mockRejectedValue(mockError);

      const store = createContextStore();
      await store.refreshChains();

      expect(store.chainsError).not.toBeNull();
      expect(store.chainsError?.code).toBe('CHAIN_ERROR');
      expect(store.chains).toEqual([]);
    });

    test('refreshChains clears previous error on success', async () => {
      const mockError = { code: 'CHAIN_ERROR', message: 'Failed' };

      // First call fails
      vi.mocked(queryChains).mockRejectedValueOnce(mockError);

      const store = createContextStore();
      await store.refreshChains();
      expect(store.chainsError).not.toBeNull();

      // Second call succeeds
      vi.mocked(queryChains).mockResolvedValueOnce(mockChainData);
      await store.refreshChains();

      expect(store.chainsError).toBeNull();
      expect(store.chains).toHaveLength(3);
    });

    test('refreshChains calls queryChains with limit', async () => {
      vi.mocked(queryChains).mockResolvedValue(mockChainData);

      const store = createContextStore();
      await store.refreshChains();

      expect(queryChains).toHaveBeenCalledWith({ limit: 50 });
    });
  });

  // Derived values tests
  describe('derived values', () => {
    test('totalChains returns correct count', async () => {
      vi.mocked(queryChains).mockResolvedValue(mockChainData);

      const store = createContextStore();
      await store.refreshChains();

      expect(store.totalChains).toBe(622);
    });

    test('getChainById returns correct chain', async () => {
      vi.mocked(queryChains).mockResolvedValue(mockChainData);

      const store = createContextStore();
      await store.refreshChains();

      const chain = store.getChainById('fa6b4bf6');
      expect(chain?.session_count).toBe(21);
      expect(chain?.file_count).toBe(193);
    });

    test('getChainById returns undefined for unknown id', async () => {
      vi.mocked(queryChains).mockResolvedValue(mockChainData);

      const store = createContextStore();
      await store.refreshChains();

      const chain = store.getChainById('unknown');
      expect(chain).toBeUndefined();
    });

    test('selectedChainData returns chain details when selected', async () => {
      vi.mocked(queryChains).mockResolvedValue(mockChainData);

      const store = createContextStore();
      await store.refreshChains();
      store.selectChain('7f389600');

      expect(store.selectedChainData?.session_count).toBe(103);
    });

    test('selectedChainData returns null when no chain selected', async () => {
      vi.mocked(queryChains).mockResolvedValue(mockChainData);

      const store = createContextStore();
      await store.refreshChains();

      expect(store.selectedChainData).toBeNull();
    });
  });
});
