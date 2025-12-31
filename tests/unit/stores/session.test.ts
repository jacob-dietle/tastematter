import { describe, test, expect, vi, beforeEach } from 'vitest';

// Mock the Tauri API before importing the store
vi.mock('$lib/api/tauri', () => ({
  querySessions: vi.fn()
}));

import { createSessionStore } from '$lib/stores/session.svelte';
import { querySessions } from '$lib/api/tauri';
import type { SessionQueryResult } from '$lib/types';

const mockSessionData: SessionQueryResult = {
  time_range: '7d',
  sessions: [
    {
      session_id: 'abc123def456',
      chain_id: 'chain-1',
      started_at: '2025-12-29T14:00:00Z',
      ended_at: '2025-12-29T16:00:00Z',
      duration_seconds: 7200,
      file_count: 8,
      total_accesses: 25,
      files: [
        { file_path: 'src/lib/store.ts', access_count: 10, access_types: ['read'], last_access: '2025-12-29T15:00:00Z' },
        { file_path: 'src/App.svelte', access_count: 8, access_types: ['read', 'write'], last_access: '2025-12-29T15:30:00Z' },
        { file_path: 'tests/test.ts', access_count: 7, access_types: ['read'], last_access: '2025-12-29T15:45:00Z' }
      ],
      top_files: [
        { file_path: 'src/lib/store.ts', access_count: 10, access_types: ['read'], last_access: '2025-12-29T15:00:00Z' },
        { file_path: 'src/App.svelte', access_count: 8, access_types: ['read', 'write'], last_access: '2025-12-29T15:30:00Z' },
        { file_path: 'tests/test.ts', access_count: 7, access_types: ['read'], last_access: '2025-12-29T15:45:00Z' }
      ]
    },
    {
      session_id: 'xyz789uvw',
      chain_id: 'chain-2',
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
  chains: [
    { chain_id: 'chain-1', session_count: 1, file_count: 8, last_active: '2025-12-29T16:00:00Z' },
    { chain_id: 'chain-2', session_count: 1, file_count: 4, last_active: '2025-12-28T12:00:00Z' }
  ],
  summary: {
    total_sessions: 2,
    total_files: 12,
    total_accesses: 37,
    active_chains: 2
  }
};

describe('sessionStore', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // Initial state tests (5)
  describe('initial state', () => {
    test('starts with loading false', () => {
      const store = createSessionStore();
      expect(store.loading).toBe(false);
    });

    test('starts with no data', () => {
      const store = createSessionStore();
      expect(store.data).toBeNull();
    });

    test('starts with 7d selected', () => {
      const store = createSessionStore();
      expect(store.selectedRange).toBe('7d');
    });

    test('starts with no expanded sessions', () => {
      const store = createSessionStore();
      expect(store.expandedSessions.size).toBe(0);
    });

    test('starts with no chain filter', () => {
      const store = createSessionStore();
      expect(store.selectedChain).toBeNull();
    });
  });

  // Fetch behavior tests (4)
  describe('fetch behavior', () => {
    test('sets loading true during fetch', async () => {
      vi.mocked(querySessions).mockImplementation(() =>
        new Promise(resolve => setTimeout(() => resolve(mockSessionData), 50))
      );

      const store = createSessionStore();
      const fetchPromise = store.fetch();

      // Check loading is true immediately
      expect(store.loading).toBe(true);

      await fetchPromise;
      expect(store.loading).toBe(false);
    });

    test('stores data on success', async () => {
      vi.mocked(querySessions).mockResolvedValue(mockSessionData);

      const store = createSessionStore();
      await store.fetch();

      expect(store.data).not.toBeNull();
      expect(store.data?.time_range).toBe('7d');
      expect(store.data?.sessions.length).toBe(2);
      expect(store.error).toBeNull();
    });

    test('stores error on failure', async () => {
      const mockError = {
        code: 'SESSION_ERROR',
        message: 'Failed to fetch session data'
      };

      vi.mocked(querySessions).mockRejectedValue(mockError);

      const store = createSessionStore();
      await store.fetch();

      expect(store.error).not.toBeNull();
      expect(store.error?.code).toBe('SESSION_ERROR');
      expect(store.data).toBeNull();
    });

    test('setRange triggers fetch with new range', async () => {
      vi.mocked(querySessions).mockResolvedValue(mockSessionData);

      const store = createSessionStore();
      await store.setRange('14d');

      expect(store.selectedRange).toBe('14d');
      expect(querySessions).toHaveBeenCalledWith(expect.objectContaining({ time: '14d' }));
    });
  });

  // Expand/collapse tests (3)
  describe('expand/collapse', () => {
    test('toggleSessionExpanded adds session to set', () => {
      const store = createSessionStore();
      store.toggleSessionExpanded('abc123');
      expect(store.isExpanded('abc123')).toBe(true);
    });

    test('toggleSessionExpanded removes session from set', () => {
      const store = createSessionStore();
      store.toggleSessionExpanded('abc123');
      store.toggleSessionExpanded('abc123');
      expect(store.isExpanded('abc123')).toBe(false);
    });

    test('collapseAll clears all expanded', () => {
      const store = createSessionStore();
      store.toggleSessionExpanded('abc123');
      store.toggleSessionExpanded('def456');
      store.collapseAll();
      expect(store.expandedSessions.size).toBe(0);
    });
  });

  // Chain filter tests (2)
  describe('chain filter', () => {
    test('setChainFilter updates selectedChain', async () => {
      vi.mocked(querySessions).mockResolvedValue(mockSessionData);

      const store = createSessionStore();
      await store.setChainFilter('chain-1');
      expect(store.selectedChain).toBe('chain-1');
    });

    test('setChainFilter with null clears filter', async () => {
      vi.mocked(querySessions).mockResolvedValue(mockSessionData);

      const store = createSessionStore();
      await store.setChainFilter('chain-1');
      await store.setChainFilter(null);
      expect(store.selectedChain).toBeNull();
    });
  });

  // Derived values tests (2)
  describe('derived values', () => {
    test('maxAccessCount returns max from all session files', async () => {
      vi.mocked(querySessions).mockResolvedValue(mockSessionData);

      const store = createSessionStore();
      await store.fetch();

      // Max access count is 12 (from src/main.rs in session xyz789uvw)
      expect(store.maxAccessCount).toBe(12);
    });

    test('filteredSessions returns all when no chain filter', async () => {
      vi.mocked(querySessions).mockResolvedValue(mockSessionData);

      const store = createSessionStore();
      await store.fetch();

      expect(store.filteredSessions.length).toBe(2);
    });
  });
});
