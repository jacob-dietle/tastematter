/**
 * Tastematter Alert Worker - Type Definitions
 * Mirrors spec 17_CONTEXT_ALERTING_AND_PUBLISHING.md type contracts.
 */

// --- Result type (same pattern as nickel conference_pr) ---

export type Result<T> =
  | { success: true; data: T }
  | { success: false; error: string };

// --- Worker environment ---

export interface Env {
  ALERTS_DB: D1Database;
  KNOCK_API_KEY: string;
  OWNER_ID: string;
  // Publishing (Phase 2)
  CONTEXT_DO: DurableObjectNamespace;
  MCP_OBJECT: DurableObjectNamespace;
  CORPUS_BUCKET: R2Bucket;
  ANTHROPIC_API_KEY: string;
}

// --- Corpus types (Phase 2: Context Publishing) ---

export interface CorpusSnapshot {
  version: string;
  commit: string;
  fileCount: number;
  totalSize: number;
  generatedAt: string;
  files: Record<string, FileEntry>;
  allPaths: string[];
}

export interface FileEntry {
  path: string;
  content: string;
  size: number;
  frontmatter?: {
    tags?: string[];
    title?: string;
    description?: string;
    [key: string]: any;
  };
}

export interface GrepOptions {
  caseInsensitive?: boolean;
  contextLines?: number;
  maxResults?: number;
  maxMatchesPerFile?: number;
}

export interface GrepResult {
  path: string;
  matches: MatchDetail[];
  score: number;
}

export interface MatchDetail {
  line: number;
  content: string;
  context?: {
    before: string[];
    after: string[];
  };
}

export interface ListOptions {
  directories?: boolean;
  files?: boolean;
  maxResults?: number;
}

export interface ListResult {
  path: string;
  type: 'directory' | 'file';
  depth: number;
  matchedPattern: string;
}

// --- Query types (Phase 2) ---

export interface QueryResult {
  response: string;
  conversationHistory: any[];
  totalTurns: number;
  duration: number;
  model: string;
}

export interface StreamingQueryResult {
  stream: ReadableStream<Uint8Array>;
  finalResultPromise: Promise<QueryResult>;
}

export interface QueryOptions {
  debug?: boolean;
  onProgress?: (message: string) => void;
  streaming?: boolean;
}

export interface QueryLogRow {
  id: number;
  engagement_id: string;
  timestamp: string;
  query: string;
  response_length: number | null;
  duration_ms: number | null;
  tool_calls: number | null;
  corpus_commit: string | null;
  success: number;
  error_message: string | null;
}

export interface InsertQueryLogInput {
  engagement_id: string;
  query: string;
  response_length?: number;
  duration_ms?: number;
  tool_calls?: number;
  corpus_commit?: string;
  success?: number;
  error_message?: string;
}

// --- D1 row types ---

export interface EngagementRow {
  id: string;
  owner_id: string;
  display_name: string;
  config_json: string;
  created_at: string;
  updated_at: string;
}

export interface AlertHistoryRow {
  id: number;
  engagement_id: string;
  rule_name: string;
  trigger_type: string;
  fired_at: string;
  knock_workflow_run_id: string | null;
  payload: string | null;
  success: number;
  error_message: string | null;
}

export interface AlertStateRow {
  rule_name: string;
  engagement_id: string;
  last_checked_at: string | null;
  last_fired_at: string | null;
  last_corpus_sha: string | null;
  state_data: string | null;
}

export interface ActivityLogRow {
  id: number;
  engagement_id: string | null;
  timestamp: string;
  event_type: string;
  message: string | null;
  details: string | null;
}

// --- Knock types ---

export interface KnockRecipient {
  id: string;
  email?: string;
  name?: string;
}

export interface KnockTriggerPayload {
  recipients: string[];
  data: {
    subject: string;
    body: string;
    html?: string;
    url?: string;
    trigger_type: TriggerType;
    changes?: string[];
    matched_files?: string[];
    metric_value?: number;
    corpus_sha?: string;
  };
}

// --- Alerting config types (from engagement config_json) ---

export type TriggerType =
  | "content_change"
  | "pattern_match"
  | "threshold"
  | "schedule"
  | "corpus_drift";

export type AlertFormat = "instant" | "digest" | "brief";

export type TriggerConfig =
  | { type: "content_change"; paths: string[]; min_changes?: number }
  | { type: "pattern_match"; pattern: string; case_insensitive?: boolean }
  | {
      type: "threshold";
      metric: string;
      operator: ">" | "<" | "=";
      value: number;
    }
  | { type: "schedule" }
  | { type: "corpus_drift"; max_commits_behind?: number };

export interface WatchRule {
  name: string;
  trigger: TriggerType;
  schedule: string;
  config: TriggerConfig;
  channels: string[];
  format: AlertFormat;
  enabled: boolean;
}

export interface AlertingConfig {
  provider: "knock";
  workflow_key: string;
  recipients: KnockRecipient[];
  rules: WatchRule[];
}

// --- DB operation input types ---

export interface InsertAlertHistoryInput {
  engagement_id: string;
  rule_name: string;
  trigger_type: string;
  knock_workflow_run_id?: string;
  payload?: string;
  success?: number;
  error_message?: string;
}

export interface UpsertAlertStateInput {
  rule_name: string;
  engagement_id: string;
  last_checked_at?: string;
  last_fired_at?: string;
  last_corpus_sha?: string;
  state_data?: string;
}

export interface InsertActivityLogInput {
  engagement_id?: string;
  event_type: string;
  message?: string;
  details?: string;
}

// --- /status Contract (what this worker returns to control plane) ---

export interface WorkerStatusResponse {
  identity: { worker: string; display_name: string; system_id?: string; account_id?: string; version?: string };
  vitals: { status: 'ok' | 'degraded' | 'error'; started_at?: string; features?: Record<string, boolean> };
  corpus?: { commit: string; file_count: number; loaded_at: string; source_repo?: string };
  trail?: { last_deposit: string; at: string; type: string; detail?: string };
  d1_health?: { total_executions: number; total_failures: number; failure_rate: string; last_execution?: { status: string; duration_ms: number; at: string }; last_failure?: { error: string; at: string } };
  schedule?: { cron: string; last_run?: string; next_run?: string };
}

// --- Trigger function signature (dependency injection) ---

export type TriggerFn = (
  apiKey: string,
  workflowKey: string,
  payload: KnockTriggerPayload
) => Promise<Result<{ workflow_run_id: string }>>;
