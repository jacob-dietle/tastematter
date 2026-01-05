import { invoke } from '@tauri-apps/api/core';
import type { QueryFlexArgs, QueryResult, CommandError, GitStatus, GitOpResult, TimelineQueryArgs, TimelineData, SessionQueryArgs, SessionQueryResult, ChainQueryArgs, ChainQueryResult } from '$lib/types';
import { logService } from '$lib/logging';

// Generic logged invoke wrapper
export async function invokeLogged<T>(
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

export async function queryFlex(args: QueryFlexArgs): Promise<QueryResult> {
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
    // Tauri returns errors as strings or objects, normalize to CommandError
    if (typeof error === 'string') {
      throw { code: 'INVOKE_ERROR', message: error } as CommandError;
    }
    throw error;
  }
}

export async function gitStatus(): Promise<GitStatus> {
  try {
    return await invokeLogged<GitStatus>('git_status');
  } catch (error) {
    if (typeof error === 'string') {
      throw { code: 'GIT_ERROR', message: error } as CommandError;
    }
    throw error;
  }
}

export async function gitPull(): Promise<GitOpResult> {
  try {
    return await invokeLogged<GitOpResult>('git_pull');
  } catch (error) {
    if (typeof error === 'string') {
      throw { code: 'GIT_ERROR', message: error } as CommandError;
    }
    throw error;
  }
}

export async function gitPush(): Promise<GitOpResult> {
  try {
    return await invokeLogged<GitOpResult>('git_push');
  } catch (error) {
    if (typeof error === 'string') {
      throw { code: 'GIT_ERROR', message: error } as CommandError;
    }
    throw error;
  }
}

export async function queryTimeline(args: TimelineQueryArgs): Promise<TimelineData> {
  try {
    return await invokeLogged<TimelineData>('query_timeline', {
      time: args.time,
      files: args.files,
      limit: args.limit,
    });
  } catch (error) {
    if (typeof error === 'string') {
      throw { code: 'TIMELINE_ERROR', message: error } as CommandError;
    }
    throw error;
  }
}

export async function querySessions(args: SessionQueryArgs): Promise<SessionQueryResult> {
  try {
    return await invokeLogged<SessionQueryResult>('query_sessions', {
      time: args.time,
      chain: args.chain,
      limit: args.limit,
    });
  } catch (error) {
    if (typeof error === 'string') {
      throw { code: 'SESSION_ERROR', message: error } as CommandError;
    }
    throw error;
  }
}

export async function queryChains(args: ChainQueryArgs): Promise<ChainQueryResult> {
  try {
    return await invokeLogged<ChainQueryResult>('query_chains', {
      limit: args.limit,
    });
  } catch (error) {
    if (typeof error === 'string') {
      throw { code: 'CHAIN_ERROR', message: error } as CommandError;
    }
    throw error;
  }
}
