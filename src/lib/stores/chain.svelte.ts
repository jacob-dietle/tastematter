import { queryChains } from '$lib/api/tauri';
import type { ChainQueryResult, CommandError } from '$lib/types';

export function createChainStore() {
  // State
  let loading = $state(false);
  let data = $state<ChainQueryResult | null>(null);
  let error = $state<CommandError | null>(null);
  let selectedChain = $state<string | null>(null);

  // Actions
  async function fetch(limit?: number) {
    loading = true;
    error = null;
    try {
      data = await queryChains({ limit: limit ?? 20 });
    } catch (e) {
      error = e as CommandError;
      data = null;
    } finally {
      loading = false;
    }
  }

  function selectChain(chainId: string | null) {
    selectedChain = chainId;
  }

  function toggleChain(chainId: string) {
    selectedChain = selectedChain === chainId ? null : chainId;
  }

  // Derived
  function getChains() {
    return data?.chains ?? [];
  }

  function getSelectedChainData() {
    if (!selectedChain || !data?.chains) return null;
    return data.chains.find(c => c.chain_id === selectedChain) ?? null;
  }

  return {
    get loading() { return loading; },
    get data() { return data; },
    get error() { return error; },
    get selectedChain() { return selectedChain; },
    get chains() { return getChains(); },
    get selectedChainData() { return getSelectedChainData(); },
    get totalChains() { return data?.total_chains ?? 0; },

    fetch,
    selectChain,
    toggleChain,
  };
}
