-- Migration 005: Add co_access table
-- Phase 4: Co-Access Matrix (Game Trails)
-- Purpose: Track files frequently accessed together for "You need Y" suggestions

CREATE TABLE IF NOT EXISTS co_access (
    file_a TEXT NOT NULL,
    file_b TEXT NOT NULL,
    jaccard_score REAL NOT NULL,
    co_occurrence_count INTEGER,     -- Sessions that touched both
    total_sessions INTEGER,          -- Sessions that touched either
    PRIMARY KEY (file_a, file_b)
);

-- Index for fast lookup by file
CREATE INDEX IF NOT EXISTS idx_co_access_file_a ON co_access(file_a);

-- Index for filtering by score
CREATE INDEX IF NOT EXISTS idx_co_access_score ON co_access(jaccard_score);

-- View: Get game trails (files with high co-access)
CREATE VIEW IF NOT EXISTS game_trails AS
SELECT
    file_a,
    file_b,
    jaccard_score,
    co_occurrence_count,
    total_sessions
FROM co_access
WHERE jaccard_score >= 0.5
ORDER BY jaccard_score DESC;
