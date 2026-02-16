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

// --- Trigger function signature (dependency injection) ---

export type TriggerFn = (
  apiKey: string,
  workflowKey: string,
  payload: KnockTriggerPayload
) => Promise<Result<{ workflow_run_id: string }>>;
