/**
 * Query Configuration
 *
 * Centralized query limits and defaults.
 * Change these values to tune pagination and performance.
 */

export const QUERY_LIMITS = {
  /** Number of chains to load in context store */
  chains: 50,

  /** Number of files to return in file queries */
  files: 50,

  /** Number of timeline buckets/days to return */
  timeline: 30,

  /** Number of sessions to return per query */
  sessions: 50,

  /** Default limit when not specified */
  default: 100
} as const;

export type QueryLimitKey = keyof typeof QUERY_LIMITS;
