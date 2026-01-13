/**
 * Transport Abstraction Tests - Phase 3.2
 *
 * TDD: RED phase - these tests MUST FAIL initially
 * Tests the transport layer that enables both Tauri IPC and HTTP modes.
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

// These imports will fail initially - that's the RED phase
import {
  createTransport,
  isTauriEnvironment,
  type Transport
} from '$lib/api/transport';
import { createHttpTransport } from '$lib/api/http-transport';
import type { QueryFlexArgs, QueryResult, TimelineQueryArgs, TimelineData, SessionQueryArgs, SessionQueryResult, ChainQueryArgs, ChainQueryResult } from '$lib/types';

describe('Transport Interface', () => {
  describe('isTauriEnvironment', () => {
    it('should return true when window.__TAURI__ exists', () => {
      // Mock Tauri environment
      (globalThis as any).window = { __TAURI__: {} };
      expect(isTauriEnvironment()).toBe(true);
    });

    it('should return false when window.__TAURI__ is undefined', () => {
      // Mock browser environment
      (globalThis as any).window = {};
      expect(isTauriEnvironment()).toBe(false);
    });

    it('should return false when window is undefined', () => {
      delete (globalThis as any).window;
      expect(isTauriEnvironment()).toBe(false);
    });
  });

  describe('createTransport', () => {
    it('should return transport with all required methods', () => {
      const transport = createTransport();

      expect(transport).toHaveProperty('queryFlex');
      expect(transport).toHaveProperty('queryTimeline');
      expect(transport).toHaveProperty('querySessions');
      expect(transport).toHaveProperty('queryChains');
      expect(typeof transport.queryFlex).toBe('function');
      expect(typeof transport.queryTimeline).toBe('function');
      expect(typeof transport.querySessions).toBe('function');
      expect(typeof transport.queryChains).toBe('function');
    });
  });
});

describe('HTTP Transport', () => {
  let fetchMock: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    fetchMock = vi.fn();
    globalThis.fetch = fetchMock;
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('queryFlex', () => {
    it('should POST to /api/query/flex with correct body', async () => {
      const mockResult: QueryResult = {
        receipt_id: 'test-123',
        timestamp: '2026-01-09T00:00:00Z',
        result_count: 1,
        results: [],
        aggregations: {}
      };

      fetchMock.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve(mockResult)
      });

      const transport = createHttpTransport();
      const args: QueryFlexArgs = { time: '7d', agg: ['count'], limit: 10 };

      const result = await transport.queryFlex(args);

      expect(fetchMock).toHaveBeenCalledWith('/api/query/flex', expect.objectContaining({
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ time: '7d', agg: ['count'], limit: 10 })
      }));
      expect(result).toEqual(mockResult);
    });

    it('should throw on non-OK response', async () => {
      fetchMock.mockResolvedValueOnce({
        ok: false,
        status: 500,
        json: () => Promise.resolve({ error: 'QueryError', message: 'Database error' })
      });

      const transport = createHttpTransport();

      await expect(transport.queryFlex({ agg: [] })).rejects.toThrow();
    });
  });

  describe('queryTimeline', () => {
    it('should POST to /api/query/timeline with correct body', async () => {
      const mockResult: TimelineData = {
        time_range: '7d',
        start_date: '2026-01-02',
        end_date: '2026-01-09',
        buckets: [],
        files: [],
        summary: { total_accesses: 0, total_files: 0, peak_day: '', peak_count: 0 }
      };

      fetchMock.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve(mockResult)
      });

      const transport = createHttpTransport();
      const args: TimelineQueryArgs = { time: '7d' };

      const result = await transport.queryTimeline(args);

      expect(fetchMock).toHaveBeenCalledWith('/api/query/timeline', expect.objectContaining({
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ time: '7d' })
      }));
      expect(result).toEqual(mockResult);
    });
  });

  describe('querySessions', () => {
    it('should POST to /api/query/sessions with correct body', async () => {
      const mockResult: SessionQueryResult = {
        time_range: '7d',
        sessions: [],
        chains: [],
        summary: { total_sessions: 0, total_files: 0, total_accesses: 0, active_chains: 0 }
      };

      fetchMock.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve(mockResult)
      });

      const transport = createHttpTransport();
      const args: SessionQueryArgs = { time: '7d', limit: 20 };

      const result = await transport.querySessions(args);

      expect(fetchMock).toHaveBeenCalledWith('/api/query/sessions', expect.objectContaining({
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ time: '7d', limit: 20 })
      }));
      expect(result).toEqual(mockResult);
    });
  });

  describe('queryChains', () => {
    it('should POST to /api/query/chains with correct body', async () => {
      const mockResult: ChainQueryResult = {
        chains: [],
        total_chains: 0
      };

      fetchMock.mockResolvedValueOnce({
        ok: true,
        json: () => Promise.resolve(mockResult)
      });

      const transport = createHttpTransport();
      const args: ChainQueryArgs = { limit: 10 };

      const result = await transport.queryChains(args);

      expect(fetchMock).toHaveBeenCalledWith('/api/query/chains', expect.objectContaining({
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ limit: 10 })
      }));
      expect(result).toEqual(mockResult);
    });
  });
});

describe('Transport Factory', () => {
  it('should return HTTP transport when not in Tauri environment', () => {
    // Ensure not in Tauri
    (globalThis as any).window = {};

    const transport = createTransport();

    // HTTP transport should be used - verify by checking it has the expected methods
    expect(transport).toBeDefined();
    expect(typeof transport.queryFlex).toBe('function');
  });
});
