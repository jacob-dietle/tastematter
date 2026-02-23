-- Control Plane v2: System Grouping + Sync Tracking + /status Support

-- System registry — groups of workers that coordinate
CREATE TABLE IF NOT EXISTS system_registry (
  id TEXT PRIMARY KEY,
  display_name TEXT NOT NULL,
  description TEXT,
  health_rule TEXT DEFAULT 'all',
  current_status TEXT DEFAULT 'unknown',
  status_changed_at TEXT,
  created_at TEXT DEFAULT (datetime('now'))
);

-- Extend worker_registry with system membership and account info
ALTER TABLE worker_registry ADD COLUMN system_id TEXT;
ALTER TABLE worker_registry ADD COLUMN account_id TEXT;
ALTER TABLE worker_registry ADD COLUMN status_url TEXT;

-- Sync log — tracks push webhooks from GitHub Actions
CREATE TABLE IF NOT EXISTS sync_log (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  worker_id TEXT NOT NULL,
  synced_at TEXT DEFAULT (datetime('now')),
  commit_sha TEXT NOT NULL,
  file_count INTEGER,
  source_repo TEXT,
  action_run_url TEXT,
  success INTEGER DEFAULT 1,
  error_message TEXT
);

CREATE INDEX IF NOT EXISTS idx_sync_log_worker ON sync_log(worker_id, synced_at DESC);

-- Seed systems
INSERT OR IGNORE INTO system_registry (id, display_name, description, health_rule) VALUES
  ('intel-pipeline', 'Intelligence Pipeline', 'Ingestion + generation sharing 05_transcripts/', 'all'),
  ('tastematter-platform', 'Tastematter Platform', 'Alerting + publishing + control plane', 'all'),
  ('client-deployments', 'Client Deployments', 'Multi-account client workers', 'any'),
  ('internal-tools', 'Internal Tools', 'Workstream reports, personal utilities', 'any'),
  ('pixee-intel', 'Pixee Intelligence', 'LinkedIn monitoring + newsletter intel on Pixee account', 'all');

-- Assign existing workers to systems
UPDATE worker_registry SET system_id = 'intel-pipeline', account_id = '4c8353a21e0bfc69a1e036e223cba4d8' WHERE id = 'transcript-processing';
UPDATE worker_registry SET system_id = 'intel-pipeline', account_id = '4c8353a21e0bfc69a1e036e223cba4d8' WHERE id = 'intelligence-pipeline';
UPDATE worker_registry SET system_id = 'tastematter-platform', account_id = '4c8353a21e0bfc69a1e036e223cba4d8' WHERE id = 'tastematter-alert-worker';
