import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import NotificationBell from '$lib/components/NotificationBell.svelte';

describe('NotificationBell', () => {
  it('renders the bell icon', () => {
    render(NotificationBell, { props: { unreadCount: 0 } });
    const link = screen.getByLabelText('Notifications');
    expect(link).toBeDefined();
    expect(link.querySelector('svg')).toBeDefined();
  });

  it('shows badge when unreadCount > 0', () => {
    render(NotificationBell, { props: { unreadCount: 5 } });
    const badge = screen.getByTestId('badge');
    expect(badge).toBeDefined();
    expect(badge.textContent).toBe('5');
  });

  it('hides badge when unreadCount is 0', () => {
    render(NotificationBell, { props: { unreadCount: 0 } });
    const badge = screen.queryByTestId('badge');
    expect(badge).toBeNull();
  });
});
