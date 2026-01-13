-- Migration 001: Add Intelligence Tables
-- Version: 1.0 → 2.0
-- Purpose: Add Layer 4 (Conversation Intelligence) and Layer 5 (Work Chains)

-- ============================================================================
-- Layer 4: CONVERSATION INTELLIGENCE
-- ============================================================================
-- Extracted contextual meaning from conversations
-- This is where raw analytics become actionable intelligence

CREATE TABLE IF NOT EXISTS conversation_intelligence (
    session_id TEXT PRIMARY KEY,          -- Links to claude_sessions

    -- Work Context (WHAT was being worked on)
    work_theme TEXT,                      -- Primary theme: "Context OS Events - Daemon Implementation"
    work_summary TEXT,                    -- 1-2 sentence summary of actual work done

    -- User Intent (WHY - extracted from first message, voice memos)
    user_intent TEXT,                     -- Extracted goal from first user message
    first_user_message TEXT,              -- Raw first user message (the intent signal)
    voice_memo_paths TEXT,                -- JSON array of voice memo paths mentioned

    -- Decisions Made (KEY CHOICES)
    decisions TEXT,                       -- JSON array of decisions: ["Migrated from Servy to NSSM"]

    -- Status Classification
    work_status TEXT,                     -- 'in_progress', 'complete', 'paused', 'abandoned'
    completion_signals TEXT,              -- JSON array of signals: ["committed", "tests passing"]

    -- Keywords (for search/filtering)
    keywords TEXT,                        -- JSON array: ["context-engineering", "daemon", "TDD"]

    -- Chain Detection
    continues_from TEXT,                  -- session_id of previous session in chain (NULL if start)
    continuation_confidence REAL,         -- 0.0-1.0 confidence this continues previous session
    continuation_signal TEXT,             -- What indicated continuation: "explicit", "temporal", "file_overlap"

    -- File Context
    primary_files TEXT,                   -- JSON array of most important files worked on
    file_overlap_with TEXT,               -- JSON array of session_ids with file overlap

    -- Slash Commands Used (high-signal context)
    slash_commands TEXT,                  -- JSON array: ["/chief-of-staff", "/map-work"]

    -- Extraction metadata
    extracted_at TEXT,
    extraction_model TEXT,                -- 'haiku', 'deterministic', 'manual'
    extraction_version TEXT DEFAULT '1.0',

    FOREIGN KEY (session_id) REFERENCES claude_sessions(session_id)
);

CREATE INDEX IF NOT EXISTS idx_conv_intel_theme ON conversation_intelligence(work_theme);
CREATE INDEX IF NOT EXISTS idx_conv_intel_status ON conversation_intelligence(work_status);
CREATE INDEX IF NOT EXISTS idx_conv_intel_continues ON conversation_intelligence(continues_from);
CREATE INDEX IF NOT EXISTS idx_conv_intel_extracted ON conversation_intelligence(extracted_at);

-- ============================================================================
-- Layer 5: WORK CHAINS
-- ============================================================================
-- Aggregated view of connected work sessions
-- A chain is a sequence of sessions working on the same thing

CREATE TABLE IF NOT EXISTS work_chains (
    chain_id TEXT PRIMARY KEY,            -- Generated ID for chain

    -- Chain metadata
    theme TEXT,                           -- Synthesized theme for entire chain
    started_at TEXT,                      -- First session start time
    ended_at TEXT,                        -- Last session end time (NULL if ongoing)
    total_duration_seconds INTEGER,       -- Sum of all session durations

    -- Sessions in chain (JSON array of session_ids, ordered)
    session_ids TEXT,                     -- ["session1", "session2", "session3"]
    session_count INTEGER,

    -- Aggregated stats
    total_messages INTEGER,
    total_tool_uses INTEGER,
    files_touched TEXT,                   -- JSON array of all files in chain

    -- Status
    chain_status TEXT,                    -- 'in_progress', 'complete', 'paused'

    -- Key deliverables (extracted from sessions)
    deliverables TEXT,                    -- JSON array: ["apps/context_os_events/", "47 tests passing"]

    -- Git integration
    commits_in_chain TEXT,                -- JSON array of commit hashes during chain

    -- Metadata
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_work_chains_theme ON work_chains(theme);
CREATE INDEX IF NOT EXISTS idx_work_chains_status ON work_chains(chain_status);
CREATE INDEX IF NOT EXISTS idx_work_chains_started ON work_chains(started_at);

-- ============================================================================
-- Layer 2 Enhancement: FILE INDEX
-- ============================================================================
-- Bidirectional mapping: file ↔ conversations

CREATE TABLE IF NOT EXISTS file_conversation_index (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_path TEXT NOT NULL,              -- Relative path to file
    session_id TEXT NOT NULL,             -- Conversation that touched this file
    access_type TEXT,                     -- 'read', 'write', 'create', 'mention'
    access_count INTEGER DEFAULT 1,       -- How many times in this session
    first_accessed_at TEXT,               -- When first accessed in session

    UNIQUE(file_path, session_id),
    FOREIGN KEY (session_id) REFERENCES claude_sessions(session_id)
);

CREATE INDEX IF NOT EXISTS idx_file_conv_file ON file_conversation_index(file_path);
CREATE INDEX IF NOT EXISTS idx_file_conv_session ON file_conversation_index(session_id);

-- ============================================================================
-- CONTEXTUAL INTELLIGENCE VIEWS
-- ============================================================================

-- Recent work streams (replaces raw analytics with contextual view)
CREATE VIEW IF NOT EXISTS recent_work AS
SELECT
    ci.work_theme,
    ci.work_summary,
    ci.work_status,
    ci.first_user_message,
    cs.started_at,
    cs.ended_at,
    cs.total_messages,
    ci.user_intent,
    ci.keywords,
    ci.slash_commands,
    cs.session_id
FROM conversation_intelligence ci
JOIN claude_sessions cs ON ci.session_id = cs.session_id
ORDER BY cs.started_at DESC;

-- Work chains with full context
CREATE VIEW IF NOT EXISTS work_chains_detailed AS
SELECT
    wc.chain_id,
    wc.theme,
    wc.chain_status,
    wc.started_at,
    wc.ended_at,
    wc.session_count,
    wc.total_messages,
    wc.deliverables,
    wc.files_touched,
    wc.commits_in_chain
FROM work_chains wc
ORDER BY wc.started_at DESC;

-- Files with conversation context
CREATE VIEW IF NOT EXISTS file_context AS
SELECT
    fci.file_path,
    COUNT(DISTINCT fci.session_id) as conversation_count,
    GROUP_CONCAT(DISTINCT ci.work_theme) as themes,
    MAX(cs.started_at) as last_accessed,
    SUM(fci.access_count) as total_accesses
FROM file_conversation_index fci
LEFT JOIN conversation_intelligence ci ON fci.session_id = ci.session_id
LEFT JOIN claude_sessions cs ON fci.session_id = cs.session_id
GROUP BY fci.file_path
ORDER BY conversation_count DESC;

-- ============================================================================
-- Update metadata
-- ============================================================================
INSERT OR REPLACE INTO _metadata (key, value, updated_at)
VALUES ('schema_version', '2.0', CURRENT_TIMESTAMP);
