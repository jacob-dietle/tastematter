/**
 * ContextProvider Store - Unified Data Architecture (Spec 08)
 *
 * Shared global state for all views:
 * - timeRange: Global time filter ('7d' | '14d' | '30d')
 * - selectedChain: Global chain filter (string | null)
 * - chains: Always-loaded chain list for navigation
 */
import { getContext, setContext } from 'svelte';
import { queryChains } from '$lib/api/tauri';
import type { ChainData, ChainQueryResult, CommandError } from '$lib/types';

const CONTEXT_KEY = Symbol('app-context');

export function createContextStore() {
  // State
  let timeRange = $state<'7d' | '14d' | '30d'>('7d');
  let selectedChain = $state<string | null>(null);
  let chains = $state<ChainData[]>([]);
  let totalChainsCount = $state(0);
  let chainsLoading = $state(false);
  let chainsError = $state<CommandError | null>(null);

  // Request deduplication: ignore stale responses from superseded requests
  let currentChainsRequestId = 0;

  // Actions
  async function refreshChains() {
    const requestId = ++currentChainsRequestId;
    chainsLoading = true;
    chainsError = null;
    try {
      const result = await queryChains({ limit: 50 });
      // Only update state if this is still the current request
      if (requestId === currentChainsRequestId) {
        chains = result.chains;
        totalChainsCount = result.total_chains;
      }
    } catch (e) {
      if (requestId === currentChainsRequestId) {
        chainsError = e as CommandError;
        chains = [];
        totalChainsCount = 0;
      }
    } finally {
      if (requestId === currentChainsRequestId) {
        chainsLoading = false;
      }
    }
  }

  function setTimeRange(range: '7d' | '14d' | '30d') {
    timeRange = range;
  }

  function selectChain(chainId: string | null) {
    selectedChain = chainId;
  }

  function clearChain() {
    selectedChain = null;
  }

  // Derived
  function getChainById(chainId: string): ChainData | undefined {
    return chains.find(c => c.chain_id === chainId);
  }

  function getSelectedChainData(): ChainData | null {
    if (!selectedChain) return null;
    return getChainById(selectedChain) ?? null;
  }

  function getTotalChains(): number {
    return totalChainsCount;
  }

  return {
    // State getters
    get timeRange() { return timeRange; },
    get selectedChain() { return selectedChain; },
    get chains() { return chains; },
    get chainsLoading() { return chainsLoading; },
    get chainsError() { return chainsError; },
    get totalChains() { return getTotalChains(); },
    get selectedChainData() { return getSelectedChainData(); },

    // Actions
    setTimeRange,
    selectChain,
    clearChain,
    refreshChains,
    getChainById,
  };
}

export type ContextStore = ReturnType<typeof createContextStore>;

export function setAppContext(ctx: ContextStore) {
  setContext(CONTEXT_KEY, ctx);
}

export function getAppContext(): ContextStore {
  return getContext(CONTEXT_KEY);
}
