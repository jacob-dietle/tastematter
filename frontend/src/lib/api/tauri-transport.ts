/**
 * Tauri Transport - Phase 3.2
 *
 * Tauri IPC transport for desktop app mode.
 * Uses @tauri-apps/api/core invoke for communication.
 */
import { invoke } from '@tauri-apps/api/core';
import type { Transport } from './transport';
import type {
  QueryFlexArgs,
  QueryResult,
  TimelineQueryArgs,
  TimelineData,
  SessionQueryArgs,
  SessionQueryResult,
  ChainQueryArgs,
  ChainQueryResult,
  CommandError
} from '$lib/types';
import { logService } from '$lib/logging';

/**
 * Generic logged invoke wrapper
 */
async function invokeLogged<T>(
  command: string,
  args: Record<string, unknown> = {}
): Promise<T> {
  const correlationId = logService.getCorrelationId();
  const start = performance.now();

  try {
    const result = await invoke<T>(command, {
      ...args,
      correlation_id: correlationId
    });

    const duration = Math.round(performance.now() - start);

    await logService.log({
      component: 'ipc',
      operation: command,
      duration_ms: duration,
      success: true,
      context: {
        args: sanitizeArgs(args),
        result_summary: summarizeResult(result)
      }
    });

    return result;
  } catch (error) {
    const duration = Math.round(performance.now() - start);

    await logService.log({
      level: 'error',
      component: 'ipc',
      operation: command,
      duration_ms: duration,
      success: false,
      context: { args: sanitizeArgs(args) },
      error: {
        type: error instanceof Error ? error.constructor.name : 'Error',
        message: String(error)
      }
    });

    throw error;
  }
}

function sanitizeArgs(args: Record<string, unknown>): Record<string, unknown> {
  const sanitized: Record<string, unknown> = {};
  const sensitiveKeys = ['password', 'token', 'api_key', 'secret', 'credential'];

  for (const [key, value] of Object.entries(args)) {
    if (key === 'correlation_id') continue;

    if (sensitiveKeys.some(k => key.toLowerCase().includes(k))) {
      sanitized[key] = '[REDACTED]';
      continue;
    }

    if (typeof value === 'string' && value.length > 100) {
      sanitized[key] = value.slice(0, 100) + '...';
      continue;
    }

    sanitized[key] = value;
  }

  return sanitized;
}

function summarizeResult(result: unknown): string {
  if (result === null || result === undefined) return 'null';
  if (Array.isArray(result)) return `${result.length} items`;
  if (typeof result === 'object') {
    const obj = result as Record<string, unknown>;
    if ('count' in obj) return `count: ${obj.count}`;
    if ('length' in obj) return `length: ${obj.length}`;
    if ('result_count' in obj) return `result_count: ${obj.result_count}`;
    if ('results' in obj && Array.isArray(obj.results)) return `${obj.results.length} results`;
    return `object with ${Object.keys(obj).length} keys`;
  }
  return typeof result;
}

function normalizeError(error: unknown): CommandError {
  if (typeof error === 'string') {
    return { code: 'INVOKE_ERROR', message: error };
  }
  return error as CommandError;
}

/**
 * Create Tauri transport for desktop app mode.
 */
export function createTauriTransport(): Transport {
  return {
    async queryFlex(args: QueryFlexArgs): Promise<QueryResult> {
      try {
        return await invokeLogged<QueryResult>('query_flex', {
          files: args.files,
          time: args.time,
          chain: args.chain,
          session: args.session,
          agg: args.agg,
          limit: args.limit,
          sort: args.sort,
        });
      } catch (error) {
        throw normalizeError(error);
      }
    },

    async queryTimeline(args: TimelineQueryArgs): Promise<TimelineData> {
      try {
        return await invokeLogged<TimelineData>('query_timeline', {
          time: args.time,
          files: args.files,
          chain: args.chain,
          limit: args.limit,
        });
      } catch (error) {
        throw normalizeError(error);
      }
    },

    async querySessions(args: SessionQueryArgs): Promise<SessionQueryResult> {
      try {
        return await invokeLogged<SessionQueryResult>('query_sessions', {
          time: args.time,
          chain: args.chain,
          limit: args.limit,
        });
      } catch (error) {
        throw normalizeError(error);
      }
    },

    async queryChains(args: ChainQueryArgs): Promise<ChainQueryResult> {
      try {
        return await invokeLogged<ChainQueryResult>('query_chains', {
          limit: args.limit,
        });
      } catch (error) {
        throw normalizeError(error);
      }
    }
  };
}
