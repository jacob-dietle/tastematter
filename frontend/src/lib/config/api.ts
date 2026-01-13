/**
 * API Configuration
 *
 * Centralized API endpoints and settings.
 * All HTTP transport calls use these paths.
 */

export const API_ENDPOINTS = {
  /** Flexible hypercube query for file aggregations */
  queryFlex: '/api/query/flex',

  /** Timeline query for daily activity buckets */
  queryTimeline: '/api/query/timeline',

  /** Sessions query for work sessions */
  querySessions: '/api/query/sessions',

  /** Chains query for conversation chains */
  queryChains: '/api/query/chains',

  /** Health check endpoint */
  health: '/api/health'
} as const;

export type ApiEndpoint = keyof typeof API_ENDPOINTS;

/** HTTP request timeout in milliseconds */
export const REQUEST_TIMEOUT_MS = 30_000;
