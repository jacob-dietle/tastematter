import { describe, test, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import ChainBadge from '$lib/components/ChainBadge.svelte';

describe('ChainBadge', () => {
  test('renders "No chain" when chainId is null', () => {
    render(ChainBadge, { props: { chainId: null } });

    expect(screen.getByText('No chain')).toBeInTheDocument();
  });

  test('renders truncated chain ID (first 8 chars)', () => {
    render(ChainBadge, { props: { chainId: 'abc123def456' } });

    expect(screen.getByText('abc123de')).toBeInTheDocument();
  });

  test('shows muted color when chainId is null', () => {
    render(ChainBadge, { props: { chainId: null } });

    const badge = screen.getByTestId('chain-badge');
    // CSS variable fallback resolves to #6a737d in test environment
    const style = badge.getAttribute('style');
    expect(style).toContain('--badge-color:');
    expect(style).toMatch(/--badge-color:\s*(var\(--color-muted,\s*#6a737d\)|#6a737d)/);
  });

  test('generates consistent HSL color from chain ID', () => {
    render(ChainBadge, { props: { chainId: 'test-chain-id' } });

    const badge = screen.getByTestId('chain-badge');
    // Color should be HSL with 40% saturation, 45% lightness
    const style = badge.getAttribute('style');
    expect(style).toMatch(/--badge-color:\s*hsl\(\d+,\s*40%,\s*45%\)/);
  });

  test('calls onClick with chain ID when clicked', async () => {
    const onClick = vi.fn();
    render(ChainBadge, { props: { chainId: 'my-chain', onClick } });

    await fireEvent.click(screen.getByTestId('chain-badge'));

    expect(onClick).toHaveBeenCalledWith('my-chain');
  });

  test('does not call onClick when chainId is null', async () => {
    const onClick = vi.fn();
    render(ChainBadge, { props: { chainId: null, onClick } });

    await fireEvent.click(screen.getByTestId('chain-badge'));

    expect(onClick).not.toHaveBeenCalled();
  });

  test('is disabled when no onClick prop', () => {
    render(ChainBadge, { props: { chainId: 'some-chain' } });

    const badge = screen.getByTestId('chain-badge');
    expect(badge).toBeDisabled();
  });

  test('is disabled when chainId is null', () => {
    const onClick = vi.fn();
    render(ChainBadge, { props: { chainId: null, onClick } });

    const badge = screen.getByTestId('chain-badge');
    expect(badge).toBeDisabled();
  });

  test('has clickable class when onClick and chainId present', () => {
    const onClick = vi.fn();
    render(ChainBadge, { props: { chainId: 'chain-123', onClick } });

    const badge = screen.getByTestId('chain-badge');
    expect(badge).toHaveClass('clickable');
  });

  test('does not have clickable class when chainId is null', () => {
    const onClick = vi.fn();
    render(ChainBadge, { props: { chainId: null, onClick } });

    const badge = screen.getByTestId('chain-badge');
    expect(badge).not.toHaveClass('clickable');
  });
});
