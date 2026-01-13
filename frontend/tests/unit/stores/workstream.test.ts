/**
 * WorkstreamStore Tests - Unified Data Architecture (Spec 08)
 *
 * TDD: Write tests FIRST (RED), then implement (GREEN)
 *
 * Purpose: Workstream (Chain → Session → Files) hierarchy view
 * - Uses context.chains for chain list
 * - Uses context.timeRange for session queries
 * - Lazy-loads sessions when chain is expanded
 * - Manages UI state (expanded chains, expanded sessions)
 */
import { describe, test, expect, vi, beforeEach } from 'vitest';

// Mock the API before importing the store
vi.mock('$lib/api', () => ({
  querySessions: vi.fn(),
  queryChains: vi.fn()
}));

import { createWorkstreamStore } from '$lib/stores/workstream.svelte';
import { createContextStore, type ContextStore } from '$lib/stores/context.svelte';
import { querySessions, queryChains } from '$lib/api';
import type { SessionQueryResult, ChainQueryResult } from '$lib/types';

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
    }
  ],
  total_chains: 622
};

const mockSessionDataChain1: SessionQueryResult = {
  time_range: '7d',
  sessions: [
    {
      session_id: 'session-1a',
      chain_id: '7f389600',
      started_at: '2025-12-29T14:00:00Z',
      ended_at: '2025-12-29T16:00:00Z',
      duration_seconds: 7200,
      file_count: 8,
      total_accesses: 25,
      files: [
        { file_path: 'src/lib/store.ts', access_count: 10, access_types: ['read'], last_access: '2025-12-29T15:00:00Z' }
      ],
      top_files: [
        { file_path: 'src/lib/store.ts', access_count: 10, access_types: ['read'], last_access: '2025-12-29T15:00:00Z' }
      ]
    },
    {
      session_id: 'session-1b',
      chain_id: '7f389600',
      started_at: '2025-12-28T10:00:00Z',
      ended_at: '2025-12-28T12:00:00Z',
      duration_seconds: 7200,
      file_count: 4,
      total_accesses: 12,
      files: [
        { file_path: 'src/main.rs', access_count: 12, access_types: ['read', 'write'], last_access: '2025-12-28T11:30:00Z' }
      ],
      top_files: [
        { file_path: 'src/main.rs', access_count: 12, access_types: ['read', 'write'], last_access: '2025-12-28T11:30:00Z' }
      ]
    }
  ],
  chains: [],
  summary: {
    total_sessions: 2,
    total_files: 12,
    total_accesses: 37,
    active_chains: 1
  }
};

const mockSessionDataChain2: SessionQueryResult = {
  time_range: '7d',
  sessions: [
    {
      session_id: 'session-2a',
      chain_id: 'fa6b4bf6',
      started_at: '2025-12-19T10:00:00Z',
      ended_at: '2025-12-19T11:00:00Z',
      duration_seconds: 3600,
      file_count: 3,
      total_accesses: 8,
      files: [
        { file_path: 'README.md', access_count: 8, access_types: ['read'], last_access: '2025-12-19T10:30:00Z' }
      ],
      top_files: [
        { file_path: 'README.md', access_count: 8, access_types: ['read'], last_access: '2025-12-19T10:30:00Z' }
      ]
    }
  ],
  chains: [],
  summary: {
    total_sessions: 1,
    total_files: 3,
    total_accesses: 8,
    active_chains: 1
  }
};

// Create a mock context with chains loaded
async function createMockContextWithChains(): Promise<ContextStore> {
  vi.mocked(queryChains).mockResolvedValue(mockChainData);
  const ctx = createContextStore();
  await ctx.refreshChains();
  return ctx;
}

