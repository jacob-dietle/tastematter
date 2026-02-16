-- Engagement config (D1-backed, not local YAML)
CREATE TABLE IF NOT EXISTS engagements (
  id TEXT PRIMARY KEY,
  owner_id TEXT NOT NULL,
  display_name TEXT NOT NULL,
  config_json TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_engagements_owner ON engagements(owner_id);

-- Alert tracking
CREATE TABLE IF NOT EXISTS alert_history (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  engagement_id TEXT NOT NULL,
  rule_name TEXT NOT NULL,
  trigger_type TEXT NOT NULL,
  fired_at TEXT NOT NULL DEFAULT (datetime('now')),
  knock_workflow_run_id TEXT,
  payload TEXT,
  success INTEGER DEFAULT 1,
  error_message TEXT
);

-- Alert state (per-rule tracking)
CREATE TABLE IF NOT EXISTS alert_state (
  rule_name TEXT PRIMARY KEY,
  engagement_id TEXT NOT NULL,
  last_checked_at TEXT,
  last_fired_at TEXT,
  last_corpus_sha TEXT,
  state_data TEXT
);

-- Activity log (shared)
CREATE TABLE IF NOT EXISTS activity_log (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  engagement_id TEXT,
  timestamp TEXT NOT NULL DEFAULT (datetime('now')),
  event_type TEXT NOT NULL,
  message TEXT,
  details TEXT
);
