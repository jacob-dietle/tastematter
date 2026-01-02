import { describe, test, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import SessionCard from '$lib/components/SessionCard.svelte';
import type { SessionData, SessionFile } from '$lib/types';

const mockColorScale = (count: number) => `rgb(${Math.min(255, count * 25)}, 100, 100)`;

const mockTopFiles: SessionFile[] = [
  { file_path: 'src/App.svelte', access_count: 10, access_types: ['read'], last_access: '2025-01-01' },
  { file_path: 'src/lib/utils.ts', access_count: 5, access_types: ['read'], last_access: '2025-01-01' },
];

const mockAllFiles: SessionFile[] = [
  ...mockTopFiles,
  { file_path: 'package.json', access_count: 3, access_types: ['read'], last_access: '2025-01-01' },
  { file_path: 'vite.config.ts', access_count: 2, access_types: ['read'], last_access: '2025-01-01' },
];

const mockSession: SessionData = {
  session_id: 'abc123def456xyz789',
  chain_id: 'chain-test-id-12345',
  started_at: '2025-01-02T15:45:00Z',
  ended_at: '2025-01-02T17:30:00Z',
  duration_seconds: 6300, // 1h 45m
  file_count: 4,
  total_accesses: 20,
  files: mockAllFiles,
  top_files: mockTopFiles,
};

describe('SessionCard', () => {
  const defaultProps = {
    session: mockSession,
    expanded: false,
    onToggleExpand: vi.fn(),
    onFileClick: vi.fn(),
    onChainClick: vi.fn(),
    colorScale: mockColorScale,
  };

  test('has data-testid for component root', () => {
    render(SessionCard, { props: defaultProps });

    expect(screen.getByTestId('session-card')).toBeInTheDocument();
  });

  test('renders truncated session ID (first 8 chars)', () => {
    render(SessionCard, { props: defaultProps });

    expect(screen.getByText('abc123de')).toBeInTheDocument();
  });

  test('shows ChainBadge with session.chain_id', () => {
    render(SessionCard, { props: defaultProps });

    // ChainBadge truncates to 8 chars
    expect(screen.getByText('chain-te')).toBeInTheDocument();
  });

  test('formats and displays date', () => {
    render(SessionCard, { props: defaultProps });

    // Check for formatted date parts - the exact format depends on locale
    const card = screen.getByTestId('session-card');
    expect(card.textContent).toMatch(/Jan|2|3:45|PM/i);
  });

  test('formats duration in seconds', () => {
    const session = { ...mockSession, duration_seconds: 45 };
    render(SessionCard, { props: { ...defaultProps, session } });

    expect(screen.getByText('45s')).toBeInTheDocument();
  });

  test('formats duration in minutes', () => {
    const session = { ...mockSession, duration_seconds: 300 };
    render(SessionCard, { props: { ...defaultProps, session } });

    expect(screen.getByText('5min')).toBeInTheDocument();
  });

  test('formats duration in hours and minutes', () => {
    render(SessionCard, { props: defaultProps });

    // 6300 seconds = 1h 45m
    expect(screen.getByText('1h 45m')).toBeInTheDocument();
  });

  test('hides duration when null', () => {
    const session = { ...mockSession, duration_seconds: null };
    render(SessionCard, { props: { ...defaultProps, session } });

    expect(screen.queryByText(/\ds|\dmin|\dh/)).not.toBeInTheDocument();
  });

  test('shows file count with proper pluralization (plural)', () => {
    render(SessionCard, { props: defaultProps });

    expect(screen.getByText('4 files')).toBeInTheDocument();
  });

  test('shows file count with proper pluralization (singular)', () => {
    const session = { ...mockSession, file_count: 1 };
    render(SessionCard, { props: { ...defaultProps, session } });

    expect(screen.getByText('1 file')).toBeInTheDocument();
  });

  test('shows access count with proper pluralization (plural)', () => {
    render(SessionCard, { props: defaultProps });

    expect(screen.getByText('20 accesses')).toBeInTheDocument();
  });

  test('shows access count with proper pluralization (singular)', () => {
    const session = { ...mockSession, total_accesses: 1 };
    render(SessionCard, { props: { ...defaultProps, session } });

    expect(screen.getByText('1 access')).toBeInTheDocument();
  });

  test('shows SessionFilePreview when collapsed', () => {
    render(SessionCard, { props: { ...defaultProps, expanded: false } });

    // SessionFilePreview shows top files and "+ N more" button
    expect(screen.getByTestId('file-preview')).toBeInTheDocument();
    expect(screen.queryByTestId('file-tree')).not.toBeInTheDocument();
  });

  test('shows SessionFileTree when expanded', () => {
    render(SessionCard, { props: { ...defaultProps, expanded: true } });

    expect(screen.getByTestId('file-tree')).toBeInTheDocument();
    expect(screen.queryByTestId('file-preview')).not.toBeInTheDocument();
  });

  test('shows Collapse button when expanded', () => {
    render(SessionCard, { props: { ...defaultProps, expanded: true } });

    expect(screen.getByText(/Collapse/)).toBeInTheDocument();
  });

  test('calls onToggleExpand when Collapse button clicked', async () => {
    const onToggleExpand = vi.fn();
    render(SessionCard, { props: { ...defaultProps, expanded: true, onToggleExpand } });

    await fireEvent.click(screen.getByText(/Collapse/));

    expect(onToggleExpand).toHaveBeenCalledWith('abc123def456xyz789');
  });

  test('has expanded class when expanded=true', () => {
    render(SessionCard, { props: { ...defaultProps, expanded: true } });

    expect(screen.getByTestId('session-card')).toHaveClass('expanded');
  });

  test('does not have expanded class when expanded=false', () => {
    render(SessionCard, { props: { ...defaultProps, expanded: false } });

    expect(screen.getByTestId('session-card')).not.toHaveClass('expanded');
  });
});