describe('workstreamStore', () => {
  let mockContext: ContextStore;

  beforeEach(async () => {
    vi.clearAllMocks();
    mockContext = await createMockContextWithChains();
  });

  // Initial state tests
  describe('initial state', () => {
    test('starts with no expanded chains', () => {
      const store = createWorkstreamStore(mockContext);
      expect(store.expandedChains.size).toBe(0);
    });

    test('starts with no expanded sessions', () => {
      const store = createWorkstreamStore(mockContext);
      expect(store.expandedSessions.size).toBe(0);
    });

    test('starts with empty sessionsByChain map', () => {
      const store = createWorkstreamStore(mockContext);
      expect(store.sessionsByChain.size).toBe(0);
    });

    test('starts with no loading chains', () => {
      const store = createWorkstreamStore(mockContext);
      expect(store.sessionsLoading.size).toBe(0);
    });
  });

  // Context integration tests
  describe('context integration', () => {
    test('chains getter returns context.chains', () => {
      const store = createWorkstreamStore(mockContext);
      expect(store.chains).toHaveLength(2);
      expect(store.chains[0].chain_id).toBe('7f389600');
    });

    test('timeRange getter returns context.timeRange', () => {
      mockContext.setTimeRange('30d');
      const store = createWorkstreamStore(mockContext);
      expect(store.timeRange).toBe('30d');
    });

    test('selectedChain getter returns context.selectedChain', () => {
      mockContext.selectChain('7f389600');
      const store = createWorkstreamStore(mockContext);
      expect(store.selectedChain).toBe('7f389600');
    });
  });

  // Chain expand/collapse tests
  describe('chain expand/collapse', () => {
    test('toggleChainExpanded adds chain to expanded set', async () => {
      vi.mocked(querySessions).mockResolvedValue(mockSessionDataChain1);

      const store = createWorkstreamStore(mockContext);
      await store.toggleChainExpanded('7f389600');

      expect(store.isChainExpanded('7f389600')).toBe(true);
    });

    test('toggleChainExpanded removes chain from expanded set', async () => {
      vi.mocked(querySessions).mockResolvedValue(mockSessionDataChain1);

      const store = createWorkstreamStore(mockContext);
      await store.toggleChainExpanded('7f389600');
      await store.toggleChainExpanded('7f389600');

      expect(store.isChainExpanded('7f389600')).toBe(false);
    });

    test('collapseAllChains clears all expanded chains', async () => {
      vi.mocked(querySessions).mockResolvedValue(mockSessionDataChain1);

      const store = createWorkstreamStore(mockContext);
      await store.toggleChainExpanded('7f389600');
      await store.toggleChainExpanded('fa6b4bf6');

      store.collapseAllChains();

      expect(store.expandedChains.size).toBe(0);
    });

    test('expandAllChains expands all chains from context', async () => {
      vi.mocked(querySessions).mockResolvedValue(mockSessionDataChain1);

      const store = createWorkstreamStore(mockContext);
      await store.expandAllChains();

      expect(store.isChainExpanded('7f389600')).toBe(true);
      expect(store.isChainExpanded('fa6b4bf6')).toBe(true);
    });
  });

  // Lazy loading sessions tests
  describe('lazy loading sessions', () => {
    test('toggleChainExpanded fetches sessions on first expand', async () => {
      vi.mocked(querySessions).mockResolvedValue(mockSessionDataChain1);

      const store = createWorkstreamStore(mockContext);
      await store.toggleChainExpanded('7f389600');

      expect(querySessions).toHaveBeenCalledWith(
        expect.objectContaining({
          chain: '7f389600',
          time: '7d'
        })
      );
    });

    test('toggleChainExpanded does not refetch on subsequent expand', async () => {
      vi.mocked(querySessions).mockResolvedValue(mockSessionDataChain1);

      const store = createWorkstreamStore(mockContext);
      await store.toggleChainExpanded('7f389600'); // expand (fetches)
      await store.toggleChainExpanded('7f389600'); // collapse
      await store.toggleChainExpanded('7f389600'); // expand again

      expect(querySessions).toHaveBeenCalledTimes(1);
    });

    test('sessions are stored in sessionsByChain map', async () => {
      vi.mocked(querySessions).mockResolvedValue(mockSessionDataChain1);

      const store = createWorkstreamStore(mockContext);
      await store.toggleChainExpanded('7f389600');

      const sessions = store.getSessionsForChain('7f389600');
      expect(sessions).toHaveLength(2);
      expect(sessions[0].session_id).toBe('session-1a');
    });

    test('loading state tracks which chains are loading', async () => {
      vi.mocked(querySessions).mockImplementation(() =>
        new Promise(resolve => setTimeout(() => resolve(mockSessionDataChain1), 50))
      );

      const store = createWorkstreamStore(mockContext);
      const expandPromise = store.toggleChainExpanded('7f389600');

      expect(store.isChainLoading('7f389600')).toBe(true);

      await expandPromise;
      expect(store.isChainLoading('7f389600')).toBe(false);
    });

    test('different chains can load independently', async () => {
      vi.mocked(querySessions)
        .mockResolvedValueOnce(mockSessionDataChain1)
        .mockResolvedValueOnce(mockSessionDataChain2);

      const store = createWorkstreamStore(mockContext);
      await store.toggleChainExpanded('7f389600');
      await store.toggleChainExpanded('fa6b4bf6');

      expect(store.getSessionsForChain('7f389600')).toHaveLength(2);
      expect(store.getSessionsForChain('fa6b4bf6')).toHaveLength(1);
    });
  });

  // Session expand/collapse tests
  describe('session expand/collapse', () => {
    test('toggleSessionExpanded adds session to expanded set', () => {
      const store = createWorkstreamStore(mockContext);
      store.toggleSessionExpanded('session-1a');

      expect(store.isSessionExpanded('session-1a')).toBe(true);
    });

    test('toggleSessionExpanded removes session from expanded set', () => {
      const store = createWorkstreamStore(mockContext);
      store.toggleSessionExpanded('session-1a');
      store.toggleSessionExpanded('session-1a');

      expect(store.isSessionExpanded('session-1a')).toBe(false);
    });

    test('collapseAllSessions clears all expanded sessions', () => {
      const store = createWorkstreamStore(mockContext);
      store.toggleSessionExpanded('session-1a');
      store.toggleSessionExpanded('session-1b');

      store.collapseAllSessions();

      expect(store.expandedSessions.size).toBe(0);
    });
  });

  // Error handling tests
  describe('error handling', () => {
    test('stores error when session fetch fails', async () => {
      const mockError = { code: 'SESSION_ERROR', message: 'Failed' };
      vi.mocked(querySessions).mockRejectedValue(mockError);

      const store = createWorkstreamStore(mockContext);
      await store.toggleChainExpanded('7f389600');

      const error = store.getChainError('7f389600');
      expect(error).not.toBeNull();
      expect(error?.code).toBe('SESSION_ERROR');
    });

    test('chain is still expanded even on error', async () => {
      const mockError = { code: 'SESSION_ERROR', message: 'Failed' };
      vi.mocked(querySessions).mockRejectedValue(mockError);

      const store = createWorkstreamStore(mockContext);
      await store.toggleChainExpanded('7f389600');

      expect(store.isChainExpanded('7f389600')).toBe(true);
    });

    test('retry fetches sessions again', async () => {
      const mockError = { code: 'SESSION_ERROR', message: 'Failed' };
      vi.mocked(querySessions).mockRejectedValueOnce(mockError);
      vi.mocked(querySessions).mockResolvedValueOnce(mockSessionDataChain1);

      const store = createWorkstreamStore(mockContext);
      await store.toggleChainExpanded('7f389600');

      expect(store.getChainError('7f389600')).not.toBeNull();

      await store.retryLoadSessions('7f389600');

      expect(store.getChainError('7f389600')).toBeNull();
      expect(store.getSessionsForChain('7f389600')).toHaveLength(2);
    });
  });

  // Refresh behavior tests
  describe('refresh behavior', () => {
    test('refreshChain refetches sessions for chain', async () => {
      vi.mocked(querySessions).mockResolvedValue(mockSessionDataChain1);

      const store = createWorkstreamStore(mockContext);
      await store.toggleChainExpanded('7f389600');

      vi.clearAllMocks();
      vi.mocked(querySessions).mockResolvedValue(mockSessionDataChain1);

      await store.refreshChain('7f389600');

      expect(querySessions).toHaveBeenCalledTimes(1);
    });

    test('refreshAllExpanded refetches all expanded chains', async () => {
      vi.mocked(querySessions)
        .mockResolvedValueOnce(mockSessionDataChain1)
        .mockResolvedValueOnce(mockSessionDataChain2);

      const store = createWorkstreamStore(mockContext);
      await store.toggleChainExpanded('7f389600');
      await store.toggleChainExpanded('fa6b4bf6');

      vi.clearAllMocks();
      vi.mocked(querySessions)
        .mockResolvedValueOnce(mockSessionDataChain1)
        .mockResolvedValueOnce(mockSessionDataChain2);

      await store.refreshAllExpanded();

      expect(querySessions).toHaveBeenCalledTimes(2);
    });
  });

  // Derived values tests
  describe('derived values', () => {
    test('totalSessions returns sum across all loaded chains', async () => {
      vi.mocked(querySessions)
        .mockResolvedValueOnce(mockSessionDataChain1)
        .mockResolvedValueOnce(mockSessionDataChain2);

      const store = createWorkstreamStore(mockContext);
      await store.toggleChainExpanded('7f389600');
      await store.toggleChainExpanded('fa6b4bf6');

      expect(store.totalLoadedSessions).toBe(3); // 2 + 1
    });

    test('getSessionsForChain returns empty array if not loaded', () => {
      const store = createWorkstreamStore(mockContext);
      expect(store.getSessionsForChain('unknown')).toEqual([]);
    });

    test('hasSessionsLoaded returns true after fetch', async () => {
      vi.mocked(querySessions).mockResolvedValue(mockSessionDataChain1);

      const store = createWorkstreamStore(mockContext);
      expect(store.hasSessionsLoaded('7f389600')).toBe(false);

      await store.toggleChainExpanded('7f389600');
      expect(store.hasSessionsLoaded('7f389600')).toBe(true);
    });
  });
});
