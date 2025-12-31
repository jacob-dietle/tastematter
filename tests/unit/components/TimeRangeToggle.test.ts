import { describe, test, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import TimeRangeToggle from '$lib/components/TimeRangeToggle.svelte';

describe('TimeRangeToggle', () => {
  test('renders default options', () => {
    render(TimeRangeToggle);

    expect(screen.getByText('7d')).toBeInTheDocument();
    expect(screen.getByText('14d')).toBeInTheDocument();
    expect(screen.getByText('30d')).toBeInTheDocument();
  });

  test('renders custom options', () => {
    render(TimeRangeToggle, { props: { options: ['1d', '3d', '7d'] } });

    expect(screen.getByText('1d')).toBeInTheDocument();
    expect(screen.getByText('3d')).toBeInTheDocument();
    expect(screen.getByText('7d')).toBeInTheDocument();
    expect(screen.queryByText('14d')).not.toBeInTheDocument();
  });

  test('shows selected state', () => {
    render(TimeRangeToggle, { props: { selected: '14d' } });

    const selectedButton = screen.getByText('14d');
    expect(selectedButton).toHaveClass('selected');
  });

  test('calls onchange when clicked', async () => {
    const onchange = vi.fn();
    render(TimeRangeToggle, { props: { onchange } });

    await fireEvent.click(screen.getByText('14d'));

    expect(onchange).toHaveBeenCalledWith('14d');
  });

  test('respects disabled prop', () => {
    render(TimeRangeToggle, { props: { disabled: true } });

    const buttons = screen.getAllByRole('button');
    buttons.forEach(button => {
      expect(button).toBeDisabled();
    });
  });
});
