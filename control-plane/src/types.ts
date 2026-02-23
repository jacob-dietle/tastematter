export interface Env {
  DB: D1Database;
  OWNER_ID: string;
  KNOCK_API_KEY: string;
  KNOCK_WORKFLOW_KEY: string;
  CF_ACCESS_CLIENT_ID: string;
  CF_ACCESS_CLIENT_SECRET: string;
}

export type WorkerStatus = "healthy" | "degraded" | "down" | "stale" | "timeout" | "reachable" | "unknown";
export type SystemStatus = "healthy" | "degraded" | "broken" | "unknown";

// --- Worker Registry ---

export interface WorkerRegistryRow {
  id: string;
  display_name: string;
  health_url: string;
  expected_cadence: string | null;
  max_silence_hours: number;
  auth_type: string;
  tags: string | null;
  enabled: number;
  system_id: string | null;
  account_id: string | null;
  status_url: string | null;
  created_at: string;
  updated_at: string;
}

export interface WorkerWithStatus extends WorkerRegistryRow {
  current_status: WorkerStatus;
  last_checked: string | null;
  last_activity: string | null;
  last_response_time_ms: number | null;
  error_message: string | null;
  raw_response: string | null;
}

// --- System Registry ---

export interface SystemRegistryRow {
  id: string;
  display_name: string;
  description: string | null;
  health_rule: string;
  current_status: SystemStatus;
  status_changed_at: string | null;
  created_at: string;
}

export interface SystemWithMembers extends SystemRegistryRow {
  members: WorkerWithStatus[];
}

// --- Health Log ---

export interface HealthLogRow {
  id: number;
  worker_id: string;
  checked_at: string;
  http_status: number | null;
  response_time_ms: number | null;
  status: WorkerStatus;
  last_activity: string | null;
  activity_type: string | null;
  raw_response: string | null;
  error_message: string | null;
}

export interface HealthCheckResult {
  worker_id: string;
  http_status: number | null;
  response_time_ms: number;
  status: WorkerStatus;
  last_activity: string | null;
  activity_type: string | null;
  raw_response: string | null;
  error_message: string | null;
}

// --- /status Contract (what workers return) ---

export interface WorkerStatusResponse {
  identity: {
    worker: string;
    display_name: string;
    system_id?: string;
    account_id?: string;
    version?: string;
  };
  vitals: {
    status: "ok" | "degraded" | "error";
    started_at?: string;
    features?: Record<string, boolean>;
  };
  corpus?: {
    commit: string;
    file_count: number;
    loaded_at: string;
    source_repo?: string;
  };
  trail?: {
    last_deposit: string;
    at: string;
    type: string;
    detail?: string;
  };
  d1_health?: {
    total_executions: number;
    total_failures: number;
    failure_rate: string;
    last_execution?: { status: string; duration_ms: number; at: string };
    last_failure?: { error: string; at: string };
  };
  schedule?: {
    cron: string;
    last_run?: string;
    next_run?: string;
  };
}

// --- Legacy /health response (for fallback) ---

export interface WorkerHealthResponse {
  status: string;
  worker?: string;
  last_activity?: string;
  activity_type?: string;
  [key: string]: unknown;
}

// --- Sync Log ---

export interface SyncLogRow {
  id: number;
  worker_id: string;
  synced_at: string;
  commit_sha: string;
  file_count: number | null;
  source_repo: string | null;
  action_run_url: string | null;
  success: number;
  error_message: string | null;
}

export interface SyncWebhookPayload {
  worker_id: string;
  commit_sha: string;
  file_count?: number;
  source_repo?: string;
  action_run_url?: string;
  success?: boolean;
  error_message?: string;
}

// --- Result type ---

export type Result<T> =
  | { success: true; data: T }
  | { success: false; error: string };
