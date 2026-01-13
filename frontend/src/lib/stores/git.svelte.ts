import { gitStatus, gitPull, gitPush } from '$lib/api';
import type { GitStatus, GitOpResult, CommandError } from '$lib/types';

export function createGitStore() {
  // State
  let loading = $state(false);
  let data = $state<GitStatus | null>(null);
  let error = $state<CommandError | null>(null);
  let isPulling = $state(false);
  let isPushing = $state(false);
  let lastOperation = $state<GitOpResult | null>(null);

  // Actions
  async function fetchStatus() {
    loading = true;
    error = null;

    try {
      data = await gitStatus();
    } catch (e) {
      error = e as CommandError;
      data = null;
    } finally {
      loading = false;
    }
  }

  async function pull() {
    isPulling = true;
    lastOperation = null;

    try {
      lastOperation = await gitPull();
      if (lastOperation.success) {
        // Refresh status in background (don't block UI)
        fetchStatus();
      }
    } catch (e) {
      lastOperation = {
        success: false,
        message: 'Pull failed',
        error: (e as Error).message
      };
    } finally {
      isPulling = false;
    }
  }

  async function push() {
    isPushing = true;
    lastOperation = null;

    try {
      lastOperation = await gitPush();
      if (lastOperation.success) {
        // Refresh status in background (don't block UI)
        fetchStatus();
      }
    } catch (e) {
      lastOperation = {
        success: false,
        message: 'Push failed',
        error: (e as Error).message
      };
    } finally {
      isPushing = false;
    }
  }

  function clearLastOperation() {
    lastOperation = null;
  }

  return {
    // Getters for state
    get loading() { return loading; },
    get data() { return data; },
    get error() { return error; },
    get isPulling() { return isPulling; },
    get isPushing() { return isPushing; },
    get lastOperation() { return lastOperation; },

    // Derived state
    get canPull() { return (data?.behind ?? 0) > 0; },
    get canPush() { return (data?.ahead ?? 0) > 0; },
    get hasChanges() {
      return (data?.staged.length ?? 0) +
             (data?.modified.length ?? 0) +
             (data?.untracked.length ?? 0) > 0;
    },

    // Actions
    fetchStatus,
    pull,
    push,
    clearLastOperation
  };
}
