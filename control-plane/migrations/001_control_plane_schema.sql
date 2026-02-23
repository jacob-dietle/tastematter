-- Control Plane Schema: Worker Registry + Health Monitoring

CREATE TABLE IF NOT EXISTS worker_registry (
  id TEXT PRIMARY KEY,
  display_name TEXT NOT NULL,
  health_url TEXT NOT NULL,
  expected_cadence TEXT,
  max_silence_hours INTEGER DEFAULT 24,
  auth_type TEXT DEFAULT 'none',
  tags TEXT,
  enabled INTEGER DEFAULT 1,
  created_at TEXT DEFAULT (datetime('now')),
  updated_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS health_log (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  worker_id TEXT NOT NULL,
  checked_at TEXT DEFAULT (datetime('now')),
  http_status INTEGER,
  response_time_ms INTEGER,
  status TEXT NOT NULL,
  last_activity TEXT,
  activity_type TEXT,
  raw_response TEXT,
  error_message TEXT
);

CREATE INDEX IF NOT EXISTS idx_health_log_worker
  ON health_log(worker_id, checked_at DESC);

-- Seed known workers
INSERT OR IGNORE INTO worker_registry (id, display_name, health_url, expected_cadence, max_silence_hours, auth_type) VALUES
  ('transcript-processing', 'Automated Transcript Processing', 'https://transcript-processing.jacob-4c8.workers.dev/health', 'event', 48, 'none'),
  ('tastematter-alert-worker', 'Tastematter Alert Worker', 'https://api.tastematter.dev/health', '4h', 8, 'cf-access'),
  ('intelligence-pipeline', 'Intelligence Pipeline', 'https://intel.tastematter.dev/health', 'daily', 48, 'cf-access');
