import { describe, test, expect, vi, beforeEach } from 'vitest';

// Mock the API before importing the store
// Git operations are re-exported from $lib/api (which delegates to tauri.ts)
vi.mock('$lib/api', () => ({
  gitStatus: vi.fn(),
  gitPull: vi.fn(),
  gitPush: vi.fn()
}));

import { createGitStore } from '$lib/stores/git.svelte';
import { gitStatus, gitPull, gitPush } from '$lib/api';
import type { GitStatus, GitOpResult } from '$lib/types';

const mockStatus: GitStatus = {
  branch: 'main',
  ahead: 2,
  behind: 1,
  staged: ['file1.ts'],
  modified: ['file2.ts'],
  untracked: ['file3.ts'],
  has_conflicts: false
};

describe('gitStore', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('initial state', () => {
    test('initial state is empty', () => {
      const store = createGitStore();
      expect(store.loading).toBe(false);
      expect(store.data).toBeNull();
      expect(store.error).toBeNull();
      expect(store.isPulling).toBe(false);
      expect(store.isPushing).toBe(false);
      expect(store.lastOperation).toBeNull();
    });
  });

  describe('fetchStatus', () => {
    test('sets loading true during fetch', async () => {
      vi.mocked(gitStatus).mockImplementation(
        () => new Promise(resolve => setTimeout(() => resolve(mockStatus), 50))
      );

      const store = createGitStore();
      const fetchPromise = store.fetchStatus();

      // Loading should be true immediately
      expect(store.loading).toBe(true);

      await fetchPromise;
      expect(store.loading).toBe(false);
    });

    test('stores data on success', async () => {
      vi.mocked(gitStatus).mockResolvedValue(mockStatus);

      const store = createGitStore();
      await store.fetchStatus();

      expect(store.data).toEqual(mockStatus);
      expect(store.error).toBeNull();
    });

    test('stores error on failure', async () => {
      const error = { code: 'GIT_ERROR', message: 'Not a git repository' };
      vi.mocked(gitStatus).mockRejectedValue(error);

      const store = createGitStore();
      await store.fetchStatus();

      expect(store.data).toBeNull();
      expect(store.error).toEqual(error);
    });
  });

  describe('derived state', () => {
    test('canPull is true when behind > 0', async () => {
      vi.mocked(gitStatus).mockResolvedValue({ ...mockStatus, behind: 3 });
      const store = createGitStore();
      await store.fetchStatus();
      expect(store.canPull).toBe(true);
    });

    test('canPull is false when behind = 0', async () => {
      vi.mocked(gitStatus).mockResolvedValue({ ...mockStatus, behind: 0 });
      const store = createGitStore();
      await store.fetchStatus();
      expect(store.canPull).toBe(false);
    });

    test('canPush is true when ahead > 0', async () => {
      vi.mocked(gitStatus).mockResolvedValue({ ...mockStatus, ahead: 2 });
      const store = createGitStore();
      await store.fetchStatus();
      expect(store.canPush).toBe(true);
    });

    test('canPush is false when ahead = 0', async () => {
      vi.mocked(gitStatus).mockResolvedValue({ ...mockStatus, ahead: 0 });
      const store = createGitStore();
      await store.fetchStatus();
      expect(store.canPush).toBe(false);
    });

    test('hasChanges is true when files exist', async () => {
      vi.mocked(gitStatus).mockResolvedValue(mockStatus);
      const store = createGitStore();
      await store.fetchStatus();
      expect(store.hasChanges).toBe(true);
    });

    test('hasChanges is false when no files', async () => {
      vi.mocked(gitStatus).mockResolvedValue({
        ...mockStatus,
        staged: [],
        modified: [],
        untracked: []
      });
      const store = createGitStore();
      await store.fetchStatus();
      expect(store.hasChanges).toBe(false);
    });
  });

  describe('pull', () => {
    test('sets isPulling during operation', async () => {
      vi.mocked(gitStatus).mockResolvedValue(mockStatus);
      vi.mocked(gitPull).mockImplementation(
        () => new Promise(resolve => setTimeout(() => resolve({ success: true, message: 'OK' }), 50))
      );

      const store = createGitStore();
      await store.fetchStatus();
      const pullPromise = store.pull();

      expect(store.isPulling).toBe(true);

      await pullPromise;
      expect(store.isPulling).toBe(false);
    });

    test('refreshes status after successful pull', async () => {
      vi.mocked(gitStatus).mockResolvedValue(mockStatus);
      vi.mocked(gitPull).mockResolvedValue({ success: true, message: 'Pulled' });

      const store = createGitStore();
      await store.fetchStatus();
      vi.mocked(gitStatus).mockClear();

      await store.pull();

      expect(gitStatus).toHaveBeenCalled();
    });

    test('stores lastOperation result', async () => {
      const result: GitOpResult = { success: true, message: 'Pulled 3 commits' };
      vi.mocked(gitStatus).mockResolvedValue(mockStatus);
      vi.mocked(gitPull).mockResolvedValue(result);

      const store = createGitStore();
      await store.fetchStatus();
      await store.pull();

      expect(store.lastOperation).toEqual(result);
    });

    test('handles pull failure', async () => {
      vi.mocked(gitStatus).mockResolvedValue(mockStatus);
      vi.mocked(gitPull).mockResolvedValue({
        success: false,
        message: 'Pull failed',
        error: 'non-fast-forward'
      });

      const store = createGitStore();
      await store.fetchStatus();
      await store.pull();

      expect(store.lastOperation?.success).toBe(false);
      expect(store.lastOperation?.error).toBe('non-fast-forward');
    });
  });

  describe('push', () => {
    test('sets isPushing during operation', async () => {
      vi.mocked(gitStatus).mockResolvedValue(mockStatus);
      vi.mocked(gitPush).mockImplementation(
        () => new Promise(resolve => setTimeout(() => resolve({ success: true, message: 'OK' }), 50))
      );

      const store = createGitStore();
      await store.fetchStatus();
      const pushPromise = store.push();

      expect(store.isPushing).toBe(true);

      await pushPromise;
      expect(store.isPushing).toBe(false);
    });

    test('refreshes status after successful push', async () => {
      vi.mocked(gitStatus).mockResolvedValue(mockStatus);
      vi.mocked(gitPush).mockResolvedValue({ success: true, message: 'Pushed' });

      const store = createGitStore();
      await store.fetchStatus();
      vi.mocked(gitStatus).mockClear();

      await store.push();

      expect(gitStatus).toHaveBeenCalled();
    });

    test('handles push failure', async () => {
      vi.mocked(gitStatus).mockResolvedValue(mockStatus);
      vi.mocked(gitPush).mockResolvedValue({
        success: false,
        message: 'Push failed',
        error: 'remote rejected'
      });

      const store = createGitStore();
      await store.fetchStatus();
      await store.push();

      expect(store.lastOperation?.success).toBe(false);
      expect(store.lastOperation?.error).toBe('remote rejected');
    });
  });

  describe('clearLastOperation', () => {
    test('clears lastOperation', async () => {
      vi.mocked(gitStatus).mockResolvedValue(mockStatus);
      vi.mocked(gitPull).mockResolvedValue({ success: true, message: 'OK' });

      const store = createGitStore();
      await store.fetchStatus();
      await store.pull();
      expect(store.lastOperation).not.toBeNull();

      store.clearLastOperation();
      expect(store.lastOperation).toBeNull();
    });
  });
});
