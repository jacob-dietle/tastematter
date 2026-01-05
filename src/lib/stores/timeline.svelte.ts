/**
 * TimelineStore - Unified Data Architecture (Spec 08)
 *
 * Timeline view store that can optionally use shared context:
 * - If context provided: uses context.timeRange
 * - If no context: uses internal selectedRange (backwards compatible)
 */
import { queryTimeline } from '$lib/api/tauri';
import type { ContextStore } from './context.svelte';
import type { TimelineData, TimeBucket, FileTimeline, CommandError } from '$lib/types';

export function createTimelineStore(ctx?: ContextStore) {
  // State
  let loading = $state(false);
  let data = $state<TimelineData | null>(null);
  let error = $state<CommandError | null>(null);
  let selectedRange = $state<'7d' | '14d' | '30d'>('7d');
  let hoveredCell = $state<{ file: string; date: string } | null>(null);

  // Get timeRange - from context if available, otherwise internal state
  function getTimeRange(): '7d' | '14d' | '30d' {
    return ctx ? ctx.timeRange : selectedRange;
  }

  // Actions
  async function fetch() {
    loading = true;
    error = null;
    try {
      data = await queryTimeline({ time: getTimeRange(), limit: 30 });
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

  function setHoveredCell(file: string, date: string) {
    hoveredCell = { file, date };
  }

  function clearHover() {
    hoveredCell = null;
  }

  // Derived: max access count from files
  function getMaxAccessCount(): number {
    if (!data?.files?.length) return 0;
    return Math.max(...data.files.map(f => f.total_accesses));
  }

  // Derived: intensity for heat map cell
  function getIntensity(filePath: string, date: string): number {
    if (!data?.files) return 0;
    const file = data.files.find(f => f.file_path === filePath);
    if (!file) return 0;
    const count = file.buckets[date] ?? 0;
    const max = getMaxAccessCount();
    if (max === 0) return 0;
    return count / max;
  }

  // Derived: files array
  function getFiles(): FileTimeline[] {
    return data?.files ?? [];
  }

  // Derived: buckets array
  function getBuckets(): TimeBucket[] {
    return data?.buckets ?? [];
  }

  // Derived: dates array from buckets
  function getDates(): string[] {
    return getBuckets().map(b => b.date);
  }

  return {
    // State getters
    get loading() { return loading; },
    get data() { return data; },
    get error() { return error; },
    get selectedRange() { return selectedRange; },
    get hoveredCell() { return hoveredCell; },
    get maxAccessCount() { return getMaxAccessCount(); },

    // Context-aware getters
    get timeRange() { return getTimeRange(); },
    get files() { return getFiles(); },
    get buckets() { return getBuckets(); },
    get dates() { return getDates(); },

    // Actions
    fetch,
    setRange,
    setHoveredCell,
    clearHover,
    getIntensity,
  };
}

export type TimelineStore = ReturnType<typeof createTimelineStore>;
