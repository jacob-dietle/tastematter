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
