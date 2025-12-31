import { queryTimeline } from '$lib/api/tauri';
import type { TimelineData, CommandError } from '$lib/types';

export function createTimelineStore() {
  // State
  let loading = $state(false);
  let data = $state<TimelineData | null>(null);
  let error = $state<CommandError | null>(null);
  let selectedRange = $state<'7d' | '14d' | '30d'>('7d');
  let hoveredCell = $state<{ file: string; date: string } | null>(null);

  // Actions
  async function fetch() {
    loading = true;
    error = null;
    try {
      data = await queryTimeline({ time: selectedRange, limit: 30 });
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

  return {
    // State getters
    get loading() { return loading; },
    get data() { return data; },
    get error() { return error; },
    get selectedRange() { return selectedRange; },
    get hoveredCell() { return hoveredCell; },
    get maxAccessCount() { return getMaxAccessCount(); },

    // Actions
    fetch,
    setRange,
    setHoveredCell,
    clearHover,
    getIntensity,
  };
}
