import { describe, test, expect } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import TimelineLegend from '$lib/components/TimelineLegend.svelte';

describe('TimelineLegend', () => {
  test('renders color scale', () => {
    render(TimelineLegend);

    const swatches = document.querySelectorAll('.swatch');
    expect(swatches.length).toBe(4); // empty, low, medium, high
  });

  test('shows Less and More labels', () => {
    render(TimelineLegend);

    expect(screen.getByText('Less')).toBeInTheDocument();
    expect(screen.getByText('More')).toBeInTheDocument();
  });

  test('renders correct colors', () => {
    render(TimelineLegend);

    const swatches = document.querySelectorAll('.swatch');
    expect(swatches[0]).toHaveStyle({ backgroundColor: '#f6f8fa' }); // empty
    expect(swatches[3]).toHaveStyle({ backgroundColor: '#1a1a2e' }); // high
  });
});
