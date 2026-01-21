# Python CLI Port Inventory

**Total:** 11,133 lines across 34 Python files
**Date:** 2026-01-17
**Purpose:** Enumerate all Python code that needs porting to Rust

---

## Summary by Layer

| Layer | Files | Lines | Port Priority |
|-------|-------|-------|---------------|
| **Capture** | 3 | 1,678 | 🔴 CRITICAL |
| **Index** | 7 | 3,221 | 🔴 CRITICAL |
| **Daemon** | 4 | 988 | 🔴 CRITICAL |
| **Database** | 1 | 75 | ✅ EXISTS in Rust |
| **CLI** | 1 | 2,238 | 🟡 PARTIAL (commands) |
| **Query Engine** | 1 | 960 | ✅ EXISTS in Rust |
| **Intelligence** | 2 | 320 | ⚪ DEFER |
| **Observability** | 4 | 475 | ⚪ DEFER |
| **Visibility** | 2 | 806 | ⚪ DEFER |
| **Other** | 9 | 372 | ⚪ DEFER |

**Must Port:** ~5,887 lines (Capture + Index + Daemon)
**Already Ported:** ~1,035 lines (Database + Query Engine)
**Defer:** ~4,211 lines (Intelligence, Observability, Visibility, CLI UI)

---

## Layer 1: CAPTURE (Must Port)

### 1.1 jsonl_parser.py (627 lines) - CRITICAL

**Purpose:** Parse Claude Code JSONL session files → extract tool uses, sessions

**Classes:**
| Line | Name | Fields |
|------|------|--------|
| 25 | `ToolUse` | id, name, input, timestamp, file_path, is_read, is_write |
| 39 | `ParsedMessage` | type, timestamp, tool_uses |
| 49 | `SessionSummary` | session_id, project_path, started_at, ended_at, duration, counts, files, tools, patterns |
| 79 | `ParseOptions` | TypedDict: project_path, incremental |
| 85 | `ParseResult` | TypedDict: sessions_parsed, tool_uses_extracted, skipped |

**Functions:**
| Line | Name | Signature | Purpose |
|------|------|-----------|---------|
| 97 | `encode_project_path` | `(path: str) -> str` | Windows path → Claude format |
| 128 | `decode_project_path` | `(encoded: str) -> str` | Claude format → Windows path |
| 156 | `get_claude_projects_dir` | `() -> Path` | `~/.claude/projects/` |
| 169 | `find_session_files` | `(project_path, claude_dir?) -> List[Path]` | Glob `**/*.jsonl` |
| 199 | `extract_session_id` | `(filepath: Path) -> str` | Filename → session ID |
| 215 | `extract_tool_uses` | `(content, timestamp) -> List[ToolUse]` | Parse tool_use blocks |
| 254 | `extract_file_path` | `(tool_name, input) -> Optional[str]` | Extract file from tool input |
| 292 | `parse_jsonl_line` | `(line: str) -> Optional[ParsedMessage]` | Single line parser |
| 394 | `aggregate_session` | `(messages) -> SessionSummary` | Combine messages |
| 479 | `parse_session_file` | `(filepath, project) -> SessionSummary` | Full file parser |
| 506 | `session_needs_update` | `(db, session_id, file_size) -> bool` | Incremental check |
| 536 | `upsert_session` | `(db, summary) -> None` | INSERT OR REPLACE |
| 574 | `sync_sessions` | `(db, options) -> ParseResult` | **MAIN ENTRY** |

**Dependencies:** json, datetime, pathlib, sqlite3, logging

---

### 1.2 git_sync.py (483 lines) - CRITICAL

**Purpose:** Sync git commit history to database

**Classes:**
| Line | Name | Fields |
|------|------|--------|
| 23 | `GitCommit` | hash, short_hash, timestamp, author_name, author_email, subject, parent_hashes, files (added/modified/deleted), insertions, deletions, is_agent_commit |
| 48 | `SyncOptions` | TypedDict: since, full, repo_path |
| 56 | `SyncResult` | TypedDict: commits_synced, agent_commits, skipped |
| 64 | `ParseError` | Exception |
| 69 | `GitError` | Exception |

**Functions:**
| Line | Name | Signature | Purpose |
|------|------|-----------|---------|
| 74 | `get_commit_body` | `(hash, repo) -> str` | `git log -1 --format=%B` |
| 113 | `detect_agent_commit` | `(message: str) -> bool` | Check for Claude signatures |
| 135 | `parse_commit_block` | `(block: str) -> GitCommit` | Parse § delimited output |
| 253 | `split_commit_blocks` | `(raw: str) -> List[str]` | Split git log output |
| 288 | `get_last_synced_hash` | `(db) -> Optional[str]` | For incremental sync |
| 308 | `commit_exists` | `(db, hash) -> bool` | Dedup check |
| 325 | `insert_commit` | `(db, commit) -> None` | INSERT |
| 363 | `sync_commits` | `(db, options) -> SyncResult` | **MAIN ENTRY** |

