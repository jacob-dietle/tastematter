/**
 * HTTP Transport - Phase 3.2
 *
 * HTTP-based transport for browser development mode.
 * Makes fetch calls to the context-os HTTP server.
 *
 * Usage:
 *   const transport = createHttpTransport();
 *   const result = await transport.queryFlex({ time: '7d', agg: ['count'] });
 */
import type { Transport } from './transport';
import type {
  QueryFlexArgs,
  QueryResult,
  TimelineQueryArgs,
  TimelineData,
  SessionQueryArgs,
  SessionQueryResult,
  ChainQueryArgs,
  ChainQueryResult
} from '$lib/types';
import { API_ENDPOINTS, REQUEST_TIMEOUT_MS } from '$lib/config';

/**
 * Parse error response safely - handles HTML error pages and malformed JSON.
 */
async function parseErrorResponse(response: Response, fallbackMessage: string): Promise<string> {
  try {
    const error = await response.json();
    return error.message || fallbackMessage;
  } catch {
    // Server returned non-JSON (e.g., HTML error page)
    return `${fallbackMessage} (HTTP ${response.status}: ${response.statusText})`;
  }
}

/**
 * Make a POST request with timeout and proper error handling.
 */
async function postWithTimeout<T>(
  url: string,
  body: unknown,
  fallbackError: string
): Promise<T> {
  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), REQUEST_TIMEOUT_MS);

  try {
    const response = await fetch(url, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
      signal: controller.signal
    });

    if (!response.ok) {
      const message = await parseErrorResponse(response, fallbackError);
      throw new Error(message);
    }

    return response.json();
  } catch (error) {
    if (error instanceof Error && error.name === 'AbortError') {
      throw new Error(`Request timed out after ${REQUEST_TIMEOUT_MS / 1000}s`);
    }
    throw error;
  } finally {
    clearTimeout(timeoutId);
  }
}

/**
 * Create HTTP transport for browser mode.
 * All requests go through Vite proxy to context-os server.
 */
export function createHttpTransport(): Transport {
  return {
    async queryFlex(args: QueryFlexArgs): Promise<QueryResult> {
      return postWithTimeout(API_ENDPOINTS.queryFlex, args, 'Query failed');
    },

    async queryTimeline(args: TimelineQueryArgs): Promise<TimelineData> {
      return postWithTimeout(API_ENDPOINTS.queryTimeline, args, 'Timeline query failed');
    },

    async querySessions(args: SessionQueryArgs): Promise<SessionQueryResult> {
      return postWithTimeout(API_ENDPOINTS.querySessions, args, 'Sessions query failed');
    },

    async queryChains(args: ChainQueryArgs): Promise<ChainQueryResult> {
      return postWithTimeout(API_ENDPOINTS.queryChains, args, 'Chains query failed');
    }
  };
}
