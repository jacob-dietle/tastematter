import { describe, test, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import SessionFilePreview from '$lib/components/SessionFilePreview.svelte';
import type { SessionFile } from '$lib/types';

const mockColorScale = (count: number) => `rgb(${Math.min(255, count * 25)}, 100, 100)`;

const mockFiles: SessionFile[] = [
  { file_path: 'src/components/App.svelte', access_count: 10, access_types: ['read'], last_access: '2025-01-01' },
  { file_path: 'src/lib/utils.ts', access_count: 5, access_types: ['read'], last_access: '2025-01-01' },
  { file_path: 'package.json', access_count: 3, access_types: ['read'], last_access: '2025-01-01' },
];

describe('SessionFilePreview', () => {
  test('renders file names extracted from paths', () => {
    render(SessionFilePreview, {
      props: { files: mockFiles, totalCount: 3, onShowMore: vi.fn(), colorScale: mockColorScale }
    });

    expect(screen.getByText('App.svelte')).toBeInTheDocument();
    expect(screen.getByText('utils.ts')).toBeInTheDocument();
    expect(screen.getByText('package.json')).toBeInTheDocument();
  });

  test('shows access count for each file', () => {
    render(SessionFilePreview, {
      props: { files: mockFiles, totalCount: 3, onShowMore: vi.fn(), colorScale: mockColorScale }
    });

    expect(screen.getByText('(10)')).toBeInTheDocument();
    expect(screen.getByText('(5)')).toBeInTheDocument();
    expect(screen.getByText('(3)')).toBeInTheDocument();
  });

  test('renders colored dots using colorScale function', () => {
    render(SessionFilePreview, {
      props: { files: mockFiles, totalCount: 3, onShowMore: vi.fn(), colorScale: mockColorScale }
    });

    const dots = document.querySelectorAll('.count-dot');
    expect(dots.length).toBe(3);
    // First file has access_count=10, colorScale returns rgb(250, 100, 100)
    expect(dots[0]).toHaveStyle({ background: 'rgb(250, 100, 100)' });
  });

  test('shows "+ N more" button when totalCount > files.length', () => {
    render(SessionFilePreview, {
      props: { files: mockFiles, totalCount: 10, onShowMore: vi.fn(), colorScale: mockColorScale }
    });

    expect(screen.getByText('+ 7 more files')).toBeInTheDocument();
  });

  test('hides "+ N more" when all files shown', () => {
    render(SessionFilePreview, {
      props: { files: mockFiles, totalCount: 3, onShowMore: vi.fn(), colorScale: mockColorScale }
    });

    expect(screen.queryByText(/more file/)).not.toBeInTheDocument();
  });

  test('calls onShowMore when button clicked', async () => {
    const onShowMore = vi.fn();
    render(SessionFilePreview, {
      props: { files: mockFiles, totalCount: 10, onShowMore, colorScale: mockColorScale }
    });

    await fireEvent.click(screen.getByText('+ 7 more files'));

    expect(onShowMore).toHaveBeenCalled();
  });

  test('pluralizes "file" correctly for singular', () => {
    render(SessionFilePreview, {
      props: { files: mockFiles, totalCount: 4, onShowMore: vi.fn(), colorScale: mockColorScale }
    });

    expect(screen.getByText('+ 1 more file')).toBeInTheDocument();
  });

  test('has data-testid for component root', () => {
    render(SessionFilePreview, {
      props: { files: mockFiles, totalCount: 3, onShowMore: vi.fn(), colorScale: mockColorScale }
    });

    expect(screen.getByTestId('file-preview')).toBeInTheDocument();
  });

  test('shows file title attribute with full path', () => {
    render(SessionFilePreview, {
      props: { files: mockFiles, totalCount: 3, onShowMore: vi.fn(), colorScale: mockColorScale }
    });

    const fileItems = document.querySelectorAll('.file-item');
    expect(fileItems[0]).toHaveAttribute('title', 'src/components/App.svelte');
  });
});
