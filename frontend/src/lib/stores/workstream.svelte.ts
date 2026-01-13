/**
 * WorkstreamStore - Unified Data Architecture (Spec 08)
 *
 * Workstream (Chain → Session → Files) hierarchy view:
 * - Uses context.chains for chain list
 * - Uses context.timeRange for session queries
 * - Lazy-loads sessions when chain is expanded
 * - Manages UI state (expanded chains, expanded sessions)
 */
import { querySessions } from '$lib/api';
import { QUERY_LIMITS } from '$lib/config';
import type { ContextStore } from './context.svelte';
import type { SessionData, ChainData, CommandError } from '$lib/types';

export function createWorkstreamStore(ctx: ContextStore) {
  // State
  let sessionsByChain = $state<Map<string, SessionData[]>>(new Map());
  let sessionsLoading = $state<Set<string>>(new Set());
  let sessionsError = $state<Map<string, CommandError>>(new Map());
  let expandedChains = $state<Set<string>>(new Set());
  let expandedSessions = $state<Set<string>>(new Set());

  // Chain expand/collapse
  async function toggleChainExpanded(chainId: string): Promise<void> {
    const newExpanded = new Set(expandedChains);

    if (newExpanded.has(chainId)) {
      newExpanded.delete(chainId);
    } else {
      newExpanded.add(chainId);

      // Lazy load sessions if not already loaded
      if (!sessionsByChain.has(chainId)) {
        await loadSessionsForChain(chainId);
      }
    }

    expandedChains = newExpanded;
  }

  function isChainExpanded(chainId: string): boolean {
    return expandedChains.has(chainId);
  }

  function collapseAllChains(): void {
    expandedChains = new Set();
  }

  async function expandAllChains(): Promise<void> {
    const allChainIds = ctx.chains.map(c => c.chain_id);
    const newExpanded = new Set(allChainIds);

    // Load sessions for all chains that aren't loaded yet
    const loadPromises = allChainIds
      .filter(id => !sessionsByChain.has(id))
      .map(id => loadSessionsForChain(id));

    await Promise.all(loadPromises);
    expandedChains = newExpanded;
  }

  // Session loading
  async function loadSessionsForChain(chainId: string): Promise<void> {
    // Mark as loading
    sessionsLoading = new Set([...sessionsLoading, chainId]);

    // Clear previous error
    const newErrors = new Map(sessionsError);
    newErrors.delete(chainId);
    sessionsError = newErrors;

    try {
      const result = await querySessions({
        time: ctx.timeRange,
        chain: chainId,
        limit: QUERY_LIMITS.sessions,
      });

      sessionsByChain = new Map([...sessionsByChain, [chainId, result.sessions]]);
    } catch (e) {
      const newErrors = new Map(sessionsError);
      newErrors.set(chainId, e as CommandError);
      sessionsError = newErrors;
    } finally {
      const newLoading = new Set(sessionsLoading);
      newLoading.delete(chainId);
      sessionsLoading = newLoading;
    }
  }

  function isChainLoading(chainId: string): boolean {
    return sessionsLoading.has(chainId);
  }

  function getSessionsForChain(chainId: string): SessionData[] {
    return sessionsByChain.get(chainId) ?? [];
  }

  function hasSessionsLoaded(chainId: string): boolean {
    return sessionsByChain.has(chainId);
  }

  function getChainError(chainId: string): CommandError | null {
    return sessionsError.get(chainId) ?? null;
  }

  async function retryLoadSessions(chainId: string): Promise<void> {
    await loadSessionsForChain(chainId);
  }

  // Session expand/collapse
  function toggleSessionExpanded(sessionId: string): void {
    const newExpanded = new Set(expandedSessions);
    if (newExpanded.has(sessionId)) {
      newExpanded.delete(sessionId);
    } else {
      newExpanded.add(sessionId);
    }
    expandedSessions = newExpanded;
  }

  function isSessionExpanded(sessionId: string): boolean {
    return expandedSessions.has(sessionId);
  }

  function collapseAllSessions(): void {
    expandedSessions = new Set();
  }

  // Refresh
  async function refreshChain(chainId: string): Promise<void> {
    await loadSessionsForChain(chainId);
  }

  async function refreshAllExpanded(): Promise<void> {
    const expandedIds = Array.from(expandedChains);
    await Promise.all(expandedIds.map(id => loadSessionsForChain(id)));
  }

  // Derived
  function getTotalLoadedSessions(): number {
    let total = 0;
    for (const sessions of sessionsByChain.values()) {
      total += sessions.length;
    }
    return total;
  }

  return {
    // Context passthrough
    get chains() { return ctx.chains; },
    get timeRange() { return ctx.timeRange; },
    get selectedChain() { return ctx.selectedChain; },

    // State getters
    get expandedChains() { return expandedChains; },
    get expandedSessions() { return expandedSessions; },
    get sessionsByChain() { return sessionsByChain; },
    get sessionsLoading() { return sessionsLoading; },
    get totalLoadedSessions() { return getTotalLoadedSessions(); },

    // Chain actions
    toggleChainExpanded,
    isChainExpanded,
    collapseAllChains,
    expandAllChains,

    // Session loading
    isChainLoading,
    getSessionsForChain,
    hasSessionsLoaded,
    getChainError,
    retryLoadSessions,

    // Session expand/collapse
    toggleSessionExpanded,
    isSessionExpanded,
    collapseAllSessions,

    // Refresh
    refreshChain,
    refreshAllExpanded,
  };
}

export type WorkstreamStore = ReturnType<typeof createWorkstreamStore>;
