-- Context OS Events Database Schema
-- Version: 1.0
-- Created: 2025-12-12

-- ============================================================================
-- Layer 1: FILE EVENTS
-- ============================================================================
-- Captures all file system events including reads (which git doesn't track)
-- Source: watchdog file system watcher

CREATE TABLE IF NOT EXISTS file_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL,              -- ISO8601
    path TEXT NOT NULL,                   -- Relative to repo root
    event_type TEXT NOT NULL,             -- 'read', 'write', 'delete', 'rename', 'create'
    size_bytes INTEGER,                   -- File size after event (NULL for delete)

    -- For rename events
    old_path TEXT,                        -- Previous path if rename

    -- Metadata
    is_directory BOOLEAN DEFAULT FALSE,
    extension TEXT,                       -- File extension for filtering

    -- Indexing
    created_at TEXT DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_file_events_path ON file_events(path);
CREATE INDEX IF NOT EXISTS idx_file_events_timestamp ON file_events(timestamp);
CREATE INDEX IF NOT EXISTS idx_file_events_type ON file_events(event_type);

-- ============================================================================
-- Layer 2: CLAUDE SESSIONS
-- ============================================================================
-- Parsed from ~/.claude/projects/{project}/*.jsonl files
-- Contains tool usage, files touched, session patterns

CREATE TABLE IF NOT EXISTS claude_sessions (
    session_id TEXT PRIMARY KEY,          -- From JSONL filename (UUID)
    project_path TEXT,                    -- Which project this session belongs to

    -- Timing
    started_at TEXT,                      -- First message timestamp
    ended_at TEXT,                        -- Last message timestamp
    duration_seconds INTEGER,             -- Calculated duration

    -- Message counts
    user_message_count INTEGER,
    assistant_message_count INTEGER,
    total_messages INTEGER,

    -- File interactions (JSON arrays)
    files_read TEXT,                      -- ["path1", "path2", ...]
    files_written TEXT,                   -- ["path1", "path2", ...]
    files_created TEXT,                   -- New files created

    -- Tool usage (JSON object with counts)
    tools_used TEXT,                      -- {"Read": 15, "Edit": 8, "Grep": 5, ...}

    -- Grep/search patterns (automation candidates)
    grep_patterns TEXT,                   -- JSON array of patterns searched

    -- Size metrics
    file_size_bytes INTEGER,              -- Size of JSONL file

    -- Metadata
    parsed_at TEXT DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_claude_sessions_started ON claude_sessions(started_at);
CREATE INDEX IF NOT EXISTS idx_claude_sessions_project ON claude_sessions(project_path);

-- ============================================================================
-- Layer 3: GIT COMMITS
-- ============================================================================
-- Synced from git log
-- Represents the canonical state transitions

CREATE TABLE IF NOT EXISTS git_commits (
    hash TEXT PRIMARY KEY,                -- Full commit hash
    short_hash TEXT,                      -- First 7 characters

    -- Commit metadata
    timestamp TEXT NOT NULL,              -- Author date (ISO8601)
    message TEXT,                         -- Commit message
    author_name TEXT,
    author_email TEXT,

    -- Files changed (JSON array)
    files_changed TEXT,                   -- ["path1", "path2", ...]
    files_added TEXT,                     -- New files
    files_deleted TEXT,                   -- Removed files
    files_modified TEXT,                  -- Changed files

    -- Stats
    insertions INTEGER,
    deletions INTEGER,
    files_count INTEGER,

    -- Classification
    is_agent_commit BOOLEAN,              -- Contains "Generated with Claude Code"
    is_merge_commit BOOLEAN,

    -- Metadata
    synced_at TEXT DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_git_commits_timestamp ON git_commits(timestamp);
CREATE INDEX IF NOT EXISTS idx_git_commits_agent ON git_commits(is_agent_commit);

-- ============================================================================
-- ANALYSIS VIEWS
-- ============================================================================

-- Game trails: Most accessed files (reads from file_events + claude_sessions)
CREATE VIEW IF NOT EXISTS game_trails AS
SELECT
    path,
    SUM(CASE WHEN source = 'file_event' THEN 1 ELSE 0 END) as file_event_reads,
    SUM(CASE WHEN source = 'claude_session' THEN 1 ELSE 0 END) as claude_reads,
    COUNT(*) as total_accesses
FROM (
    SELECT path, 'file_event' as source
    FROM file_events
    WHERE event_type = 'read'
    UNION ALL
    SELECT json_each.value as path, 'claude_session' as source
    FROM claude_sessions, json_each(files_read)
)
GROUP BY path
ORDER BY total_accesses DESC;

-- Modification hotspots: Files changed most often
CREATE VIEW IF NOT EXISTS modification_hotspots AS
SELECT
    path,
    SUM(CASE WHEN source = 'file_event' THEN 1 ELSE 0 END) as local_writes,
    SUM(CASE WHEN source = 'git_commit' THEN 1 ELSE 0 END) as committed_changes,
    COUNT(*) as total_modifications
FROM (
    SELECT path, 'file_event' as source
    FROM file_events
    WHERE event_type IN ('write', 'create')
    UNION ALL
    SELECT json_each.value as path, 'git_commit' as source
    FROM git_commits, json_each(files_changed)
)
GROUP BY path
ORDER BY total_modifications DESC;

-- Tool usage patterns (automation candidates)
CREATE VIEW IF NOT EXISTS tool_patterns AS
SELECT
    json_each.key as tool,
    SUM(json_each.value) as total_uses,
    COUNT(DISTINCT session_id) as sessions_used_in
FROM claude_sessions, json_each(tools_used)
GROUP BY json_each.key
ORDER BY total_uses DESC;

-- Commit velocity by directory
CREATE VIEW IF NOT EXISTS commit_velocity_by_area AS
SELECT
    CASE
        WHEN instr(json_each.value, '/') > 0
        THEN substr(json_each.value, 1, instr(json_each.value, '/') - 1)
        ELSE json_each.value
    END as area,
    COUNT(*) as file_changes,
    COUNT(DISTINCT hash) as commits
FROM git_commits, json_each(files_changed)
GROUP BY area
ORDER BY file_changes DESC;

-- ============================================================================
-- METADATA TABLE
-- ============================================================================
-- Tracks schema version and sync state

CREATE TABLE IF NOT EXISTS _metadata (
    key TEXT PRIMARY KEY,
    value TEXT,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP
);

-- Insert schema version
INSERT OR REPLACE INTO _metadata (key, value) VALUES ('schema_version', '2.0');

-- ============================================================================
-- Layer 4: CONVERSATION INTELLIGENCE
-- ============================================================================
-- Extracted contextual meaning from conversations
-- This is where raw analytics become actionable intelligence

CREATE TABLE IF NOT EXISTS conversation_intelligence (
    session_id TEXT PRIMARY KEY,          -- Links to claude_sessions

    -- Work Context (WHAT was being worked on)
    work_theme TEXT,                      -- Primary theme: "Context OS Events - User Visibility Layer"
    work_summary TEXT,                    -- 1-2 sentence summary of actual work done

    -- User Intent (WHY - extracted from voice memos, first messages)
    user_intent TEXT,                     -- Extracted goal: "combine Pixee pattern with Cloudflare workers"
    voice_memo_content TEXT,              -- Raw voice memo transcripts (JSON array)

    -- Decisions Made (KEY CHOICES)
    decisions TEXT,                       -- JSON array of decisions: ["Shifted from custom script to CLI"]

    -- Status Classification
    work_status TEXT,                     -- 'in_progress', 'complete', 'paused', 'abandoned'
    completion_signals TEXT,              -- JSON array of signals: ["committed", "tests passing"]

    -- Keywords (for search/filtering)
    keywords TEXT,                        -- JSON array: ["context-engineering", "visibility", "CLI"]

    -- Chain Detection
    continues_from TEXT,                  -- session_id of previous session in chain (NULL if start)
    continuation_confidence REAL,         -- 0.0-1.0 confidence this continues previous session
    continuation_signal TEXT,             -- What indicated continuation: "explicit", "temporal", "thematic"

    -- Slash Commands Used (high-signal context)
    slash_commands TEXT,                  -- JSON array: ["/chief-of-staff", "/map-work"]

    -- Extraction metadata
    extracted_at TEXT DEFAULT CURRENT_TIMESTAMP,
    extraction_version TEXT DEFAULT '1.0' -- For re-extraction when algorithm improves
);

CREATE INDEX IF NOT EXISTS idx_conv_intel_theme ON conversation_intelligence(work_theme);
CREATE INDEX IF NOT EXISTS idx_conv_intel_status ON conversation_intelligence(work_status);
CREATE INDEX IF NOT EXISTS idx_conv_intel_continues ON conversation_intelligence(continues_from);

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
    ended_at TEXT,                        -- Last session end time
    total_duration_seconds INTEGER,       -- Sum of all session durations

    -- Sessions in chain (JSON array of session_ids, ordered)
    session_ids TEXT,                     -- ["session1", "session2", "session3"]
    session_count INTEGER,

    -- Aggregated stats
    total_messages INTEGER,
    total_tool_uses INTEGER,

    -- Status
    chain_status TEXT,                    -- 'in_progress', 'complete', 'paused'

    -- Key deliverables (extracted from sessions)
    deliverables TEXT,                    -- JSON array: ["apps/context_os_events/", "CLI status command"]

    -- Metadata
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_work_chains_theme ON work_chains(theme);
CREATE INDEX IF NOT EXISTS idx_work_chains_status ON work_chains(chain_status);

-- ============================================================================
-- CONTEXTUAL INTELLIGENCE VIEWS
-- ============================================================================

-- Recent work streams (replaces raw analytics with contextual view)
CREATE VIEW IF NOT EXISTS recent_work AS
SELECT
    ci.work_theme,
    ci.work_summary,
    ci.work_status,
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
    GROUP_CONCAT(ci.work_summary, ' → ') as narrative
FROM work_chains wc
LEFT JOIN conversation_intelligence ci
    ON ci.session_id IN (SELECT value FROM json_each(wc.session_ids))
GROUP BY wc.chain_id
ORDER BY wc.started_at DESC;
