import { describe, test, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import WorkstreamView from '$lib/components/WorkstreamView.svelte';
import type { SessionData, CommandError } from '$lib/types';

// Mock session data
const mockSessions: SessionData[] = [
  {
    session_id: 'session-001',
    chain_id: 'chain-001',
    started_at: '2025-12-15T14:00:00Z',
    ended_at: '2025-12-15T16:00:00Z',
    duration_seconds: 7200,
    file_count: 5,
    total_accesses: 20,
    files: [],
    top_files: [],
  },
  {
    session_id: 'session-002',
    chain_id: 'chain-001',
    started_at: '2025-12-14T10:00:00Z',
    ended_at: '2025-12-14T12:00:00Z',
    duration_seconds: 7200,
    file_count: 3,
    total_accesses: 15,
    files: [],
    top_files: [],
  },
  {
    session_id: 'session-003',
    chain_id: 'chain-002',
    started_at: '2025-12-13T08:00:00Z',
    ended_at: '2025-12-13T10:00:00Z',
    duration_seconds: 7200,
    file_count: 2,
    total_accesses: 10,
    files: [],
    top_files: [],
  },
];

// Mock context state
let mockContextState = {
  timeRange: '7d' as '7d' | '14d' | '30d',
  selectedChain: null as string | null,
};

const mockSetSelectedChain = vi.fn();

// Mock querySessions API
const mockQuerySessions = vi.fn().mockResolvedValue({ sessions: mockSessions });

// Mock the context store
vi.mock('$lib/stores/context.svelte', () => ({
  getAppContext: vi.fn(() => ({
    get timeRange() { return mockContextState.timeRange; },
    get selectedChain() { return mockContextState.selectedChain; },
    setSelectedChain: mockSetSelectedChain,
  })),
}));

// Mock the Tauri API
vi.mock('$lib/api/tauri', () => ({
  querySessions: (...args: unknown[]) => mockQuerySessions(...args),
}));

describe('WorkstreamView', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockContextState = {
      timeRange: '7d',
      selectedChain: null,
    };
    mockQuerySessions.mockResolvedValue({ sessions: mockSessions });
  });

  // Rendering tests
  test('has data-testid="workstream-view" for component root', () => {
    render(WorkstreamView);
    expect(screen.getByTestId('workstream-view')).toBeInTheDocument();
  });

  test('renders header with "Sessions" title', () => {
    render(WorkstreamView);
    expect(screen.getByText('Sessions')).toBeInTheDocument();
  });

  test('renders refresh button', () => {
    render(WorkstreamView);
    expect(screen.getByTitle('Refresh data')).toBeInTheDocument();
  });

  // API call tests
  test('calls querySessions on mount', async () => {
    render(WorkstreamView);
    // Wait for effect to run
    await vi.waitFor(() => {
      expect(mockQuerySessions).toHaveBeenCalledWith({ time: '7d', limit: 100 });
    });
  });

  test('calls querySessions with current timeRange', async () => {
    mockContextState.timeRange = '30d';
    render(WorkstreamView);
    await vi.waitFor(() => {
      expect(mockQuerySessions).toHaveBeenCalledWith({ time: '30d', limit: 100 });
    });
  });

  // Summary stats tests
  test('displays summary stats after loading', async () => {
    render(WorkstreamView);
    await vi.waitFor(() => {
      expect(screen.getByTestId('session-summary')).toBeInTheDocument();
    });
  });

  test('shows session count in summary', async () => {
    render(WorkstreamView);
    await vi.waitFor(() => {
      expect(screen.getByText(/3 sessions/i)).toBeInTheDocument();
    });
  });

  test('shows chains count in summary', async () => {
    render(WorkstreamView);
    await vi.waitFor(() => {
      expect(screen.getByText(/2 chains/i)).toBeInTheDocument();
    });
  });

  // Session list rendering tests
  test('renders SessionCard for each session', async () => {
    render(WorkstreamView);
    await vi.waitFor(() => {
      const sessionCards = screen.getAllByTestId('session-card');
      expect(sessionCards).toHaveLength(3);
    });
  });

  // Loading state tests
  test('shows LoadingSpinner initially', () => {
    mockQuerySessions.mockImplementation(() => new Promise(() => {})); // Never resolves
    render(WorkstreamView);
    expect(screen.getByTestId('loading-spinner')).toBeInTheDocument();
  });

  // Error state tests
  test('shows ErrorDisplay when API fails', async () => {
    mockQuerySessions.mockRejectedValue({ code: 'API_ERROR', message: 'Failed to load' });
    render(WorkstreamView);
    await vi.waitFor(() => {
      expect(screen.getByText(/Failed to load/i)).toBeInTheDocument();
    });
  });

  // Empty state tests
  test('shows empty state when no sessions', async () => {
    mockQuerySessions.mockResolvedValue({ sessions: [] });
    render(WorkstreamView);
    await vi.waitFor(() => {
      expect(screen.getByText(/No sessions found/i)).toBeInTheDocument();
    });
  });

  // Filter bar tests
  test('shows filter bar when selectedChain is set', async () => {
    mockContextState.selectedChain = 'chain-001';
    render(WorkstreamView);
    await vi.waitFor(() => {
      expect(screen.getByText(/Filtered by chain/i)).toBeInTheDocument();
    });
  });

  test('hides filter bar when selectedChain is null', async () => {
    mockContextState.selectedChain = null;
    render(WorkstreamView);
    await vi.waitFor(() => {
      expect(screen.queryByText(/Filtered by chain/i)).not.toBeInTheDocument();
    });
  });

  test('calls setSelectedChain(null) when Clear filter clicked', async () => {
    mockContextState.selectedChain = 'chain-001';
    render(WorkstreamView);
    await vi.waitFor(() => {
      expect(screen.getByText('Clear filter')).toBeInTheDocument();
    });
    const clearBtn = screen.getByText('Clear filter');
    await fireEvent.click(clearBtn);
    expect(mockSetSelectedChain).toHaveBeenCalledWith(null);
  });

  // Refresh button tests
  test('refresh button is clickable and not disabled when not loading', async () => {
    render(WorkstreamView);
    await vi.waitFor(() => {
      expect(screen.getByTestId('session-summary')).toBeInTheDocument();
    });
    const refreshBtn = screen.getByTitle('Refresh data');
    expect(refreshBtn).not.toBeDisabled();
  });

  // Client-side filtering tests
  test('filters sessions by selectedChain', async () => {
    mockContextState.selectedChain = 'chain-002';
    render(WorkstreamView);
    await vi.waitFor(() => {
      const sessionCards = screen.getAllByTestId('session-card');
      // Should only show 1 session (chain-002 has 1 session)
      expect(sessionCards).toHaveLength(1);
    });
  });

  test('updates summary when filtered', async () => {
    mockContextState.selectedChain = 'chain-002';
    render(WorkstreamView);
    await vi.waitFor(() => {
      expect(screen.getByText(/1 sessions/i)).toBeInTheDocument();
      expect(screen.getByText(/1 chains/i)).toBeInTheDocument();
    });
  });
});
