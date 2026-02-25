-- ============================================================================
-- Base Schema: Execution Logging (from cf-worker-scaffold)
-- ============================================================================

CREATE TABLE IF NOT EXISTS flow_logs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  flow_name TEXT NOT NULL,
  execution_id TEXT NOT NULL,
  timestamp TEXT DEFAULT (datetime('now')),
  duration_ms INTEGER,
  event_type TEXT NOT NULL,
  level TEXT DEFAULT 'info',
  message TEXT,
  details TEXT,
  input_id TEXT,
  output_path TEXT,
  error_message TEXT,
  error_stack TEXT,
  CONSTRAINT valid_level CHECK (level IN ('debug', 'info', 'warn', 'error')),
  CONSTRAINT valid_event CHECK (event_type IN ('started', 'step', 'completed', 'failed'))
);

CREATE INDEX IF NOT EXISTS idx_flow_logs_flow_timestamp
  ON flow_logs(flow_name, timestamp DESC);

CREATE INDEX IF NOT EXISTS idx_flow_logs_execution
  ON flow_logs(execution_id);

CREATE TABLE IF NOT EXISTS flow_health (
  flow_name TEXT PRIMARY KEY,
  last_execution_id TEXT,
  last_run_at TEXT,
  last_status TEXT,
  last_duration_ms INTEGER,
  last_error TEXT,
  total_executions INTEGER DEFAULT 0,
  total_failures INTEGER DEFAULT 0,
  avg_duration_ms INTEGER,
  updated_at TEXT DEFAULT (datetime('now'))
);
