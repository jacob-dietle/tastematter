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
