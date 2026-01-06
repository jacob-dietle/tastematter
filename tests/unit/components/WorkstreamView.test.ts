import { describe, test, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import WorkstreamView from '$lib/components/WorkstreamView.svelte';
import type { ChainData, CommandError } from '$lib/types';

// Mock chain data
const mockChains: ChainData[] = [
  {
    chain_id: 'chain-001',
    session_count: 10,
    file_count: 50,
    time_range: { start: '2025-12-01T00:00:00Z', end: '2025-12-15T00:00:00Z' },
  },
  {
    chain_id: 'chain-002',
    session_count: 5,
    file_count: 20,
    time_range: { start: '2025-12-10T00:00:00Z', end: '2025-12-14T00:00:00Z' },
  },
];

// Mock context state
let mockContextState = {
  timeRange: '7d' as '7d' | '14d' | '30d',
  selectedChain: null as string | null,
  chains: mockChains,
  chainsLoading: false,
  chainsError: null as CommandError | null,
  totalChains: 2,
};

// Mock workstream store state
let mockWorkstreamState = {
  expandedChains: new Set<string>(),
  expandedSessions: new Set<string>(),
  totalLoadedSessions: 0,
};

// Mock functions
const mockExpandAllChains = vi.fn();
const mockCollapseAllChains = vi.fn();
const mockToggleChainExpanded = vi.fn();
const mockIsChainExpanded = vi.fn((chainId: string) => mockWorkstreamState.expandedChains.has(chainId));
const mockIsChainLoading = vi.fn(() => false);
const mockGetSessionsForChain = vi.fn(() => []);
const mockGetChainError = vi.fn(() => null);
const mockToggleSessionExpanded = vi.fn();
const mockRetryLoadSessions = vi.fn();

// Mock the context store
vi.mock('$lib/stores/context.svelte', () => ({
  getAppContext: vi.fn(() => ({
    get timeRange() { return mockContextState.timeRange; },
    get selectedChain() { return mockContextState.selectedChain; },
    get chains() { return mockContextState.chains; },
    get chainsLoading() { return mockContextState.chainsLoading; },
    get chainsError() { return mockContextState.chainsError; },
    get totalChains() { return mockContextState.totalChains; },
  })),
}));

// Mock the workstream store
vi.mock('$lib/stores/workstream.svelte', () => ({
  createWorkstreamStore: vi.fn(() => ({
    get chains() { return mockContextState.chains; },
    get timeRange() { return mockContextState.timeRange; },
    get selectedChain() { return mockContextState.selectedChain; },
    get expandedChains() { return mockWorkstreamState.expandedChains; },
    get expandedSessions() { return mockWorkstreamState.expandedSessions; },
    get totalLoadedSessions() { return mockWorkstreamState.totalLoadedSessions; },
    expandAllChains: mockExpandAllChains,
    collapseAllChains: mockCollapseAllChains,
    toggleChainExpanded: mockToggleChainExpanded,
    isChainExpanded: mockIsChainExpanded,
    isChainLoading: mockIsChainLoading,
    getSessionsForChain: mockGetSessionsForChain,
    getChainError: mockGetChainError,
    toggleSessionExpanded: mockToggleSessionExpanded,
    retryLoadSessions: mockRetryLoadSessions,
  })),
}));

describe('WorkstreamView', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Reset to default state
    mockContextState = {
      timeRange: '7d',
      selectedChain: null,
      chains: mockChains,
      chainsLoading: false,
      chainsError: null,
      totalChains: 2,
    };
    mockWorkstreamState = {
      expandedChains: new Set<string>(),
      expandedSessions: new Set<string>(),
      totalLoadedSessions: 0,
    };
  });

  // Rendering tests
  test('has data-testid="workstream-view" for component root', () => {
    render(WorkstreamView);
    expect(screen.getByTestId('workstream-view')).toBeInTheDocument();
  });

  test('renders header with "Workstreams" title', () => {
    render(WorkstreamView);
    expect(screen.getByText('Workstreams')).toBeInTheDocument();
  });

  test('displays total chains count from context', () => {
    render(WorkstreamView);
    expect(screen.getByText(/2 chains/i)).toBeInTheDocument();
  });

  test('displays total loaded sessions count', () => {
    mockWorkstreamState.totalLoadedSessions = 15;
    render(WorkstreamView);
    expect(screen.getByText(/15 sessions/i)).toBeInTheDocument();
  });

  // Chain list rendering tests
  test('renders ChainCard for each chain from context.chains', () => {
    render(WorkstreamView);
    const chainCards = screen.getAllByTestId('chain-card');
    expect(chainCards).toHaveLength(2);
  });

  test('shows LoadingSpinner when chainsLoading is true', () => {
    mockContextState.chainsLoading = true;
    mockContextState.chains = [];
    render(WorkstreamView);
    expect(screen.getByTestId('loading-spinner')).toBeInTheDocument();
  });

  test('shows ErrorDisplay when chainsError is present', () => {
    mockContextState.chainsError = { code: 'CHAIN_ERROR', message: 'Failed to load chains' };
    mockContextState.chains = [];
    render(WorkstreamView);
    expect(screen.getByText(/Failed to load chains/i)).toBeInTheDocument();
  });

  test('shows empty state when chains is empty', () => {
    mockContextState.chains = [];
    mockContextState.totalChains = 0;
    render(WorkstreamView);
    expect(screen.getByText(/No chains/i)).toBeInTheDocument();
  });

  // Expand/collapse all tests
  test('has "Expand All" button', () => {
    render(WorkstreamView);
    expect(screen.getByText('Expand All')).toBeInTheDocument();
  });

  test('has "Collapse All" button', () => {
    render(WorkstreamView);
    expect(screen.getByText('Collapse All')).toBeInTheDocument();
  });

  test('calls workstreamStore.expandAllChains when Expand All clicked', async () => {
    render(WorkstreamView);
    const expandAllBtn = screen.getByText('Expand All');
    await fireEvent.click(expandAllBtn);
    expect(mockExpandAllChains).toHaveBeenCalled();
  });

  test('calls workstreamStore.collapseAllChains when Collapse All clicked', async () => {
    render(WorkstreamView);
    const collapseAllBtn = screen.getByText('Collapse All');
    await fireEvent.click(collapseAllBtn);
    expect(mockCollapseAllChains).toHaveBeenCalled();
  });
});
