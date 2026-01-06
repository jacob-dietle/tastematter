import { describe, test, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import ChainCard from '$lib/components/ChainCard.svelte';
import type { ChainData, SessionData, CommandError } from '$lib/types';

// Mock callbacks
const mockOnToggleExpand = vi.fn();
const mockOnToggleSession = vi.fn();
const mockOnFileClick = vi.fn();
const mockOnRetry = vi.fn();

// Mock data
const mockChain: ChainData = {
  chain_id: 'abc12345',
  session_count: 5,
  file_count: 23,
  time_range: {
    start: '2025-12-01T10:00:00Z',
    end: '2025-12-15T16:00:00Z',
  },
};

const mockChainNoTimeRange: ChainData = {
  chain_id: 'def67890',
  session_count: 2,
  file_count: 8,
  time_range: null,
};

const mockSession: SessionData = {
  session_id: 'session-001',
  chain_id: 'abc12345',
  started_at: '2025-12-15T14:00:00Z',
  ended_at: '2025-12-15T16:00:00Z',
  duration_seconds: 7200,
  file_count: 5,
  total_accesses: 20,
  files: [
    { file_path: 'src/App.svelte', access_count: 10, access_types: ['read'], last_access: '2025-12-15T15:00:00Z' },
  ],
  top_files: [
    { file_path: 'src/App.svelte', access_count: 10, access_types: ['read'], last_access: '2025-12-15T15:00:00Z' },
  ],
};

const mockSessions: SessionData[] = [mockSession];

const mockError: CommandError = {
  code: 'LOAD_ERROR',
  message: 'Failed to load sessions',
};

// Default props
const defaultProps = {
  chain: mockChain,
  expanded: false,
  loading: false,
  sessions: [] as SessionData[],
  error: null as CommandError | null,
  expandedSessions: new Set<string>(),
  onToggleExpand: mockOnToggleExpand,
  onToggleSession: mockOnToggleSession,
  onFileClick: mockOnFileClick,
  onRetry: mockOnRetry,
};

describe('ChainCard', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // Rendering tests
  test('has data-testid="chain-card" for component root', () => {
    render(ChainCard, { props: defaultProps });
    expect(screen.getByTestId('chain-card')).toBeInTheDocument();
  });

  test('renders ChainBadge with chain.chain_id', () => {
    render(ChainCard, { props: defaultProps });
    // ChainBadge renders the first 8 chars of chain_id
    expect(screen.getByText('abc12345')).toBeInTheDocument();
  });

  test('displays session count from chain.session_count', () => {
    render(ChainCard, { props: defaultProps });
    expect(screen.getByText(/5 sessions/i)).toBeInTheDocument();
  });

  test('displays file count from chain.file_count', () => {
    render(ChainCard, { props: defaultProps });
    expect(screen.getByText(/23 files/i)).toBeInTheDocument();
  });

  test('formats and displays time range when present', () => {
    render(ChainCard, { props: defaultProps });
    // Should show formatted date range like "12/01 - 12/15"
    expect(screen.getByText(/12\/01.*12\/15/)).toBeInTheDocument();
  });

  test('hides time range when chain.time_range is null', () => {
    render(ChainCard, { props: { ...defaultProps, chain: mockChainNoTimeRange } });
    // Should not show any date range
    expect(screen.queryByText(/\d{2}\/\d{2}.*\d{2}\/\d{2}/)).not.toBeInTheDocument();
  });

  // Expand/collapse state tests
  test('shows expand button when collapsed', () => {
    render(ChainCard, { props: defaultProps });
    const expandBtn = screen.getByTestId('expand-button');
    expect(expandBtn).toBeInTheDocument();
    expect(expandBtn.textContent).toContain('▼');
  });

  test('shows collapse button when expanded', () => {
    render(ChainCard, { props: { ...defaultProps, expanded: true, sessions: mockSessions } });
    const expandBtn = screen.getByTestId('expand-button');
    expect(expandBtn.textContent).toContain('▲');
  });

  test('calls onToggleExpand with chainId when expand button clicked', async () => {
    render(ChainCard, { props: defaultProps });
    const expandBtn = screen.getByTestId('expand-button');
    await fireEvent.click(expandBtn);
    expect(mockOnToggleExpand).toHaveBeenCalledWith('abc12345');
  });

  test('has expanded class when expanded=true', () => {
    render(ChainCard, { props: { ...defaultProps, expanded: true, sessions: mockSessions } });
    const card = screen.getByTestId('chain-card');
    expect(card.classList.contains('expanded')).toBe(true);
  });

  test('does not have expanded class when expanded=false', () => {
    render(ChainCard, { props: defaultProps });
    const card = screen.getByTestId('chain-card');
    expect(card.classList.contains('expanded')).toBe(false);
  });

  // Loading state tests
  test('shows LoadingSpinner when loading=true and expanded', () => {
    render(ChainCard, { props: { ...defaultProps, expanded: true, loading: true } });
    expect(screen.getByTestId('loading-spinner')).toBeInTheDocument();
  });

  test('hides sessions list when loading=true', () => {
    render(ChainCard, { props: { ...defaultProps, expanded: true, loading: true, sessions: mockSessions } });
    // Sessions should not be rendered while loading
    expect(screen.queryByTestId('session-card')).not.toBeInTheDocument();
  });

  // Sessions rendering tests
  test('renders SessionCard for each session when expanded', () => {
    render(ChainCard, { props: { ...defaultProps, expanded: true, sessions: mockSessions } });
    expect(screen.getByTestId('session-card')).toBeInTheDocument();
  });

  test('shows empty state message when expanded with no sessions', () => {
    render(ChainCard, { props: { ...defaultProps, expanded: true, sessions: [] } });
    expect(screen.getByText(/No sessions/i)).toBeInTheDocument();
  });

  // Error handling tests
  test('displays ErrorDisplay when error is present', () => {
    render(ChainCard, { props: { ...defaultProps, expanded: true, error: mockError } });
    expect(screen.getByText(/Failed to load sessions/i)).toBeInTheDocument();
  });

  test('calls onRetry with chainId when retry button clicked', async () => {
    render(ChainCard, { props: { ...defaultProps, expanded: true, error: mockError } });
    const retryBtn = screen.getByText(/Retry/i);
    await fireEvent.click(retryBtn);
    expect(mockOnRetry).toHaveBeenCalledWith('abc12345');
  });
});
