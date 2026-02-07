# Data Pipeline Audit: Tastematter Ingestion & Query Pipeline

**Date:** 2026-02-06
**Auditor:** data-pipeline agent
**Scope:** All Rust core files: capture/, index/, storage.rs, query.rs, types.rs
**Ground Truth:** 07_CLAUDE_CODE_DATA_MODEL_V2.md (7 record types, 13 linking mechanisms)

---

## 1. File-by-File Audit

### 1.1 `core/src/capture/jsonl_parser.rs` (866 lines + 841 test lines)

**What it does:** Parses Claude Code session JSONL files, extracts tool usage, file access patterns, and aggregates into SessionSummary objects.

**Record types handled (vs v2 spec's 7):**

| Record Type | Handled? | How | Lines |
|-------------|----------|-----|-------|
| `assistant` | YES | Source 1: extracts tool_use content blocks from `message.content[]` | 249-298 |
| `user` | PARTIAL | Source 2: extracts `toolUseResult` file paths only | 308-354 |
| `file-history-snapshot` | YES | Source 3: extracts `trackedFileBackups` keys | 364-390 |
| `tool_result` | ACKNOWLEDGED | Accepted in parse_jsonl_line but produces empty tool_uses vec | 488 |
| `summary` | **NOT PARSED** | Completely skipped (returns None at line 491) | - |
| `system` | **NOT PARSED** | Falls through to `_ => return None` | 491 |
| `progress` | **NOT PARSED** | Falls through to `_ => return None` | 491 |
| `queue-operation` | **NOT PARSED** | Falls through to `_ => return None` | 491 |

**Fields extracted (vs v2 spec):**

| Field | Extracted? | Notes |
|-------|-----------|-------|
| `type` | YES | Discriminator at line 449 |
| `timestamp` | YES | Multi-location: root, message.timestamp, snapshot.timestamp (lines 403-425) |
| `message.role` | YES | From message or inferred from type (lines 455-467) |
| `message.content` | YES | As raw Value for user message text (line 470-474) |
| `message.content[].type=="tool_use"` | YES | Source 1 extraction (line 264) |
| `message.content[].name` | YES | Tool name (line 275) |
| `message.content[].id` | YES | Tool use ID (line 269) |
| `message.content[].input` | YES | Raw JSON preserved (line 280) |
| `toolUseResult.filePath` | YES | Direct path (line 318-320) |
| `toolUseResult.file.filePath` | YES | Nested path (line 323-328) |
| `toolUseResult.type` | YES | Maps to read/write classification (line 335-340) |
| `snapshot.trackedFileBackups` keys | YES | File paths only (line 367-377) |
| `message.content` (string) | YES | First user message + conversation excerpt (lines 545-555) |
| **message.usage** | **NOT EXTRACTED** | Token/cost data lost |
| **message.model** | **NOT EXTRACTED** | Model tracking unavailable |
| **requestId** | **NOT EXTRACTED** | Cannot group blocks from same API call |
| **parentUuid** | **NOT EXTRACTED** | Cannot reconstruct conversation order |
| **sessionId** (on records) | **NOT EXTRACTED** | Uses filename stem instead |
| **isSidechain** | **NOT EXTRACTED** | Agent detection by filename only |
| **agentId** | **NOT EXTRACTED** | Agent identity lost |
| **thinking blocks** | **NOT EXTRACTED** | Extended thinking content discarded |
| **text blocks** | **NOT EXTRACTED** | Assistant text responses discarded |
| **stop_reason** | **NOT EXTRACTED** | Turn completion semantics lost |

**Linking mechanisms used (vs v2 spec's 13):**

| Mechanism | Used? | Notes |
|-----------|-------|-------|
| uuid | NO | Not extracted from any record |
| parentUuid | NO | Conversation tree not reconstructed |
| logicalParentUuid | NO | Compaction bridges not followed |
| sessionId | NO (by parser) | Used only by chain_graph.rs |
| leafUuid | NO (by parser) | Used only by chain_graph.rs |
| agentId | NO | Not extracted |
| tool_use_id | NO | Not extracted from tool_result blocks |
| sourceToolAssistantUUID | NO | Not extracted |
| sourceToolUseID | NO | Not extracted |
| requestId | NO | Not extracted |
| messageId | NO | Not extracted |
| toolUseID | NO | Not extracted |
| parentToolUseID | NO | Not extracted |

**Bugs found:**

1. **BUG: `total_messages` counts only parsed messages** (line 637): `messages.len() as i32` counts only messages that passed `parse_jsonl_line`, which returns `None` for `summary`, `system`, `progress`, `queue-operation`. The v2 spec says a session can have 27,483+ progress records alone. The `total_messages` field significantly undercounts actual JSONL lines.

2. **BUG: Session ID from filename can be duplicated** (line 805-813): The code uses `extract_session_id()` which takes the filename stem. Per v2 spec section 4.3, 216 unique session IDs map to 604 files. Multiple files sharing a `sessionId` (agent files reference parent) will have different filename stems but collide in the database if an agent filename happens to match another session (unlikely but not prevented).

3. **POTENTIAL BUG: `Bash` tool classified as neither read nor write** (line 993-996): The `Bash` tool is not in READ_TOOLS or WRITE_TOOLS, so `file_path` extraction never happens for Bash commands. This is intentional (Bash doesn't have structured file_path input), but means file operations done via Bash (e.g., `cat`, `mv`) are invisible to the index.

**Gaps identified:**

- **GAP-1:** 27,483 `progress` records completely ignored. Bash output streaming, agent progress, MCP tool calls, web search results all invisible.
- **GAP-2:** 1,838 `system` records ignored. Turn duration metrics, compaction boundaries, API errors all lost.
- **GAP-3:** Token usage (`message.usage`) not extracted. Cannot compute cost per session or track model switching.
- **GAP-4:** `tool-results/` overflow files (1,714 files, 48 MB) not indexed. Large tool outputs invisible.
- **GAP-5:** `Skill` tool extracted as read but mapped to `.claude/skills/{name}/SKILL.md` (line 220-225) which is a synthetic path, not the actual skill file content.

---

### 1.2 `core/src/index/chain_graph.rs` (464 lines + 708 test lines)

**What it does:** Builds session chains using leafUuid (from summary records) and sessionId (from agent sessions). 5-pass algorithm.

**Record types handled:**

| Record Type | Handled? | How | Lines |
|-------------|----------|-----|-------|
| `summary` | YES | Extracts `leafUuid` from LAST summary (critical) | 99-128 |
| `user` | YES | Extracts `uuid` field for message ownership | 169-202 |
| `assistant` | YES | Extracts `uuid` field for message ownership | 191 |
| `tool_result` | YES | Extracts `uuid` field for message ownership | 191 |
| `file-history-snapshot` | NO | Not relevant to chain building |
| `system` | NO | Not relevant to chain building |
| `progress` | NO | Not relevant to chain building |
| `queue-operation` | NO | Not relevant to chain building |

**Linking mechanisms used:**

| Mechanism | Used? | How | Lines |
|-----------|-------|-----|-------|
| leafUuid | **YES** | Pass 1: Links child session to parent via LAST summary's leafUuid | 99-128, 259-275 |
| sessionId | **YES** | Pass 2: Links agent-* files to parent session | 134-163, 278-293 |
| uuid | **YES** | Pass 3: Builds message UUID ownership map for leafUuid resolution | 169-202, 296-310 |
| parentUuid | NO | Not used for chain building |
| logicalParentUuid | NO | Not used |

**Correctness assessment:**

1. **CORRECT: Recursive glob** (line 217): Uses `**/*.jsonl` pattern, correctly finds subdirectory agents.

2. **CORRECT: Last summary leafUuid** (line 103): Reads ALL summaries at start of file, keeps LAST one. Stops at first non-summary record (line 122). This matches the v2 spec's "stacked oldest-first" behavior.

3. **CORRECT: Agent parent linking** (line 134-163): Only processes files with `agent-` prefix. Extracts `sessionId` from first record. Validates parent exists in session set.

4. **CORRECT: Self-link prevention** (line 320-321, 335): Explicitly checks `child != parent_session` before linking.

5. **CORRECT: BFS chain grouping** (line 362-382): Uses BFS to find connected components. Sessions not in any parent map are roots.

6. **CORRECT: Branching support** (line 324-328): children_map is `HashMap<String, Vec<String>>`, correctly handles multiple children per parent (tree, not chain).

**Does it handle v2 spec's known complexities?**

| Complexity | Handled? | Notes |
|------------|----------|-------|
| Recursive glob for subdirectory agents | YES | `**/*.jsonl` pattern |
| leafUuid dual purpose (cross-file + same-file) | **PARTIAL** | Works for cross-file continuation. Same-file compaction bookmarks (1,361 per spec) would create self-links, but self-link prevention (line 320) correctly ignores these. |
| Tree (not chain) parentUuid | N/A | parentUuid not used for chain building |
| Agent sessionId linking | YES | Correctly implemented |
| Last-summary-not-first rule | YES | Explicitly documented and implemented |

**Bugs found:**

1. **BUG: Chain time_range and files_list always empty** (lines 405-408): The `Chain` struct has `time_range: Option<(DateTime, DateTime)>` and `files_list: Vec<String>`, but `build_chain_graph()` never populates them (set to `None` and `Vec::new()`). The `chains` table's `files_count` will always be 0 when persisted via `persist_chains()` in query.rs (line 1473: `chain.files_list.len() as i32`).

2. **MINOR: agent-* detection only by filename** (line 136-138): Uses `stem.starts_with("agent-")` instead of checking `isSidechain` field. If Anthropic changes the naming convention, this breaks silently. The v2 spec confirms 100% correlation between `agent-*` prefix and `isSidechain: true`, so this is currently correct but fragile.

3. **POTENTIAL BUG: Multiple files with same session ID** (line 250-257): `all_session_ids` is built from filename stems. If two files have the same stem (unlikely but possible across subdirectories), only one would be tracked.

**Gaps identified:**

- **GAP-6:** `logicalParentUuid` not used. Compaction boundaries within a session create chain breaks that could be bridged but aren't.
- **GAP-7:** No validation that `leafUuid` actually resolves to a message UUID that exists in the expected parent. If the parent session was deleted, the reference becomes orphan silently.

---

### 1.3 `core/src/index/inverted_index.rs` (373 lines + 438 test lines)

**What it does:** Builds bidirectional file-path-to-session mapping for "which sessions touched this file?" queries.

**Record types handled:**

| Record Type | Handled? | How | Lines |
|-------------|----------|-----|-------|
| `assistant` | YES | Source 1: tool_use content blocks | 184-211 |
| `user` | YES | Source 2: toolUseResult file paths | 213-230 |
| `file-history-snapshot` | YES | Source 3: trackedFileBackups keys | 232-250 |
| Others | NO | Skipped |

**Tool classification differences from jsonl_parser.rs:**

| Tool | jsonl_parser classification | inverted_index classification |
|------|---------------------------|------------------------------|
| Grep | READ_TOOLS (creates GREP: pseudo-path) | Returns `None` (filtered out) |
| Glob | READ_TOOLS (creates GLOB: pseudo-path) | Returns `None` (filtered out) |
| Skill | READ_TOOLS (creates synthetic path) | Returns `None` (not handled) |
| WebFetch | READ_TOOLS | `classify_access_type` returns "read" but no file path extraction |
| WebSearch | READ_TOOLS | `classify_access_type` returns "read" but no file path extraction |

**BUG: Skill tool not handled in inverted_index** (line 77-84): `classify_access_type("Skill")` returns `None`, and `extract_inverted_file_path` doesn't handle "Skill" tool. So skill invocations are invisible in the inverted index, even though jsonl_parser.rs tracks them.

**BUG: Timestamp parsing inconsistency** (line 286-296): The inverted_index has its own `parse_timestamp` function that only checks `record.get("timestamp")` at root level. It does NOT fall back to `snapshot.timestamp` for file-history-snapshot records. This means all file-history-snapshot timestamps fall back to `Utc::now()` (ingestion time). The jsonl_parser.rs version (lines 403-425) correctly handles this with priority fallthrough to `snapshot.timestamp`.

**Deduplication behavior** (line 259-284): Deduplicates by `(file_path, access_type)` within a session, incrementing `access_count`. This means reading the same file 5 times in one session creates one record with `access_count: 5`.

**Gaps:**

- **GAP-8:** WebFetch and WebSearch are classified as "read" access types but have no file_path to extract, so they never produce FileAccess records. The classification is dead code.
- **GAP-9:** No chain_id enrichment happens during extraction. The `build_inverted_index` function accepts optional chains but only sets chain_id on FileAccess records after extraction (line 347). This is correct architecturally but means FileAccess records queried before chain building completes have no chain context.

---

### 1.4 `core/src/storage.rs` (313 lines + 355 test lines)

**What it does:** SQLite connection management and schema creation.

**Full table/column inventory from `ensure_schema()` (lines 132-223):**

```
TABLE file_events:
  id              INTEGER PRIMARY KEY AUTOINCREMENT
  timestamp       TEXT NOT NULL
  path            TEXT NOT NULL
  event_type      TEXT NOT NULL
  size_bytes      INTEGER
  old_path        TEXT
  is_directory    BOOLEAN DEFAULT FALSE
  extension       TEXT
  created_at      TEXT DEFAULT CURRENT_TIMESTAMP
  INDEX idx_file_events_path ON file_events(path)
  INDEX idx_file_events_timestamp ON file_events(timestamp)

TABLE claude_sessions:
  session_id                TEXT PRIMARY KEY
  project_path              TEXT
  started_at                TEXT
  ended_at                  TEXT
  duration_seconds          INTEGER
  user_message_count        INTEGER
  assistant_message_count   INTEGER
  total_messages            INTEGER
  files_read                TEXT          -- JSON array
  files_written             TEXT          -- JSON array
  tools_used                TEXT          -- JSON object
  file_size_bytes           INTEGER
  first_user_message        TEXT
  conversation_excerpt      TEXT
  parsed_at                 TEXT DEFAULT CURRENT_TIMESTAMP
  INDEX idx_claude_sessions_started ON claude_sessions(started_at)
  INDEX idx_claude_sessions_project ON claude_sessions(project_path)

TABLE git_commits:
  hash              TEXT PRIMARY KEY
  short_hash        TEXT
  timestamp         TEXT NOT NULL
  message           TEXT
  author_name       TEXT
  author_email      TEXT
  files_changed     TEXT          -- JSON array
  files_added       TEXT          -- JSON array
  files_deleted     TEXT          -- JSON array
  files_modified    TEXT          -- JSON array
  insertions        INTEGER
  deletions         INTEGER
  files_count       INTEGER
  is_agent_commit   BOOLEAN
  is_merge_commit   BOOLEAN
  synced_at         TEXT DEFAULT CURRENT_TIMESTAMP
  INDEX idx_git_commits_timestamp ON git_commits(timestamp)

TABLE chains:
  chain_id          TEXT PRIMARY KEY
  root_session_id   TEXT
  session_count     INTEGER
  files_count       INTEGER
  updated_at        TEXT

TABLE chain_graph:
  session_id        TEXT PRIMARY KEY
  chain_id          TEXT NOT NULL
  INDEX idx_chain_graph_chain ON chain_graph(chain_id)

TABLE chain_metadata:
  chain_id          TEXT PRIMARY KEY
  generated_name    TEXT
  summary           TEXT
  key_topics        TEXT
  created_at        TEXT DEFAULT CURRENT_TIMESTAMP
  updated_at        TEXT DEFAULT CURRENT_TIMESTAMP

TABLE _metadata:
  key               TEXT PRIMARY KEY
  value             TEXT
  updated_at        TEXT DEFAULT CURRENT_TIMESTAMP
  -- Initialized with schema_version='2.1'
```

**Schema vs Python schema divergence:**

| Column/Table | Rust | Python | Impact |
|-------------|------|--------|--------|
| `claude_sessions.files_created` | MISSING | Present | Rust loses file creation tracking |
| `claude_sessions.grep_patterns` | MISSING | Present | Rust loses search pattern tracking |
| `chain_graph` extra columns | Only session_id, chain_id | Also: parent_session_id, is_root, position_in_chain, children_count, parent_message_uuid, indexed_at | Rust loses chain structure detail |
| `chain_metadata` | Present | MISSING | Python lacks Intel-generated names |
| `conversation_intelligence` | MISSING | Present (unused) | No impact (Python doesn't use it either) |
| `work_chains` | MISSING | Present (unused) | No impact |

**CRITICAL NOTE: persist_chains() in query.rs creates its OWN schema** (lines 1426-1461): The `persist_chains()` function DROP+recreates `chains` and `chain_graph` with a DIFFERENT schema than `ensure_schema()`:

```
-- persist_chains() creates chain_graph WITH extra columns:
chain_graph:
  session_id          TEXT PRIMARY KEY
  chain_id            TEXT
  parent_session_id   TEXT      -- EXTRA vs ensure_schema
  is_root             BOOLEAN   -- EXTRA vs ensure_schema
  indexed_at          TEXT      -- EXTRA vs ensure_schema
```

This means **the chain_graph table schema depends on whether ensure_schema() or persist_chains() ran last**. If ensure_schema() runs after persist_chains(), it will NOT drop the extra columns (IF NOT EXISTS), so the schema is additive. But the queries in query.rs only reference `session_id` and `chain_id`, so the extra columns are written but never read.

---

### 1.5 `core/src/query.rs` (1601 lines)

**What it does:** SQL query execution against the indexed SQLite database. All queries operate on pre-indexed data, not raw JSONL.

**Query-data alignment analysis:**

| Query Function | Tables/Columns Used | Data Written By | Alignment |
|---------------|--------------------|-----------------|-----------|
| `query_flex` | `claude_sessions.files_read` via `json_each()`, `chain_graph.session_id/chain_id` | `upsert_session` writes files_read as JSON array, `persist_chains` writes chain_graph | **ALIGNED** |
| `query_chains` | `chain_graph.chain_id`, `claude_sessions` (joined), `chain_metadata.generated_name` | `persist_chains`, separate Intel process for chain_metadata | **ALIGNED** |
| `query_timeline` | `claude_sessions.files_read` via `json_each()`, `started_at` for bucketing | `upsert_session` | **ALIGNED** |
| `query_sessions` | `claude_sessions.*`, `chain_graph` | `upsert_session`, `persist_chains` | **ALIGNED** |
| `query_search` | `claude_sessions.files_read` via `json_each()`, LOWER() for case-insensitive | `upsert_session` | **ALIGNED** |
| `query_file` | `claude_sessions.files_read` via `json_each()`, exact/suffix/substring | `upsert_session` | **ALIGNED** |
| `query_co_access` | `claude_sessions.files_read` via `json_each()`, session co-occurrence | `upsert_session` | **ALIGNED** |
| `query_heat` | `claude_sessions.files_read` via `json_each()`, `started_at` for 7d window | `upsert_session` | **ALIGNED** |

**All queries are aligned with what the parser writes.** The queries correctly use `json_each()` to expand the JSON arrays stored in `files_read` and `files_written`.

**BUG: query_flex only queries files_read, not files_written** (line 66-68): The main query uses `json_each(s.files_read)` but never joins `files_written`. Files that were only written (not read) are invisible to flex queries.

**BUG: query_sessions file_count uses files_read only** (lines 433-438): The CASE expression counts files from `json_each(s.files_read)` but ignores `files_written`. Same issue as above.

**BUG: query_heat only considers files_read** (line 875-878): Heat scores are computed only from read access patterns. Write-heavy files (frequently edited but rarely read independently) will have artificially low heat scores.

**BUG: persist_chains() is destructive** (lines 1426-1434): Drops and recreates `chain_graph` and `chains` tables on EVERY sync. This means:
- All chain_metadata foreign key references survive (chain_metadata uses chain_id, which is deterministic from root session MD5)
- But any manual additions to chain_graph are lost
- During the DROP-recreate window, queries return empty chain results

**Gaps:**

- **GAP-10:** `files_written` never queried by any query function. Write-only file access patterns are invisible.
- **GAP-11:** `tools_used` column never queried. Tool usage patterns are stored but not queryable.
- **GAP-12:** `conversation_excerpt` stored but only used by Intel (chain naming), not queryable via CLI.

---

### 1.6 `core/src/types.rs` (1083 lines)

**What it does:** Defines all type contracts for the query API and provides helper functions (heat metrics, time parsing).

**SessionSummary to SessionInput conversion** (lines 520-538):

```rust
impl From<SessionSummary> for SessionInput {
    fn from(s: SessionSummary) -> Self {
        SessionInput {
            session_id: s.session_id,
            project_path: Some(s.project_path),
            started_at: Some(s.started_at.to_rfc3339()),
            ended_at: Some(s.ended_at.to_rfc3339()),
            duration_seconds: Some(s.duration_seconds as i32),
            user_message_count: Some(s.user_message_count),
            assistant_message_count: Some(s.assistant_message_count),
            total_messages: Some(s.total_messages),
            files_read: Some(serde_json::to_string(&s.files_read).unwrap_or_default()),
            files_written: Some(serde_json::to_string(&s.files_written).unwrap_or_default()),
            tools_used: Some(serde_json::to_string(&s.tools_used).unwrap_or_default()),
            first_user_message: s.first_user_message,
            conversation_excerpt: s.conversation_excerpt,
            file_size_bytes: Some(s.file_size_bytes),
        }
    }
}
```

**BUG: `files_created` dropped in conversion** (line 520-538): `SessionSummary` has a `files_created: Vec<String>` field (jsonl_parser.rs line 94), but `SessionInput` has no corresponding field, and the conversion drops it. Even though the Rust schema lacks `files_created`, this data loss happens silently during the parsing stage.

**BUG: `grep_patterns` dropped in conversion** (same): `SessionSummary.grep_patterns` exists but has no target in `SessionInput`.

**Heat metric correctness:**
- `classify_heat` thresholds: >0.7=HOT, >=0.4=WARM, >=0.2=COOL, <0.2=COLD (correct per spec)
- `compute_velocity`: accesses/day, floors days_active to 1 (correct)
- `compute_heat_score`: `(normalized_AV * 0.3) + (RCR * 0.5) + (recency * 0.2)` where AV is capped at 5.0 (correct)
- `compute_recency_bonus`: 1.0 if <24h, 0.5 if <7d, 0.0 otherwise (correct)

---

### 1.7 `core/src/capture/file_watcher.rs` (766 lines)

**What it does:** Captures filesystem events (create, write, delete, rename) with filtering and debouncing. Not directly related to JSONL parsing but feeds the `file_events` table.

**No data model issues.** This module watches the filesystem directly and doesn't parse JSONL.

---

### 1.8 `core/src/capture/git_sync.rs` (656 lines)

**What it does:** Parses git log output into GitCommit structs for database storage.

**Agent commit detection** (lines 90-110): Checks commit messages for Claude Code signatures: `"generated with claude code"`, `"🤖 generated with"`, `"co-authored-by: claude"`. Case-insensitive. Also checks full body via separate `git log -1 --format=%B` call.

**No data model issues.** This module parses git data, not JSONL.

---

### 1.9 `core/src/capture/git_status.rs` (261 lines)

**What it does:** Queries current git repository status for GitOps decisions.

**No data model issues.** Parses `git status --porcelain` output, not JSONL.

---

### 1.10 `core/src/capture/mod.rs` and `core/src/index/mod.rs`

Simple module re-exports. No logic.

---

## 2. Summary of Findings

### 2.1 Record Type Coverage

| Record Type | v2 Spec Count | Parsed by Code? | What's Extracted |
|-------------|--------------|-----------------|------------------|
| `assistant` | 65,853 | YES | tool_use blocks only (name, id, input) |
| `user` | 35,260 | PARTIAL | toolUseResult file paths + first message text |
| `progress` | 27,483 | **NO** | 0% extracted |
| `summary` | 11,241 | Chain only | leafUuid for chain linking |
| `file-history-snapshot` | 8,580 | YES | File path keys only |
| `queue-operation` | 2,158 | **NO** | 0% extracted |
| `system` | 1,838 | **NO** | 0% extracted |

**Total records in corpus: ~152,413. Records parsed: ~118,273 (77.6%). But extraction depth varies significantly.**

### 2.2 Linking Mechanism Coverage

| Mechanism | v2 Spec | Used in Code? | Where |
|-----------|---------|---------------|-------|
| uuid | Identity field | YES (chain_graph) | extract_message_uuids() |
| parentUuid | Conversation tree | **NO** | Not reconstructed anywhere |
| logicalParentUuid | Compaction bridge | **NO** | Not used |
| sessionId | Agent-to-parent | YES (chain_graph) | extract_agent_parent() |
| leafUuid | Continuation/compaction | YES (chain_graph) | extract_last_leaf_uuid() |
| agentId | Agent identity | **NO** | Filename heuristic used instead |
| tool_use_id | Tool result matching | **NO** | Not extracted |
| sourceToolAssistantUUID | Tool provenance | **NO** | Not extracted |
| sourceToolUseID | Tool result matching | **NO** | Not extracted |
| requestId | API response grouping | **NO** | Not extracted |
| messageId | Snapshot trigger | **NO** | Not extracted |
| toolUseID | Progress tracking | **NO** | Not extracted |
| parentToolUseID | Progress-to-tool | **NO** | Not extracted |
| teamName | Team identity | **NO** | Not extracted |

**3 of 14 linking mechanisms used (21%).**

### 2.3 All Bugs Found

| ID | Severity | Location | Description |
|----|----------|----------|-------------|
| BUG-01 | Medium | jsonl_parser.rs:637 | `total_messages` undercounts by excluding summary/system/progress/queue-operation records |
| BUG-02 | Low | chain_graph.rs:405-408 | Chain `time_range` and `files_list` always empty, causing `chains.files_count=0` in DB |
| BUG-03 | Medium | inverted_index.rs:286-296 | Timestamp fallback for file-history-snapshot doesn't check `snapshot.timestamp`, causing all snapshot timestamps to be ingestion time |
| BUG-04 | Low | inverted_index.rs:77-84 | Skill tool returns `None` from `classify_access_type`, invisible in inverted index |
| BUG-05 | Medium | query.rs:66-68 | `query_flex` only queries `files_read`, never `files_written` -- write-only files invisible |
| BUG-06 | Medium | query.rs:875-878 | `query_heat` only considers `files_read` -- write-heavy files have artificially low heat |
| BUG-07 | High | query.rs:1426-1434 | `persist_chains()` DROP+recreates tables destructively on every sync |
| BUG-08 | Low | types.rs:520-538 | `files_created` and `grep_patterns` dropped silently in SessionSummary->SessionInput conversion |
| BUG-09 | Medium | storage.rs + query.rs | `chain_graph` schema diverges between `ensure_schema()` and `persist_chains()` |
| BUG-10 | Medium | query.rs:433-438 | `query_sessions` file_count only considers files_read |

### 2.4 All Gaps Found

| ID | Category | Description | Impact |
|----|----------|-------------|--------|
| GAP-01 | Record types | 27,483 `progress` records ignored | Bash output, agent progress, MCP calls, search results all invisible |
| GAP-02 | Record types | 1,838 `system` records ignored | Turn duration, compaction metrics, API errors lost |
| GAP-03 | Fields | Token usage not extracted | Cannot compute cost per session or track model usage |
| GAP-04 | External data | `tool-results/` overflow files not indexed | 1,714 files (48 MB) of large tool outputs invisible |
| GAP-05 | Tool handling | Skill tool maps to synthetic path | Not real file access |
| GAP-06 | Linking | `logicalParentUuid` not used | Compaction bridges not followed in chain building |
| GAP-07 | Linking | No leafUuid orphan validation | Deleted parent sessions cause silent orphans |
| GAP-08 | Dead code | WebFetch/WebSearch classified but never produce FileAccess | Classification code has no effect |
| GAP-09 | Timing | Chain enrichment happens post-extraction | FileAccess records without chain context during partial builds |
| GAP-10 | Query | `files_written` never queried | Write patterns invisible in all query commands |
| GAP-11 | Query | `tools_used` never queried | Tool usage patterns stored but not accessible |
| GAP-12 | Query | `conversation_excerpt` not queryable via CLI | Only used by Intel chain naming |

### 2.5 chain_graph.rs Correctness Summary

The chain graph builder is **fundamentally correct** for its intended purpose:

| Aspect | Status | Evidence |
|--------|--------|----------|
| Recursive glob for subdirectory agents | CORRECT | `**/*.jsonl` at line 217 |
| leafUuid dual purpose handling | CORRECT | Same-file refs become self-links, correctly prevented at line 320 |
| Tree structure (not chain) via parentUuid | N/A | parentUuid not used, but branches HashMap correctly handles tree topology |
| Agent sessionId linking | CORRECT | filename prefix detection + sessionId extraction from first record |
| Last-summary-not-first rule | CORRECT | Iterates all leading summaries, keeps last leafUuid |
| Self-link prevention | CORRECT | Explicit check at lines 320-321 and 335 |
| Disconnected chain detection | CORRECT | BFS from all roots at line 356-411 |

The main weakness is that chain metadata (time_range, files_list) is never populated during graph building, making the `chains` table's `files_count` column always 0.
