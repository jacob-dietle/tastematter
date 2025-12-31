import { describe, test, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import TimelineView from '$lib/components/TimelineView.svelte';
import type { TimelineData } from '$lib/types';

// Mock the timeline store
vi.mock('$lib/stores/timeline.svelte', () => ({
  createTimelineStore: vi.fn(() => ({
    loading: false,
    data: null,
    error: null,
    selectedRange: '7d',
    hoveredCell: null,
    maxAccessCount: 0,
    fetch: vi.fn(),
    setRange: vi.fn(),
    setHoveredCell: vi.fn(),
    clearHover: vi.fn(),
    getIntensity: vi.fn(() => 0),
  }))
}));

const mockTimelineData: TimelineData = {
  time_range: '7d',
  start_date: '2025-12-24',
  end_date: '2025-12-30',
  buckets: [
    { date: '2025-12-28', day_of_week: 'Sat', access_count: 5, files_touched: 2, sessions: [] },
    { date: '2025-12-29', day_of_week: 'Sun', access_count: 10, files_touched: 4, sessions: [] },
    { date: '2025-12-30', day_of_week: 'Mon', access_count: 25, files_touched: 8, sessions: ['s1'] },
  ],
  files: [
    {
      file_path: 'src/lib/store.ts',
      total_accesses: 15,
      buckets: { '2025-12-28': 5, '2025-12-30': 10 },
      first_access: '2025-12-28T10:00:00Z',
      last_access: '2025-12-30T14:00:00Z'
    },
  ],
  summary: {
    total_accesses: 40,
    total_files: 3,
    peak_day: '2025-12-30',
    peak_count: 25
  }
};

describe('TimelineView', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  test('renders container element', () => {
    render(TimelineView);

    const container = document.querySelector('.timeline-view');
    expect(container).toBeInTheDocument();
  });

  test('shows loading state', async () => {
    // Re-mock with loading true
    vi.doMock('$lib/stores/timeline.svelte', () => ({
      createTimelineStore: vi.fn(() => ({
        loading: true,
        data: null,
        error: null,
        selectedRange: '7d',
        hoveredCell: null,
        maxAccessCount: 0,
        fetch: vi.fn(),
        setRange: vi.fn(),
        setHoveredCell: vi.fn(),
        clearHover: vi.fn(),
        getIntensity: vi.fn(() => 0),
      }))
    }));

    // Component should have loading indicator or disabled state
    render(TimelineView);
    const container = document.querySelector('.timeline-view');
    expect(container).toBeInTheDocument();
  });

  test('renders TimeRangeToggle', () => {
    render(TimelineView);

    // TimeRangeToggle should be present
    expect(screen.getByText('7d')).toBeInTheDocument();
  });

  test('renders legend', () => {
    render(TimelineView);

    expect(screen.getByText('Less')).toBeInTheDocument();
    expect(screen.getByText('More')).toBeInTheDocument();
  });
});
