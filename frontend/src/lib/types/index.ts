// IPC Types for Tastematter
// Matches Rust structs in src-tauri/src/commands.rs

export interface QueryFlexArgs {
  files?: string;
  time?: string;
  chain?: string;
  session?: string;
  agg: string[];
  limit?: number;
  sort?: 'count' | 'recency' | 'alpha';
}

export interface QueryResult {
  receipt_id: string;
  timestamp: string;
  result_count: number;
  results: FileResult[];
  aggregations: {
    count?: { total_files: number; total_accesses: number };
    recency?: { newest: string; oldest: string };
  };
}

export interface FileResult {
  file_path: string;
  access_count: number;
  last_access: string | null;
  session_count?: number;
  sessions?: string[];
  chains?: string[];
}

export interface CommandError {
  code: string;
  message: string;
  details?: string;
}

export interface QueryState {
  loading: boolean;
  data: QueryResult | null;
  error: CommandError | null;
  lastQuery: QueryFlexArgs | null;
}

// Phase 2: Heat Map View types

export type ViewMode = 'table' | 'heatmap';
export type Granularity = 'file' | 'directory';

export interface DirectoryResult {
  directory_path: string;
  file_count: number;
  total_access_count: number;
  max_access_count: number;
  last_access: string | null;
  files: FileResult[];
}

export interface HeatMapRowProps {
  label: string;
  accessCount: number;
  maxAccessCount: number;
  lastAccess: string | null;
  isDirectory: boolean;
  onclick?: () => void;
}

// Phase 3: Git Panel types

export interface GitStatus {
  branch: string;
  ahead: number;              // Commits ahead of remote
  behind: number;             // Commits behind remote
  staged: string[];           // Staged file paths
  modified: string[];         // Unstaged modifications
  untracked: string[];        // Untracked files
  has_conflicts: boolean;
}

export interface GitOpResult {
  success: boolean;
  message: string;            // Human-readable result
  error?: string;             // Error details if failed
  files_affected?: number;
}

export interface GitState {
  loading: boolean;
  data: GitStatus | null;
  error: CommandError | null;
  isPulling: boolean;
  isPushing: boolean;
  lastOperation: GitOpResult | null;
}

// Phase 4: Timeline View types

/**
 * Temporal bucket for timeline visualization.
 * Groups file accesses by time period.
 */
export interface TimeBucket {
  date: string;           // ISO date: "2025-12-29"
  day_of_week: string;    // "Mon", "Tue", etc.
  access_count: number;   // Total accesses in bucket
  files_touched: number;  // Unique files in bucket
  sessions: string[];     // Session IDs active in bucket
}

/**
 * File activity across time buckets.
 * One row in the timeline visualization.
 */
export interface FileTimeline {
  file_path: string;
  total_accesses: number;
  buckets: Record<string, number>;  // date -> access_count
  first_access: string;   // ISO datetime
  last_access: string;    // ISO datetime
}

/**
 * Result of temporal query for timeline view.
 */
export interface TimelineData {
  time_range: string;     // "7d", "14d", "30d"
  start_date: string;     // ISO date
  end_date: string;       // ISO date
  buckets: TimeBucket[];  // One per day
  files: FileTimeline[];  // Sorted by total_accesses desc
  summary: {
    total_accesses: number;
    total_files: number;
    peak_day: string;     // Date with most activity
    peak_count: number;
  };
}

/**
 * Timeline query parameters.
 */
export interface TimelineQueryArgs {
  time: string;           // "7d", "14d", "30d"
  files?: string;         // Optional file pattern filter
  chain?: string;         // Optional chain ID filter
  limit?: number;         // Max files to return (default: 30)
}

/**
 * Timeline store state.
 */
export interface TimelineState {
  loading: boolean;
  data: TimelineData | null;
  error: CommandError | null;
  selectedRange: '7d' | '14d' | '30d';
  hoveredCell: { file: string; date: string } | null;
}

// Phase 5: Session View types

/**
 * File within a session.
 */
export interface SessionFile {
  file_path: string;
  access_count: number;
  access_types: string[];    // ["read", "write"]
  last_access: string;       // ISO datetime
}

/**
 * Session data from CLI query.
 * One card in the session view.
 */
export interface SessionData {
  session_id: string;
  chain_id: string | null;
  started_at: string;        // ISO datetime
  ended_at: string | null;   // ISO datetime (null if ongoing)
  duration_seconds: number | null;
  file_count: number;
  total_accesses: number;
  files: SessionFile[];      // All files in session
  top_files: SessionFile[];  // Top 3 by access count
}

/**
 * Chain summary for filtering/grouping.
 */
export interface ChainSummary {
  chain_id: string;
  session_count: number;
  file_count: number;
  last_active: string;
}

/**
 * Result of session query.
 */
export interface SessionQueryResult {
  time_range: string;        // "7d", "14d", "30d"
  sessions: SessionData[];
  chains: ChainSummary[];    // Unique chains in result
  summary: {
    total_sessions: number;
    total_files: number;
    total_accesses: number;
    active_chains: number;
  };
}

/**
 * Session query parameters.
 */
export interface SessionQueryArgs {
  time: string;              // "7d", "14d", "30d"
  chain?: string;            // Filter by chain ID
  limit?: number;            // Max sessions (default: 50)
}

/**
 * Session store state.
 */
export interface SessionState {
  loading: boolean;
  data: SessionQueryResult | null;
  error: CommandError | null;
  selectedRange: '7d' | '14d' | '30d';
  expandedSessions: Set<string>;  // session_ids with expanded trees
  selectedChain: string | null;   // Chain filter
}

/**
 * Directory tree node for SessionFileTree component.
 * Used to visualize file hierarchy within a session.
 */
export interface DirectoryNode {
  name: string;
  path: string;
  type: 'file' | 'directory';
  access_count: number;
  children?: DirectoryNode[];
  expanded?: boolean;
}

// Chain types

/**
 * Time range for a chain.
 */
export interface ChainTimeRange {
  start: string;  // ISO datetime
  end: string;    // ISO datetime
}

/**
 * Chain data from CLI query.
 * Represents a conversation chain across sessions.
 */
export interface ChainData {
  chain_id: string;
  session_count: number;
  file_count: number;
  time_range: ChainTimeRange | null;
}

/**
 * Result of chain query.
 */
export interface ChainQueryResult {
  chains: ChainData[];
  total_chains: number;
}

/**
 * Chain query parameters.
 */
export interface ChainQueryArgs {
  limit?: number;  // Default: 20
}

/**
 * Chain store state.
 */
export interface ChainState {
  loading: boolean;
  data: ChainQueryResult | null;
  error: CommandError | null;
  selectedChain: string | null;
}
