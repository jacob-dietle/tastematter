import { describe, it, expect, vi, beforeEach } from 'vitest';
import { initKnock, getKnock, resetKnock } from '$lib/knock';

vi.mock('@knocklabs/client', () => {
  const authenticate = vi.fn();
  return {
    default: vi.fn().mockImplementation(() => ({
      authenticate,
    })),
  };
});

describe('knock client', () => {
  beforeEach(() => {
    resetKnock();
  });

  it('creates and authenticates a Knock instance', () => {
    const knock = initKnock('pk_test_123', 'user-1');
    expect(knock).toBeDefined();
    expect(knock.authenticate).toHaveBeenCalledWith('user-1');
  });

  it('returns the same instance on subsequent calls', () => {
    const first = initKnock('pk_test_123', 'user-1');
    const second = initKnock('pk_test_456', 'user-2');
    expect(first).toBe(second);
  });

  it('getKnock returns null before init', () => {
    expect(getKnock()).toBeNull();
  });

  it('getKnock returns instance after init', () => {
    const knock = initKnock('pk_test_123', 'user-1');
    expect(getKnock()).toBe(knock);
  });

  it('resetKnock clears the instance', () => {
    initKnock('pk_test_123', 'user-1');
    resetKnock();
    expect(getKnock()).toBeNull();
  });
});
