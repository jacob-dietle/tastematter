/**
 * FilesStore - Unified Data Architecture (Spec 08)
 *
 * Files view store that uses shared context for filtering:
 * - Reads timeRange from context
 * - Reads selectedChain from context
 * - Refetches when context changes
 */
import { queryFlex } from '$lib/api/tauri';
import type { ContextStore } from './context.svelte';
import type { QueryResult, FileResult, CommandError } from '$lib/types';

export function createFilesStore(ctx: ContextStore) {
  // State
  let data = $state<QueryResult | null>(null);
  let loading = $state(false);
  let error = $state<CommandError | null>(null);
  let sort = $state<'count' | 'recency' | 'alpha'>('count');
  let granularity = $state<'file' | 'directory'>('file');

  // Actions
  async function fetch() {
    loading = true;
    error = null;
    try {
      data = await queryFlex({
        time: ctx.timeRange,
        chain: ctx.selectedChain ?? undefined,
        agg: ['count', 'recency', 'sessions'],
        limit: 50,
        sort: sort,
      });
    } catch (e) {
      error = e as CommandError;
      data = null;
    } finally {
      loading = false;
    }
  }

  function setSort(newSort: 'count' | 'recency' | 'alpha') {
    sort = newSort;
  }

  function setGranularity(newGranularity: 'file' | 'directory') {
    granularity = newGranularity;
  }

  // Derived
  function getFiles(): FileResult[] {
    return data?.results ?? [];
  }

  function getTotalFiles(): number {
    return data?.aggregations?.count?.total_files ?? 0;
  }

  function getTotalAccesses(): number {
    return data?.aggregations?.count?.total_accesses ?? 0;
  }

  function getMaxAccessCount(): number {
    const files = getFiles();
    if (files.length === 0) return 0;
    return Math.max(...files.map(f => f.access_count));
  }

  return {
    // State getters
    get loading() { return loading; },
    get data() { return data; },
    get error() { return error; },
    get sort() { return sort; },
    get granularity() { return granularity; },

    // Derived getters
    get files() { return getFiles(); },
    get totalFiles() { return getTotalFiles(); },
    get totalAccesses() { return getTotalAccesses(); },
    get maxAccessCount() { return getMaxAccessCount(); },

    // Actions
    fetch,
    setSort,
    setGranularity,
  };
}

export type FilesStore = ReturnType<typeof createFilesStore>;
