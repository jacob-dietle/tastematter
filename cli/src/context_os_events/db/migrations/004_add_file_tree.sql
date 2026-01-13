-- Migration 004: Add file_tree table
-- Phase 3: File Tree Index (Annotated Trie)
-- Purpose: Hierarchical navigation with "fog of war" - session/chain stats at each node

CREATE TABLE IF NOT EXISTS file_tree (
    path TEXT PRIMARY KEY,           -- Relative path (forward slashes)
    name TEXT NOT NULL,              -- Just the filename/dirname
    is_directory BOOLEAN NOT NULL,   -- True for directories, False for files
    parent_path TEXT,                -- Parent directory path (NULL for root)
    chains_json TEXT,                -- JSON array of chain_ids that touched this or children
    sessions_json TEXT,              -- JSON array of session_ids that touched this or children
    session_count INTEGER,           -- Cached len(sessions)
    last_accessed TEXT,              -- ISO timestamp of most recent access
    depth INTEGER                    -- 0 = root, 1 = top-level, etc.
);

-- Index for hierarchical queries
CREATE INDEX IF NOT EXISTS idx_file_tree_parent ON file_tree(parent_path);

-- Index for depth-based queries (e.g., get all top-level dirs)
CREATE INDEX IF NOT EXISTS idx_file_tree_depth ON file_tree(depth);

-- Index for recency queries
CREATE INDEX IF NOT EXISTS idx_file_tree_last_accessed ON file_tree(last_accessed);

-- View: Get hot directories (most sessions)
CREATE VIEW IF NOT EXISTS hot_directories AS
SELECT
    path,
    name,
    session_count,
    last_accessed,
    depth
FROM file_tree
WHERE is_directory = 1
ORDER BY session_count DESC;

-- View: Get recent files
CREATE VIEW IF NOT EXISTS recent_files AS
SELECT
    path,
    name,
    session_count,
    last_accessed
FROM file_tree
WHERE is_directory = 0
  AND last_accessed IS NOT NULL
ORDER BY last_accessed DESC;