**Dependencies:** subprocess (git), json, datetime, pathlib, sqlite3

**Note:** Uses subprocess to call git. Rust port should use `git2` crate.

---

### 1.3 file_watcher.py (568 lines) - CRITICAL

**Purpose:** Continuous file system monitoring using watchdog

**Classes:**
| Line | Name | Fields |
|------|------|--------|
| 28 | `FileEvent` | timestamp, path, event_type, size, extension |
| 120 | `EventFilter` | patterns (40+ ignore patterns) |
| 195 | `EventDebouncer` | window_ms, events dict |
| 466 | `FileWatcher` | observer, handler, debouncer, running |

**Functions:**
| Line | Name | Signature | Purpose |
|------|------|-----------|---------|
| 273 | `create_event_from_path` | `(path, event_type) -> FileEvent` | Factory |
| 328 | `insert_event` | `(db, event) -> None` | Single INSERT |
| 351 | `insert_events` | `(db, events) -> int` | Batch INSERT |
| 539 | `start_watcher` | `(path, db) -> FileWatcher` | **START** |
| 559 | `stop_watcher` | `(watcher) -> dict` | **STOP** |

**Dependencies:** watchdog, threading, fnmatch, datetime

**Note:** Uses `watchdog` library. Rust port should use `notify` crate.

---

## Layer 2: INDEX (Must Port)

### 2.1 chain_graph.py (627 lines) - CRITICAL

**Purpose:** Build DAG of session continuations/chains

**Classes:**
| Line | Name | Fields |
|------|------|--------|
| 38 | `ChainNode` | session_id, parent_session_id, parent_message_uuid, children, is_agent |
| 47 | `Chain` | chain_id, root_session, sessions, branches, files_list, files_bloom |

**Functions:**
| Line | Name | Signature | Purpose |
|------|------|-----------|---------|
| 63 | `extract_leaf_uuids` | `(filepath) -> List[str]` | LAST summary's leafUuid |
| 115 | `extract_agent_parent` | `(filepath) -> Optional[str]` | sessionId for agents |
| 152 | `extract_message_uuids` | `(filepath) -> List[str]` | All message UUIDs |
| 194 | `build_chain_graph` | `(jsonl_dir) -> Dict[str, Chain]` | **5-PASS ALGORITHM** |
| 355 | `get_session_chain` | `(chains, session_id) -> Optional[str]` | Lookup |
| 371 | `get_session_parent` | `(chains, session_id) -> Optional[str]` | Lookup |
| 388 | `get_chain_depth` | `(chain, session_id) -> int` | Compute depth |
| 422 | `persist_chains` | `(db, chains) -> Dict[str, int]` | Write to DB |
| 498 | `load_chains` | `(db) -> Dict[str, Chain]` | Read from DB |
| 571 | `get_chain_for_session` | `(db, session_id) -> Optional[str]` | DB lookup |
| 589 | `get_session_context` | `(db, session_id) -> Optional[Dict]` | Full context |

**Critical Algorithm (5-pass):**
1. Extract `leafUuid` from LAST summary (not first!)
2. Extract `sessionId` from agent sessions
3. Extract `message.uuid` ownership
4. Build parent-child relationships
5. Group into chains via BFS (connected components)

---

### 2.2 inverted_index.py (482 lines) - CRITICAL

**Purpose:** Build file → sessions mapping

**Classes:**
| Line | Name | Fields |
|------|------|--------|
| 31 | `FileAccess` | session_id, chain_id, file_path, access_type, tool_name, timestamp, count |

**Functions:**
| Line | Name | Signature | Purpose |
|------|------|-----------|---------|
| 64 | `_classify_access_type` | `(tool_name) -> Optional[str]` | read/write/create |
| 82 | `_extract_file_path_from_tool` | `(tool, input) -> Optional[str]` | Extract path |
| 114 | `_extract_tool_use_result_path` | `(record) -> Optional[str]` | Gap 1 fix |
| 143 | `_classify_tool_use_result_access` | `(record) -> str` | Gap 1 type |
| 165 | `_extract_file_history_paths` | `(record) -> List[str]` | Gap 2 fix |
| 187 | `extract_file_accesses` | `(filepath, session_id?) -> List[FileAccess]` | **3 SOURCES** |
| 300 | `build_inverted_index` | `(jsonl_dir, chains?) -> Dict` | Main builder |
| 348 | `get_sessions_for_file` | `(index, path) -> List[FileAccess]` | Lookup |
| 364 | `get_files_for_session` | `(index, session) -> List[FileAccess]` | Lookup |
| 390 | `persist_inverted_index` | `(db, index) -> Dict[str, int]` | Write to DB |
| 436 | `load_inverted_index` | `(db) -> Dict` | Read from DB |

