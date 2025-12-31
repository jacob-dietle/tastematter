import { describe, test, expect } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import TimelineAxis from '$lib/components/TimelineAxis.svelte';
import type { TimeBucket } from '$lib/types';

const mockBuckets: TimeBucket[] = [
  { date: '2025-12-28', day_of_week: 'Sat', access_count: 5, files_touched: 2, sessions: [] },
  { date: '2025-12-29', day_of_week: 'Sun', access_count: 10, files_touched: 4, sessions: [] },
  { date: '2025-12-30', day_of_week: 'Mon', access_count: 25, files_touched: 8, sessions: ['s1'] },
];

describe('TimelineAxis', () => {
  test('renders date labels for each bucket', () => {
    render(TimelineAxis, { props: { buckets: mockBuckets } });

    expect(screen.getByText('28')).toBeInTheDocument();
    expect(screen.getByText('29')).toBeInTheDocument();
    expect(screen.getByText('30')).toBeInTheDocument();
  });

  test('renders day of week abbreviation', () => {
    render(TimelineAxis, { props: { buckets: mockBuckets } });

    // Now shows first letter of day (S, S, M)
    const dayLabels = document.querySelectorAll('.day-of-week');
    expect(dayLabels.length).toBe(3);
    expect(dayLabels[0]).toHaveTextContent('S'); // Sat
    expect(dayLabels[1]).toHaveTextContent('S'); // Sun
    expect(dayLabels[2]).toHaveTextContent('M'); // Mon
  });

  test('handles empty buckets', () => {
    render(TimelineAxis, { props: { buckets: [] } });

    // Should render container but no date cells
    const container = document.querySelector('.timeline-axis');
    expect(container).toBeInTheDocument();
  });

  test('renders in correct order', () => {
    render(TimelineAxis, { props: { buckets: mockBuckets } });

    // Dates should render in order
    const dates = document.querySelectorAll('.date');
    expect(dates[0]).toHaveTextContent('28');
    expect(dates[1]).toHaveTextContent('29');
    expect(dates[2]).toHaveTextContent('30');
  });
});
