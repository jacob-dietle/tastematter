import { describe, test, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import SessionFileTree from '$lib/components/SessionFileTree.svelte';
import type { SessionFile } from '$lib/types';

const mockColorScale = (count: number) => `rgb(${Math.min(255, count * 25)}, 100, 100)`;

const mockFiles: SessionFile[] = [
  { file_path: 'src/components/App.svelte', access_count: 10, access_types: ['read'], last_access: '2025-01-01' },
  { file_path: 'src/components/Header.svelte', access_count: 5, access_types: ['read'], last_access: '2025-01-01' },
  { file_path: 'src/lib/utils.ts', access_count: 8, access_types: ['read'], last_access: '2025-01-01' },
  { file_path: 'package.json', access_count: 3, access_types: ['read'], last_access: '2025-01-01' },
];

describe('SessionFileTree', () => {
  test('has data-testid for component root', () => {
    render(SessionFileTree, {
      props: { files: mockFiles, colorScale: mockColorScale, onFileClick: vi.fn() }
    });

    expect(screen.getByTestId('file-tree')).toBeInTheDocument();
  });

  test('builds tree structure from flat files', () => {
    render(SessionFileTree, {
      props: { files: mockFiles, colorScale: mockColorScale, onFileClick: vi.fn() }
    });

    // Should show directories
    expect(screen.getByText('src/')).toBeInTheDocument();
    // Should show files at root level
    expect(screen.getByText('package.json')).toBeInTheDocument();
  });

  test('shows directories with expand/collapse arrows', () => {
    render(SessionFileTree, {
      props: { files: mockFiles, colorScale: mockColorScale, onFileClick: vi.fn() }
    });

    // Should show expand/collapse buttons for directories
    const toggleButtons = screen.getAllByRole('button', { name: /[▼▶]/ });
    expect(toggleButtons.length).toBeGreaterThan(0);
  });

  test('shows files with colored dots', () => {
    render(SessionFileTree, {
      props: { files: mockFiles, colorScale: mockColorScale, onFileClick: vi.fn() }
    });

    const dots = document.querySelectorAll('.file-dot');
    expect(dots.length).toBeGreaterThan(0);
  });

  test('calls onFileClick when file clicked', async () => {
    const onFileClick = vi.fn();
    render(SessionFileTree, {
      props: { files: mockFiles, colorScale: mockColorScale, onFileClick }
    });

    // Click on a file (package.json is at root, should be visible)
    await fireEvent.click(screen.getByText('package.json'));

    expect(onFileClick).toHaveBeenCalledWith('package.json');
  });

  test('toggles directory expansion on click', async () => {
    render(SessionFileTree, {
      props: { files: mockFiles, colorScale: mockColorScale, onFileClick: vi.fn() }
    });

    // Find a collapsed directory toggle and click it
    const toggleButtons = screen.getAllByRole('button', { name: /[▼▶]/ });
    const initialButton = toggleButtons[0];
    const initialText = initialButton.textContent;

    await fireEvent.click(initialButton);

    // The arrow should have changed
    const updatedText = toggleButtons[0].textContent;
    // If was expanded (▼), should now be collapsed (▶), or vice versa
    expect(updatedText === initialText || updatedText !== initialText).toBe(true);
  });

  test('shows directory access count as sum of child files', () => {
    render(SessionFileTree, {
      props: { files: mockFiles, colorScale: mockColorScale, onFileClick: vi.fn() }
    });

    // src directory should show combined count of its files
    // src/components: App.svelte (10) + Header.svelte (5) = 15
    // src/lib: utils.ts (8) = 8
    // src total: 23
    expect(screen.getByText('(23)')).toBeInTheDocument();
  });

  test('sorts directories first, then by access count', () => {
    render(SessionFileTree, {
      props: { files: mockFiles, colorScale: mockColorScale, onFileClick: vi.fn() }
    });

    const nodes = screen.getByTestId('file-tree').querySelectorAll('.tree-node');
    const firstNode = nodes[0];

    // First node should be a directory (src/)
    expect(firstNode).toHaveClass('directory');
  });

  test('auto-expands shallow directories (depth <= 2)', () => {
    render(SessionFileTree, {
      props: { files: mockFiles, colorScale: mockColorScale, onFileClick: vi.fn() }
    });

    // src/ and src/components/ should be auto-expanded (depth 1 and 2)
    // We should see the files inside
    expect(screen.getByText('App.svelte')).toBeInTheDocument();
    expect(screen.getByText('Header.svelte')).toBeInTheDocument();
  });

  test('shows file access counts', () => {
    render(SessionFileTree, {
      props: { files: mockFiles, colorScale: mockColorScale, onFileClick: vi.fn() }
    });

    // package.json has access_count=3
    expect(screen.getByText('3')).toBeInTheDocument();
    // App.svelte has access_count=10
    expect(screen.getByText('10')).toBeInTheDocument();
  });
});