**3 Sources (from Phase 2.5 fix):**
1. `assistant.tool_use` blocks
2. `user.toolUseResult` (Gap 1)
3. `file-history-snapshot` (Gap 2)

---

### 2.3 context_index.py (735 lines) - MEDIUM

**Purpose:** Unified query interface wrapping all indexes

**Classes:**
| Line | Name | Fields |
|------|------|--------|
| 60 | `LoadedChainNode` | session_id, parent_id, children, chain_id, depth |
| 69 | `LoadedChain` | id, root, sessions, agent_count |
| 77 | `ContextIndex` | chains, inverted_index, file_tree, co_access, temporal |

**Key Methods:**
- `get_chain()`, `get_chain_for_session()`, `get_all_chains()`
- `get_sessions_for_file()`, `get_files_for_session()`
- `get_directory_stats()`, `get_hot_directories()`
- `get_week_summary()`, `get_recent_weeks()`
- `persist()`, `load()` class method

**Note:** This is a READ interface. Much of this exists in Rust `query.rs`.

---

### 2.4 bloom.py (184 lines) - LOW

**Purpose:** Bloom filter for O(1) membership checks

**Classes:**
| Line | Name | Methods |
|------|------|---------|
| 20 | `BloomFilter` | add(), __contains__(), serialize(), deserialize() |

**Algorithm:** Double hashing with configurable false positive rate.

**Note:** Pure algorithm, easy to port.

---

### 2.5 temporal.py (339 lines) - MEDIUM

**Purpose:** Group sessions by ISO week

**Classes:**
| Line | Name | Fields |
|------|------|--------|
| 29 | `TemporalBucket` | period, period_type, sessions, chains, files_bloom, commits |

**Functions:**
| Line | Name | Purpose |
|------|------|---------|
| 56 | `_get_iso_week` | `datetime -> "2026-W03"` |
| 99 | `build_temporal_buckets` | Main builder |
| 207 | `file_touched_in_week` | Bloom check |
| 242 | `persist_temporal_buckets` | Write to DB |
| 292 | `load_temporal_buckets` | Read from DB |

---

### 2.6 file_tree.py (430 lines) - MEDIUM

**Purpose:** Directory tree with session/chain aggregation

**Classes:**
| Line | Name | Fields |
|------|------|--------|
| 31 | `FileTreeNode` | path, name, is_dir, chains, sessions, children |

**Functions:**
| Line | Name | Purpose |
|------|------|---------|
| 52 | `normalize_path` | Standardize paths |
| 85 | `_ensure_path_exists` | Create tree path |
| 126 | `bubble_up_stats` | Propagate counts to parents |
| 157 | `build_file_tree` | Main builder |
| 293 | `persist_file_tree` | Write to DB |
| 345 | `load_file_tree` | Read from DB |

---

### 2.7 co_access.py (324 lines) - LOW

**Purpose:** File co-access patterns (Jaccard/PMI similarity)

**Classes:**
| Line | Name | Fields |
|------|------|--------|
| 33 | `CoAccessEntry` | file_a, file_b, jaccard, pmi, shared_sessions |

**Functions:**
| Line | Name | Purpose |
|------|------|---------|
| 73 | `_compute_jaccard` | Set similarity |
| 100 | `_compute_pmi` | Pointwise mutual info |
| 151 | `build_co_access_matrix` | Main builder |
| 250 | `persist_co_access` | Write to DB |
| 294 | `load_co_access` | Read from DB |

**Note:** Can defer - not critical for core functionality.

---

## Layer 3: DAEMON (Must Port)

### 3.1 runner.py (336 lines) - CRITICAL

**Purpose:** Main daemon orchestrator

**Classes:**
| Line | Name | Methods |
|------|------|---------|
| 27 | `ContextOSDaemon` | start(), stop(), run_sync(), _sync_git(), _sync_sessions(), _build_chains() |

**Architecture:**
- Scheduler thread (30 min interval)
- File watcher (if enabled)
- Event handlers (future AI hooks)

**Dependencies:** schedule, threading, time

**Note:** Rust port uses `tokio::time::interval`

---

### 3.2 config.py (223 lines) - MEDIUM

**Purpose:** Configuration management

