import { querySessions } from '$lib/api/tauri';
import type { SessionQueryResult, CommandError } from '$lib/types';

export function createSessionStore() {
  // State
  let loading = $state(false);
  let data = $state<SessionQueryResult | null>(null);
  let error = $state<CommandError | null>(null);
  let selectedRange = $state<'7d' | '14d' | '30d'>('7d');
  let expandedSessions = $state<Set<string>>(new Set());
  let selectedChain = $state<string | null>(null);

  // Actions
  async function fetch() {
    loading = true;
    error = null;
    try {
      data = await querySessions({ time: selectedRange, chain: selectedChain ?? undefined, limit: 50 });
    } catch (e) {
      error = e as CommandError;
      data = null;
    } finally {
      loading = false;
    }
  }

  async function setRange(range: '7d' | '14d' | '30d') {
    selectedRange = range;
    await fetch();
  }

  async function setChainFilter(chainId: string | null) {
    selectedChain = chainId;
    await fetch();
  }

  function toggleSessionExpanded(sessionId: string) {
    const newSet = new Set(expandedSessions);
    if (newSet.has(sessionId)) {
      newSet.delete(sessionId);
    } else {
      newSet.add(sessionId);
    }
    expandedSessions = newSet;
  }

  function isExpanded(sessionId: string): boolean {
    return expandedSessions.has(sessionId);
  }

  function collapseAll() {
    expandedSessions = new Set();
  }

  // Derived
  function getMaxAccessCount(): number {
    if (!data?.sessions?.length) return 0;
    return Math.max(...data.sessions.flatMap(s => s.files.map(f => f.access_count)), 1);
  }

  function getFilteredSessions() {
    if (!data?.sessions) return [];
    if (!selectedChain) return data.sessions;
    return data.sessions.filter(s => s.chain_id === selectedChain);
  }

  return {
    get loading() { return loading; },
    get data() { return data; },
    get error() { return error; },
    get selectedRange() { return selectedRange; },
    get selectedChain() { return selectedChain; },
    get expandedSessions() { return expandedSessions; },
    get maxAccessCount() { return getMaxAccessCount(); },
    get filteredSessions() { return getFilteredSessions(); },

    fetch,
    setRange,
    setChainFilter,
    toggleSessionExpanded,
    isExpanded,
    collapseAll,
  };
}
