import { describe, test, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import TimelineRow from '$lib/components/TimelineRow.svelte';

const mockDates = ['2025-12-28', '2025-12-29', '2025-12-30'];
const mockBuckets: Record<string, number> = {
  '2025-12-28': 5,
  '2025-12-29': 0,
  '2025-12-30': 15,
};

describe('TimelineRow', () => {
  test('renders file path label', () => {
    render(TimelineRow, {
      props: {
        filePath: 'src/lib/store.ts',
        dates: mockDates,
        buckets: mockBuckets,
        maxCount: 15
      }
    });

    expect(screen.getByText('src/lib/store.ts')).toBeInTheDocument();
  });

  test('renders cells for each date', () => {
    render(TimelineRow, {
      props: {
        filePath: 'src/lib/store.ts',
        dates: mockDates,
        buckets: mockBuckets,
        maxCount: 15
      }
    });

    const cells = document.querySelectorAll('.heat-cell');
    expect(cells.length).toBe(3);
  });

  test('applies correct intensity colors', () => {
    render(TimelineRow, {
      props: {
        filePath: 'src/lib/store.ts',
        dates: mockDates,
        buckets: mockBuckets,
        maxCount: 15
      }
    });

    const cells = document.querySelectorAll('.heat-cell');
    // First cell (5/15 = 0.33) should be low
    // Second cell (0) should be empty
    // Third cell (15/15 = 1.0) should be high
    expect(cells[1]).toHaveStyle({ backgroundColor: '#f6f8fa' }); // empty
  });

  test('calls onHover with file and date', async () => {
    const onHover = vi.fn();
    render(TimelineRow, {
      props: {
        filePath: 'src/lib/store.ts',
        dates: mockDates,
        buckets: mockBuckets,
        maxCount: 15,
        onHover
      }
    });

    const cells = document.querySelectorAll('.heat-cell');
    await fireEvent.mouseEnter(cells[0]);

    expect(onHover).toHaveBeenCalledWith('src/lib/store.ts', '2025-12-28');
  });

  test('calls onLeave when mouse leaves', async () => {
    const onLeave = vi.fn();
    render(TimelineRow, {
      props: {
        filePath: 'src/lib/store.ts',
        dates: mockDates,
        buckets: mockBuckets,
        maxCount: 15,
        onLeave
      }
    });

    const cells = document.querySelectorAll('.heat-cell');
    await fireEvent.mouseLeave(cells[0]);

    expect(onLeave).toHaveBeenCalled();
  });

  test('truncates long file paths', () => {
    render(TimelineRow, {
      props: {
        filePath: 'src/very/long/path/to/deeply/nested/component/file.ts',
        dates: mockDates,
        buckets: mockBuckets,
        maxCount: 15
      }
    });

    const label = document.querySelector('.file-label');
    expect(label).toBeInTheDocument();
  });
});
