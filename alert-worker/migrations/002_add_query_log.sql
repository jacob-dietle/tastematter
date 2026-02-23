-- Query logging for context publishing (Phase 2)
CREATE TABLE IF NOT EXISTS query_log (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  engagement_id TEXT NOT NULL,
  timestamp TEXT NOT NULL DEFAULT (datetime('now')),
  query TEXT NOT NULL,
  response_length INTEGER,
  duration_ms INTEGER,
  tool_calls INTEGER,
  corpus_commit TEXT,
  success INTEGER DEFAULT 1,
  error_message TEXT
);
