/**
 * Transport Abstraction - Phase 3.2
 *
 * Enables the frontend to work in both:
 * - Tauri mode (desktop app via IPC)
 * - Browser mode (dev server via HTTP)
 *
 * Auto-detects environment and returns appropriate transport.
 */
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

/**
 * Transport interface - common API for both Tauri and HTTP transports.
 */
export interface Transport {
  queryFlex(args: QueryFlexArgs): Promise<QueryResult>;
  queryTimeline(args: TimelineQueryArgs): Promise<TimelineData>;
  querySessions(args: SessionQueryArgs): Promise<SessionQueryResult>;
  queryChains(args: ChainQueryArgs): Promise<ChainQueryResult>;
}

/**
 * Detect if running in Tauri environment.
 * Checks for window.__TAURI__ which Tauri injects.
 */
export function isTauriEnvironment(): boolean {
  try {
    return typeof window !== 'undefined' && '__TAURI__' in window;
  } catch {
    return false;
  }
}

// Cached transport instance
let transportInstance: Transport | null = null;
let transportInitPromise: Promise<Transport> | null = null;

/**
 * Create transport based on environment.
 * - In Tauri: Uses IPC (invoke)
 * - In browser: Uses HTTP (fetch)
 *
 * Returns cached instance for performance.
 * For synchronous access in non-Tauri environments.
 */
export function createTransport(): Transport {
  // In browser mode, return HTTP transport immediately
  if (!isTauriEnvironment()) {
    if (!transportInstance) {
      transportInstance = createHttpTransportInternal();
    }
    return transportInstance;
  }

  // In Tauri mode, we need async import but return sync
  // The first call may return HTTP transport while Tauri loads
  if (transportInstance) {
    return transportInstance;
  }

  // Fallback to HTTP while Tauri loads (rare edge case)
  return createHttpTransportInternal();
}

/**
 * Initialize transport asynchronously (for Tauri support).
 * Call this early in app startup for best results.
 */
export async function initializeTransport(): Promise<Transport> {
  if (transportInstance) {
    return transportInstance;
  }

  if (transportInitPromise) {
    return transportInitPromise;
  }

  transportInitPromise = (async () => {
    if (isTauriEnvironment()) {
      try {
        const { createTauriTransport } = await import('./tauri-transport');
        transportInstance = createTauriTransport();
      } catch {
        // Fallback to HTTP if Tauri import fails
        transportInstance = createHttpTransportInternal();
      }
    } else {
      transportInstance = createHttpTransportInternal();
    }
    return transportInstance;
  })();

  return transportInitPromise;
}

/**
 * Internal HTTP transport factory.
 * Exported separately for direct use in tests.
 */
function createHttpTransportInternal(): Transport {
  return {
    async queryFlex(args: QueryFlexArgs): Promise<QueryResult> {
      const response = await fetch('/api/query/flex', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(args)
      });

      if (!response.ok) {
        const error = await response.json();
        throw new Error(error.message || 'Query failed');
      }

      return response.json();
    },

    async queryTimeline(args: TimelineQueryArgs): Promise<TimelineData> {
      const response = await fetch('/api/query/timeline', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(args)
      });

      if (!response.ok) {
        const error = await response.json();
        throw new Error(error.message || 'Timeline query failed');
      }

      return response.json();
    },

    async querySessions(args: SessionQueryArgs): Promise<SessionQueryResult> {
      const response = await fetch('/api/query/sessions', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(args)
      });

      if (!response.ok) {
        const error = await response.json();
        throw new Error(error.message || 'Sessions query failed');
      }

      return response.json();
    },

    async queryChains(args: ChainQueryArgs): Promise<ChainQueryResult> {
      const response = await fetch('/api/query/chains', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(args)
      });

      if (!response.ok) {
        const error = await response.json();
        throw new Error(error.message || 'Chains query failed');
      }

      return response.json();
    }
  };
}
