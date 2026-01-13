-- Migration 002: Add Chain Graph Tables
-- Purpose: Store leafUuid-based chain graph for session linking
-- Version: 2.1
-- Created: 2025-12-16

-- ============================================================================
-- CHAIN GRAPH TABLES (leafUuid-based)
-- ============================================================================
-- These tables store the explicit chain relationships from Claude Code's
-- leafUuid mechanism - no heuristics, just explicit links.

-- Individual session chain membership
CREATE TABLE IF NOT EXISTS chain_graph (
    session_id TEXT PRIMARY KEY,
    parent_session_id TEXT,               -- Session containing the parent message
    parent_message_uuid TEXT,             -- The leafUuid value that links to parent
    chain_id TEXT NOT NULL,               -- Which chain this session belongs to
    position_in_chain INTEGER,            -- 0 = root, 1+ = depth from root
    is_root BOOLEAN DEFAULT FALSE,        -- True if this is the chain root
    children_count INTEGER DEFAULT 0,     -- Number of direct children

    -- Metadata
    indexed_at TEXT DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY (session_id) REFERENCES claude_sessions(session_id),
    FOREIGN KEY (parent_session_id) REFERENCES claude_sessions(session_id)
);

CREATE INDEX IF NOT EXISTS idx_chain_graph_chain ON chain_graph(chain_id);
CREATE INDEX IF NOT EXISTS idx_chain_graph_parent ON chain_graph(parent_session_id);
CREATE INDEX IF NOT EXISTS idx_chain_graph_root ON chain_graph(is_root);

-- Chain metadata table
CREATE TABLE IF NOT EXISTS chains (
    chain_id TEXT PRIMARY KEY,
    root_session_id TEXT NOT NULL,        -- First session in chain

    -- Chain statistics
    session_count INTEGER DEFAULT 1,
    branch_count INTEGER DEFAULT 0,       -- Number of branches (sessions with >1 child)
    max_depth INTEGER DEFAULT 0,          -- Deepest path in chain

    -- Time range
    started_at TEXT,                      -- Earliest session start
    ended_at TEXT,                        -- Latest session end
    total_duration_seconds INTEGER,       -- Sum of all session durations

    -- File tracking
    files_bloom BLOB,                     -- Serialized bloom filter of all files
    files_json TEXT,                      -- JSON array of file paths (for queries)
    files_count INTEGER DEFAULT 0,

    -- Metadata
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY (root_session_id) REFERENCES claude_sessions(session_id)
);

CREATE INDEX IF NOT EXISTS idx_chains_root ON chains(root_session_id);
CREATE INDEX IF NOT EXISTS idx_chains_started ON chains(started_at);
CREATE INDEX IF NOT EXISTS idx_chains_session_count ON chains(session_count);

-- ============================================================================
-- CHAIN VIEWS
-- ============================================================================

-- View: Sessions with their chain context
CREATE VIEW IF NOT EXISTS sessions_with_chains AS
SELECT
    cs.session_id,
    cs.project_path,
    cs.started_at,
    cs.ended_at,
    cs.duration_seconds,
    cs.total_messages,
    cg.chain_id,
    cg.parent_session_id,
    cg.position_in_chain,
    cg.is_root,
    c.session_count as chain_session_count,
    c.root_session_id as chain_root
FROM claude_sessions cs
LEFT JOIN chain_graph cg ON cs.session_id = cg.session_id
LEFT JOIN chains c ON cg.chain_id = c.chain_id
ORDER BY cs.started_at DESC;

-- View: Chain summary for quick lookups
CREATE VIEW IF NOT EXISTS chain_summary AS
SELECT
    c.chain_id,
    c.root_session_id,
    c.session_count,
    c.branch_count,
    c.started_at,
    c.ended_at,
    c.total_duration_seconds,
    c.files_count,
    -- First user message from root session (chain intent)
    cs.first_user_message as chain_intent
FROM chains c
LEFT JOIN claude_sessions cs ON c.root_session_id = cs.session_id
ORDER BY c.session_count DESC;

-- Update schema version
INSERT OR REPLACE INTO _metadata (key, value, updated_at)
VALUES ('schema_version', '2.1', CURRENT_TIMESTAMP);
