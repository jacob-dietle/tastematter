/**
 * Environment bindings and domain types for outreach worker.
 */

export interface Env {
  DB: D1Database;
  CF_ACCESS_CLIENT_ID: string;
  CF_ACCESS_CLIENT_SECRET: string;
  WEBHOOK_SECRET: string;
}

// ============================================================================
// Domain Types
// ============================================================================

export type ContactStatus =
  | "identified"
  | "contacted"
  | "replied"
  | "installed"
  | "active"
  | "feedback_received"
  | "churned";

export type ContactSource =
  | "kondo_webhook"
  | "kondo_sync"
  | "manual"
  | "batch_import"
  | "linkedin_commenter"
  | "quickstart_fork"
  | "alpha_referral"
  | "diy_builder";

export type EventType =
  | "identified"
  | "dm_sent"
  | "reply_received"
  | "install_confirmed"
  | "feedback_received"
  | "label_changed"
  | "note_updated"
  | "status_override";

export interface Contact {
  id: string;
  linkedin_url: string;
  name: string | null;
  headline: string | null;
  location: string | null;
  source: ContactSource;
  wave: string;
  status: ContactStatus;
  kondo_labels: string | null;
  kondo_notes: string | null;
  kondo_url: string | null;
  last_message_at: string | null;
  last_message_preview: string | null;
  first_contact_at: string | null;
  install_confirmed_at: string | null;
  feedback_count: number;
  created_at: string;
  updated_at: string;
}

export interface OutreachEvent {
  id: number;
  contact_id: string;
  event_type: EventType;
  event_data: string | null;
  source: string;
  created_at: string;
}

// ============================================================================
// Kondo Webhook Payload (actual structure from production test)
// ============================================================================

export interface KondoLabel {
  kondo_label_id: string;
  kondo_label_name: string;
  kondo_labeled_at: string | null;
}

export interface KondoWebhookPayload {
  event?: {
    type?: string;
    timestamp?: number;
  };
  data?: {
    contact_first_name?: string;
    contact_last_name?: string;
    contact_linkedin_handle?: string;
    contact_linkedin_uid?: string;
    contact_linkedin_url?: string;
    contact_headline?: string;
    contact_location?: string;
    contact_picture?: string;
    contact_connected_at?: string;
    contact_connected_by?: string;
    conversation_history?: string;
    conversation_latest_content?: string;
    conversation_latest_timestamp?: string;
    conversation_starred?: boolean;
    conversation_status?: string;
    conversation_state?: string;
    kondo_url?: string;
    kondo_note?: string;
    kondo_labels?: KondoLabel[];
  };
}

// ============================================================================
// API Request/Response Types
// ============================================================================

export interface BatchImportRequest {
  contacts: Array<{
    linkedin_url: string;
    name?: string;
    headline?: string;
    location?: string;
    source?: ContactSource;
    wave?: string;
  }>;
}

export interface ContactUpdateRequest {
  status?: ContactStatus;
  wave?: string;
  name?: string;
  headline?: string;
  notes?: string;
}

export interface DashboardResponse {
  pipeline: Record<ContactStatus, number>;
  by_wave: Record<string, { total: number; installed: number }>;
  by_source: Record<string, number>;
  recent_events: OutreachEvent[];
  total_contacts: number;
  last_updated: string;
}
