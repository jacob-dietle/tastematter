/**
 * API Index - Phase 3.2
 *
 * Re-exports transport functions with same interface as original tauri.ts
 * This allows stores to simply change their import path.
 *
 * Usage:
 *   // Before: import { queryFlex } from '$lib/api/tauri';
 *   // After:  import { queryFlex } from '$lib/api';
 */
import { createTransport, initializeTransport } from './transport';
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

// Re-export transport utilities
export { createTransport, initializeTransport, isTauriEnvironment } from './transport';
export type { Transport } from './transport';

// Convenience functions that use the singleton transport
// These match the original tauri.ts API for minimal migration effort

export async function queryFlex(args: QueryFlexArgs): Promise<QueryResult> {
  const transport = createTransport();
  return transport.queryFlex(args);
}

export async function queryTimeline(args: TimelineQueryArgs): Promise<TimelineData> {
  const transport = createTransport();
  return transport.queryTimeline(args);
}

export async function querySessions(args: SessionQueryArgs): Promise<SessionQueryResult> {
  const transport = createTransport();
  return transport.querySessions(args);
}

export async function queryChains(args: ChainQueryArgs): Promise<ChainQueryResult> {
  const transport = createTransport();
  return transport.queryChains(args);
}

// Git operations are not supported in HTTP mode (Tauri only)
// Keep using tauri.ts directly for git operations
export { gitStatus, gitPull, gitPush } from './tauri';