**TypedDicts:**
| Line | Name | Fields |
|------|------|--------|
| 19 | `SyncConfig` | interval_minutes, git_since_days |
| 26 | `WatchConfig` | enabled, paths, debounce_ms |
| 34 | `ProjectConfig` | path |
| 40 | `IntelligenceConfig` | enabled, auto_commit |
| 48 | `LoggingConfig` | level, max_size_mb |
| 56 | `DaemonConfig` | All above + nested |

**Functions:**
| Line | Name | Purpose |
|------|------|---------|
| 105 | `get_default_config` | Defaults |
| 148 | `load_config` | YAML → DaemonConfig |
| 178 | `validate_config` | Return errors |

---

### 3.3 state.py (79 lines) - LOW

**Purpose:** Daemon state persistence

**Classes:**
| Line | Name | Fields |
|------|------|--------|
| 17 | `DaemonState` | started_at, last_git_sync, last_session_parse, last_chain_build, counters |

---

### 3.4 service.py (350 lines) - DEFER

**Purpose:** Windows service management via NSSM

**Note:** Platform-specific, can defer or simplify.

---

## Layer 4: DATABASE (Exists in Rust)

### 4.1 connection.py (75 lines) - ✅ ALREADY PORTED

**Functions:**
| Line | Name | Rust Equivalent |
|------|------|-----------------|
| 11 | `get_schema_path` | N/A (embedded) |
| 16 | `init_database` | `Database::open_rw()` |
| 50 | `get_connection` | `Database::open()` |
| 73 | `get_database_path` | `Database::canonical_path()` |

---

## Layer 5: CLI (Partial Port)

### 5.1 cli.py (2,238 lines) - PARTIAL

**Commands to Port:**
| Line | Command | Rust CLI | Status |
|------|---------|----------|--------|
| 256 | `init` | `--init` | Need |
| 276 | `sync-git` | `sync-git` | Need |
| 334 | `parse-sessions` | `parse-sessions` | Need |
| 408 | `build-chains` | `build-chains` | Need |
| 497 | `watch` | `watch` | Need |
| 549 | `status` | `status` | Need |
| 1047 | `daemon run` | `daemon` | Need |

**Commands Already in Rust:**
- `query flex/chains/timeline/sessions` (in query.rs)
- `serve` (in http.rs)

**UI/Formatting Functions (Defer):**
- `results_to_json`, `log_command_*`, `query_logged`

---

## Layer 6: QUERY ENGINE (Exists in Rust)

### 6.1 query_engine.py (960 lines) - ✅ ALREADY PORTED

Rust equivalent: `core/src/query.rs`

---

## Layers 7-9: DEFER

### Intelligence (320 lines)
- `jsonl_context.py` (200 lines) - JSONL context extraction
- `queries.py` (120 lines) - Intelligence queries

### Observability (475 lines)
- `state.py` (249 lines) - State tracking
- `event_logger.py` (141 lines) - Event logging
- `events.py` (51 lines) - Event types

### Visibility (806 lines)
- `snapshot.py` (652 lines) - Snapshot generation
- `query_logger.py` (154 lines) - Query logging

---

## Port Order (Recommended)

### Phase 3: Git Sync (8-12 hrs)
Port: `git_sync.py` (483 lines)
- 5 classes, 8 functions
- Replace subprocess with git2 crate

### Phase 4: JSONL Parser (12-16 hrs)
Port: `jsonl_parser.py` (627 lines)
- 5 classes, 12 functions
- Critical: 3-source extraction, path encoding

### Phase 5: Chain Graph (8-12 hrs)
Port: `chain_graph.py` (627 lines)
- 2 classes, 11 functions
- Critical: 5-pass algorithm, LAST leafUuid

### Phase 6: Inverted Index (4-6 hrs)
Port: `inverted_index.py` (482 lines)
- 1 class, 10 functions
- Shares logic with JSONL parser

### Phase 7: File Watcher (6-8 hrs)
Port: `file_watcher.py` (568 lines)
- 4 classes, 5 functions
- Replace watchdog with notify crate

### Phase 8: Daemon Runner (6-8 hrs)
Port: `runner.py` + `config.py` + `state.py` (638 lines)
- 3 classes, orchestration logic
- Replace schedule with tokio interval

---

## Total Port Scope

| Category | Lines | Classes | Functions |
|----------|-------|---------|-----------|
| **Must Port** | 3,405 | 20 | 57 |
| **Medium Priority** | 1,528 | 5 | 25 |
| **Low Priority/Defer** | 508 | 3 | 12 |
| **Already Ported** | 1,035 | - | - |
| **UI/Defer** | 4,657 | - | - |

**Estimated Effort:** 44-62 hours for Must Port + Medium Priority
