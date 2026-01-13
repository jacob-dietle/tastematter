-- Migration 006: Add temporal_buckets table
-- Phase 5: Temporal Buckets
-- Purpose: Group sessions by time period for "What was I working on last week?" queries

CREATE TABLE IF NOT EXISTS temporal_buckets (
    period TEXT PRIMARY KEY,        -- "2025-W50" or "2025-12-16"
    period_type TEXT NOT NULL,      -- "week" or "day"
    sessions_json TEXT,             -- JSON array of session IDs
    chains_json TEXT,               -- JSON array of chain IDs
    files_bloom BLOB,               -- Serialized bloom filter for fast file lookup
    commits_json TEXT,              -- JSON array of git commit hashes
    started_at TEXT,                -- ISO datetime for period start
    ended_at TEXT,                  -- ISO datetime for period end
    session_count INTEGER,          -- Cached count
    chain_count INTEGER,            -- Cached count
    commit_count INTEGER            -- Cached count
);

-- Index for filtering by period type (week vs day)
CREATE INDEX IF NOT EXISTS idx_temporal_period_type ON temporal_buckets(period_type);

-- Index for date range queries
CREATE INDEX IF NOT EXISTS idx_temporal_started ON temporal_buckets(started_at);

-- View: Get recent weeks with activity stats
CREATE VIEW IF NOT EXISTS recent_activity AS
SELECT
    period,
    session_count,
    chain_count,
    commit_count,
    started_at,
    ended_at
FROM temporal_buckets
WHERE period_type = 'week'
ORDER BY started_at DESC
LIMIT 12;
