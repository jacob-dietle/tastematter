import { describe, test, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import SessionView from '$lib/components/SessionView.svelte';
import type { SessionQueryResult, SessionData } from '$lib/types';

const mockFetch = vi.fn();
const mockSetRange = vi.fn();
const mockSetChainFilter = vi.fn();
const mockToggleSessionExpanded = vi.fn();
const mockIsExpanded = vi.fn(() => false);
const mockColorScale = vi.fn((count: number) => `rgb(${count * 25}, 100, 100)`);

const mockSession: SessionData = {
  session_id: 'abc123def456',
  chain_id: 'chain-test-id',
  started_at: '2025-01-02T14:00:00Z',
  ended_at: '2025-01-02T16:00:00Z',
  duration_seconds: 7200,
  file_count: 5,
  total_accesses: 20,
  files: [
    { file_path: 'src/App.svelte', access_count: 10, access_types: ['read'], last_access: '2025-01-02T15:00:00Z' },
    { file_path: 'src/lib/store.ts', access_count: 5, access_types: ['read'], last_access: '2025-01-02T15:30:00Z' },
  ],
  top_files: [
    { file_path: 'src/App.svelte', access_count: 10, access_types: ['read'], last_access: '2025-01-02T15:00:00Z' },
    { file_path: 'src/lib/store.ts', access_count: 5, access_types: ['read'], last_access: '2025-01-02T15:30:00Z' },
  ],
};

const mockSessionData: SessionQueryResult = {
  time_range: '7d',
  sessions: [mockSession],
  chains: [{ chain_id: 'chain-test-id', session_count: 1, file_count: 5, last_active: '2025-01-02T16:00:00Z' }],
  summary: { total_sessions: 1, total_files: 5, total_accesses: 20, active_chains: 1 },
};

// Default mock store state
let mockStoreState = {
  loading: false,
  data: mockSessionData,
  error: null as { code: string; message: string } | null,
  selectedRange: '7d' as '7d' | '14d' | '30d',
  selectedChain: null as string | null,
  filteredSessions: [mockSession],
};

// Mock the session store
vi.mock('$lib/stores/session.svelte', () => ({
  createSessionStore: vi.fn(() => ({
    get loading() { return mockStoreState.loading; },
    get data() { return mockStoreState.data; },
    get error() { return mockStoreState.error; },
    get selectedRange() { return mockStoreState.selectedRange; },
    get selectedChain() { return mockStoreState.selectedChain; },
    get filteredSessions() { return mockStoreState.filteredSessions; },
    fetch: mockFetch,
    setRange: mockSetRange,
    setChainFilter: mockSetChainFilter,
    toggleSessionExpanded: mockToggleSessionExpanded,
    isExpanded: mockIsExpanded,
    colorScale: mockColorScale,
  })),
}));

describe('SessionView', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Reset to default state
    mockStoreState = {
      loading: false,
      data: mockSessionData,
      error: null,
      selectedRange: '7d',
      selectedChain: null,
      filteredSessions: [mockSession],
    };
  });

  test('has data-testid="session-view"', () => {
    render(SessionView);

    expect(screen.getByTestId('session-view')).toBeInTheDocument();
  });

  test('renders title "Sessions"', () => {
    render(SessionView);

    expect(screen.getByText('Sessions')).toBeInTheDocument();
  });

  test('renders TimeRangeToggle', () => {
    render(SessionView);

    expect(screen.getByText('7d')).toBeInTheDocument();
  });

  test('renders refresh button', () => {
    render(SessionView);

    const refreshBtn = screen.getByTitle('Refresh data');
    expect(refreshBtn).toBeInTheDocument();
  });

  test('shows LoadingSpinner when loading and no data', () => {
    mockStoreState.loading = true;
    mockStoreState.data = null;

    render(SessionView);

    expect(screen.getByTestId('loading-spinner')).toBeInTheDocument();
  });

  test('shows ErrorDisplay when error', () => {
    mockStoreState.error = { code: 'TEST_ERROR', message: 'Test error message' };
    mockStoreState.data = null;

    render(SessionView);

    expect(screen.getByText(/Test error message/i)).toBeInTheDocument();
  });

  test('shows summary stats when data exists', () => {
    render(SessionView);

    const summary = screen.getByTestId('session-summary');
    expect(summary).toBeInTheDocument();
    expect(summary.textContent).toContain('1 sessions');
    expect(summary.textContent).toContain('5 files');
    expect(summary.textContent).toContain('20 accesses');
    expect(summary.textContent).toContain('1 chains');
  });

  test('renders SessionCard for each session', () => {
    render(SessionView);

    expect(screen.getByTestId('session-card')).toBeInTheDocument();
  });

  test('shows empty state when no sessions', () => {
    mockStoreState.filteredSessions = [];

    render(SessionView);

    expect(screen.getByText(/No sessions found/i)).toBeInTheDocument();
  });

  test('shows filter bar when chain selected', () => {
    mockStoreState.selectedChain = 'chain-test-id';

    render(SessionView);

    expect(screen.getByText(/Filtered by chain/i)).toBeInTheDocument();
    expect(screen.getByText('Clear filter')).toBeInTheDocument();
  });

  test('calls sessionStore.fetch on refresh click', async () => {
    render(SessionView);

    const refreshBtn = screen.getByTitle('Refresh data');
    await fireEvent.click(refreshBtn);

    expect(mockFetch).toHaveBeenCalled();
  });

  test('disables refresh button when loading', () => {
    mockStoreState.loading = true;

    render(SessionView);

    const refreshBtn = screen.getByTitle('Refresh data');
    expect(refreshBtn).toBeDisabled();
  });
});
