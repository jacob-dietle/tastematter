# Runtime Services Audit Report

**Date:** 2026-02-06
**Auditor:** runtime-services agent (Task #2)
**Scope:** Daemon, Intelligence, Telemetry, HTTP modules in `apps/tastematter/core/src/`
**Cross-check addendum (Task #4):** 2026-02-06 -- verified against `_system/temp/audit_data_pipeline.md`

---

## 1. Daemon Module (`core/src/daemon/`)

### 1.1 `daemon/mod.rs` (Lines 1-211)

**Purpose:** Module root. Re-exports all daemon submodules. Contains CLI integration tests (assert_cmd) and integration tests validating Config -> State -> Sync -> Update lifecycle.

**Key exports:**
- `load_config`, `validate_config`, `DaemonConfig` (from config.rs)
- `collect_gitops_signals`, `load_user_rules` (from gitops.rs)
- `get_platform`, `DaemonPlatform`, `InstallConfig` (from platform/)
- `DaemonState` (from state.rs)
- `run_sync`, `SyncResult` (from sync.rs)

**Tests:** 4 CLI tests (help, status, once, start), 4 integration tests (full workflow, config+state, sync phases, accumulation).

**Dead code:** None in this file.

---

### 1.2 `daemon/config.rs` (Lines 1-277)

**Purpose:** YAML-based configuration for daemon behavior. Stored at `~/.context-os/config.yaml`.

**Configuration sections:**
| Section | Fields | Defaults |
|---------|--------|----------|
| `SyncConfig` | `interval_minutes`, `git_since_days` | 30 min, 7 days |
| `WatchConfig` | `enabled`, `paths`, `debounce_ms` | true, ["."], 100ms |
| `ProjectConfig` | `path` | None (current dir) |
| `LoggingConfig` | `level` | "INFO" |

**Key behavior:**
- `load_config()` creates default config file if missing (line 158-172)
- Partial YAML files merge with defaults via serde defaults (line 179)
- `validate_config()` checks: interval > 0, git_since_days > 0, valid log levels, non-empty watch paths (lines 188-216)

**Dead code:** None. All config types are used by sync.rs and main.rs.

---

### 1.3 `daemon/state.rs` (Lines 1-138)

**Purpose:** JSON-based daemon state persistence. Tracks sync progress across restarts.

**State fields:**
| Field | Type | Purpose |
|-------|------|---------|
| `started_at` | Option<DateTime> | Daemon start time |
| `last_git_sync` | Option<DateTime> | Last git sync timestamp |
| `last_session_parse` | Option<DateTime> | Last session parse timestamp |
| `last_chain_build` | Option<DateTime> | Last chain build timestamp |
| `file_events_captured` | i64 | Total file events |
| `git_commits_synced` | i64 | Total git commits |
| `sessions_parsed` | i64 | Total sessions parsed |
| `chains_built` | i64 | Total chains built |

**Key behavior:**
- `load_or_default()` never fails -- returns default on any error (line 69-71)
- Handles empty/corrupted files gracefully (line 56-63)
- Creates parent directories on save (line 33-36)

**Gap:** State is NOT updated by `run_sync()` itself. The integration test in mod.rs (line 138-140) shows state update is manual. In main.rs `daemon start` loop (line 497-520), state is NOT persisted between syncs -- only SyncResult is logged. **State accumulation is tested but not wired into the actual daemon loop.**

---

### 1.4 `daemon/sync.rs` (Lines 1-1189)

**Purpose:** Core sync orchestrator. This is the most critical file in the daemon.

**Sync phases (executed in order):**
1. **Git sync** (line 92-95) -- calls `sync_commits()` from capture module
2. **Session parsing WITH DB persistence** (line 98) -- calls `sync_sessions()`, then upserts each session to DB
3. **Chain building WITH DB persistence** (line 101) -- calls `build_chain_graph()`, then persists chains to DB
4. **Intelligence enrichment** (line 104-106) -- optional, calls IntelClient for chain naming + summarization
5. **Inverted index** (line 109) -- builds file->session mapping (in-memory only, NOT persisted to DB)

**Database lifecycle (lines 56-83):**
- Opens DB at `~/.context-os/context_os_events.db` in RW mode
- Creates directory if missing (fresh install support)
- Calls `ensure_schema()` for idempotent table creation
- Continues without persistence if DB open fails (graceful degradation)

**Incremental sync (lines 150-156):**
- Loads existing session file sizes from DB
- Sessions whose JSONL file size hasn't changed are skipped
- Uses `HashMap<String, i64>` for session_id -> file_size lookup

**Intelligence enrichment (lines 271-417):**
- Health-checks the Intel service before attempting enrichment
- Names chains that don't have cached names (cache-first pattern)
- Summarizes "interesting" chains: multi-session (>=2), long duration (>30 min), or many files (>10)
- Queries session intent data (first_user_message, conversation_excerpt) from DB for naming quality
- Loads workstream keys from `_system/state/workstreams.yaml` for hybrid tagging
- Aggregates conversation excerpts from multiple sessions (up to 10, max 8K chars)
- **Never fails the sync** -- enrichment is optional

**Workstream loading (lines 582-640):**
- Reads `workstreams.yaml` from project root (`_system/state/workstreams.yaml`)
- Extracts top-level keys from `streams:` section
- Uses CWD to find project root -- **will miss workstreams if CWD is wrong**

**Dead code:** None. All functions are called within `run_sync()`.

**Gap identified:** The inverted index (phase 4) is built in memory but NOT persisted to the database. `build_index_phase()` (line 238-245) only sets `result.files_indexed` but doesn't write to any table. This means **inverted index must be rebuilt on every sync** and is not queryable between syncs except through the SQLite tables populated by session parsing.

**Gap identified:** `enrich_chains_phase()` pushes info messages to `result.errors` (line 410-414). This conflates informational messages with actual errors.

---

### 1.5 `daemon/gitops.rs` (Lines 1-216)

**Purpose:** GitOps signal collection for intelligent git decisions. Collects signals from git status, recent sessions, active chains, and user rules.

**Signal sources:**
| Signal | Source | Data |
|--------|--------|------|
| Git status | `query_repo_status()` | uncommitted files, unpushed commits, branch |
| Time context | Computed from timestamps | hours since last commit/push |
| Session context | Passed as parameter | session_id, files_touched, duration, summary |
| Chain context | Passed as parameter | chain_id, workstream_tags, accomplishments |
| User rules | `load_user_rules()` | Natural language rules from YAML config |

**User rules loading (lines 84-131):**
- Reads `~/.context-os/gitops-rules.yaml`
- Simple hand-rolled YAML parser (not using serde_yaml!)
- Handles quoted strings, comments, section boundaries
- Returns empty vec on missing file

**Integration status:** `collect_gitops_signals()` produces a `GitOpsSignals` struct that matches the TypeScript `/api/intel/gitops-decide` endpoint contract. However, **the GitOps decision endpoint is NOT called anywhere in the codebase**. The function collects signals but there is no consumer calling the Intel service for decisions.

**Dead code:** `collect_gitops_signals()` is exported and tested but **not called from sync.rs or main.rs**. The GitOps decision flow is defined in types (types.rs lines 133-216) but not wired up. This is a Level 0 implementation -- signals only, no automated actions.

**Gap:** `load_user_rules()` uses a hand-rolled YAML parser instead of `serde_yaml`. It's fragile and won't handle complex YAML structures. The test at line 197-215 acknowledges it can't fully test because the function hardcodes the path.

---

### 1.6 `daemon/platform/mod.rs` (Lines 1-257)

**Purpose:** Platform-agnostic daemon registration trait and types. Dispatches to Windows/macOS/Linux implementations via conditional compilation (`#[cfg(target_os)]`).

**Trait: `DaemonPlatform`**
| Method | Purpose |
|--------|---------|
| `install(&InstallConfig)` | Register daemon to run on login |
| `uninstall()` | Remove daemon registration |
| `is_installed()` | Check if registered |
| `status()` | Detailed registration status |

**`InstallConfig` defaults:** binary at `~/.local/bin/tastematter[.exe]`, 30 min interval, service name "tastematter".

**`PlatformStatus` fields:** installed, running, last_run, next_run, message, platform_name.

**`get_default_binary_path()`:** Checks `~/.local/bin/` first, falls back to expecting binary in PATH.

**Dead code:** None. All types used by platform implementations and main.rs.

---

### 1.7 `daemon/platform/windows.rs` (Lines 1-385)

**Purpose:** Windows daemon registration using Startup folder (VBS script, no admin required).

**Installation method:**
- Generates a VBS script that runs `tastematter.exe daemon start --interval N` hidden (no console window)
- Places script in `%APPDATA%\Microsoft\Windows\Start Menu\Programs\Startup\`
- Falls back to checking Task Scheduler for legacy installs

**Windows-specific behavior:**
- `install()`: Verifies binary exists, creates VBS script in Startup folder
- `uninstall()`: Removes VBS script AND cleans up legacy Task Scheduler task
- `status()`: Checks Startup folder first, then Task Scheduler. Uses `tasklist` to check if process is running
- `is_installed()`: Checks both methods (Startup + Task Scheduler)

**FUNCTIONAL on Windows:** Yes. The implementation is complete with proper VBS generation, process checking via `tasklist`, and date parsing for schtasks CSV output.

**Dead code:** `build_daemon_command()` (line 68-73) is marked `#[allow(dead_code)]`. It generates a command string but is not used -- the VBS script generates its own command inline.

**Gap:** `parse_csv_line()` (line 235-251) assumes specific CSV column ordering from `schtasks`. Column order varies by Windows version and locale. This is only used for legacy Task Scheduler installs (fallback path).

---

### 1.8 `daemon/platform/macos.rs` (Lines 1-284)

**Purpose:** macOS daemon registration using launchd (launchctl).

**Installation method:**
- Generates plist XML at `~/Library/LaunchAgents/dev.tastematter.daemon.plist`
- Sets `RunAtLoad: true`, `KeepAlive: false`
- Logs to `~/.context-os/daemon.stdout.log` and `daemon.stderr.log`
- Uses `launchctl load/unload` for service management

**Status checking:** Parses `launchctl list` output to find PID and running status.

**Dead code:** None.

---

### 1.9 `daemon/platform/linux.rs` (Lines 1-281)

**Purpose:** Linux daemon registration using systemd user services.

**Installation method:**
- Generates unit file at `~/.config/systemd/user/tastematter.service`
- Uses `Type=simple`, `Restart=on-failure`, `RestartSec=10`
- `WantedBy=default.target` for user-level service
- Runs `systemctl --user daemon-reload`, `enable`, `start` on install

**Status checking:** Uses `systemctl --user is-active` and `show --property=MainPID`.

**Dead code:** None.

---

## 2. Intelligence Module (`core/src/intelligence/`)

### 2.1 `intelligence/mod.rs` (Lines 1-47)

**Purpose:** Module root. Exports `MetadataStore` (cache), `IntelClient` (HTTP client), and all types.

**Architecture:** HTTP client calls a **separate TypeScript intelligence service** at `localhost:3002`. This is NOT an embedded LLM -- it's a sidecar service.

---

### 2.2 `intelligence/client.rs` (Lines 1-327)

**Purpose:** HTTP client for the TypeScript intelligence service.

**Endpoints called:**
| Method | Endpoint | Purpose |
|--------|----------|---------|
| `name_chain()` | `POST /api/intel/name-chain` | Generate human-readable chain names |
| `summarize_chain()` | `POST /api/intel/summarize-chain` | Generate chain summaries with workstream tags |
| `health_check()` | `GET /api/intel/health` | Service availability check |

**Graceful degradation pattern:** ALL client methods return `Ok(None)` on any failure (network, HTTP error, parse error). Never returns `Err` for service failures. This means:
- Intel service down = sync continues without enrichment
- Parse failures logged but swallowed
- Timeout: 10 seconds (line 34)

**Observability:** Every request gets a UUID correlation ID (line 48), logged at start and end with duration_ms.

**Cost implications:**
- Each chain naming call = 1 LLM API call (on the TypeScript side)
- Each chain summary call = 1 LLM API call
- Calls are made per-chain during sync (potentially hundreds of calls)
- **Mitigated by cache-first pattern** -- only uncached chains are sent to the service
- The "interesting chain" filter (>= 2 sessions OR > 30 min OR > 10 files) limits summary calls

**Dead code:** None. All methods are called from sync.rs.

---

### 2.3 `intelligence/cache.rs` (Lines 1-833)

**Purpose:** SQLite-based cache for intelligence responses. Uses the SAME database (`context_os_events.db`) as the main data store.

**Tables created (MIGRATION_SQL, lines 402-464):**
| Table | Purpose | Key Fields |
|-------|---------|------------|
| `chain_metadata` | Chain naming results | chain_id PK, generated_name, category, confidence, model_used |
| `commit_analysis` | Commit risk analysis | commit_hash PK, is_agent_commit, risk_level, review_focus |
| `session_summaries` | Session summaries | session_id PK, summary, key_files, focus_area |
| `insights_cache` | Cached insights | id, insight_type, title, evidence, expires_at |
| `intelligence_costs` | Cost tracking | id, operation, model, cost_usd, timestamp |
| `chain_summaries` | Chain summary results | chain_id PK, summary, accomplishments (JSON), status, workstream_tags (JSON) |

**Cache operations:**
- `cache_chain_name()` / `get_chain_name()` -- INSERT OR REPLACE pattern
- `cache_chain_summary()` / `get_chain_summary()` -- INSERT OR REPLACE pattern
- `get_all_chain_names()` / `get_all_chain_summaries()` -- List all cached entries
- `delete_chain_name()` / `clear_chain_names()` -- Cache management

**Dead code / Unused tables:**
- `commit_analysis` table -- created but **no code writes to or reads from it**
- `session_summaries` table -- created but **no code writes to or reads from it**
- `insights_cache` table -- created but **no code writes to or reads from it**
- `intelligence_costs` table -- created but **no code writes to or reads from it**

These 4 tables are aspirational schema from future features (commit analysis, session summarization, insights generation, cost tracking). They are created on every `MetadataStore::new()` call but never populated.

---

### 2.4 `intelligence/types.rs` (Lines 1-578)

**Purpose:** Type definitions matching the TypeScript intelligence service API contract.

**Type groups:**

**Chain Naming:**
- `ChainCategory` (8 variants): BugFix, Feature, Refactor, Research, Cleanup, Documentation, Testing, Unknown
- `ChainNamingRequest`: chain_id, files_touched, session_count, recent_sessions, + optional enrichment fields (tools_used, first_user_intent, commit_messages, first_user_message, conversation_excerpt)
- `ChainNamingResponse`: chain_id, generated_name, category, confidence, model_used
- `ChainMetadata`: cached version with all fields optional

**Chain Summary:**
- `WorkStatus` (4 variants): InProgress, Complete, Paused, Abandoned
- `WorkstreamTag`: tag + source (Existing/Generated)
- `ChainSummaryRequest`: chain_id, conversation_excerpt, files_touched, session_count, duration_seconds, existing_workstreams
- `ChainSummaryResponse`: chain_id, summary, accomplishments, status, key_files, workstream_tags, model_used

**GitOps Decision:**
- `GitOpsAction` (5 variants): Commit, Push, Notify, Wait, Ask
- `GitOpsUrgency` (3 variants): Low, Medium, High
- `GitOpsSignals`: uncommitted_files, unpushed_commits, branch, timestamps, session context, chain context, user rules
- `GitOpsDecision`: action, reason, urgency, suggested_commit_message, files_to_stage, coherence_assessment

**Dead code:**
- `GitOpsDecision` struct -- defined and tested for deserialization but **never constructed or returned by any code path**. The Intel client has no `gitops_decide()` method.
- `GitOpsAction`, `GitOpsUrgency` -- types exist but no endpoint calls them.
- `UncommittedFile` -- used only by `GitOpsSignals` which is collected but never sent to Intel service.

---

## 3. Telemetry Module (`core/src/telemetry/`)

### 3.1 `telemetry/mod.rs` (Lines 1-227)

**Purpose:** Anonymous telemetry using PostHog. Fire-and-forget, privacy-first.

**Privacy guarantees (documented in module doc):**
- NEVER: file paths, query content, error messages, user identity
- ALWAYS: machine UUID, platform, version, command, duration, success
- WITH CARE: result counts, time range buckets, error codes

**Opt-out:** `TASTEMATTER_NO_TELEMETRY=1` env var or config file
**Debug:** `TASTEMATTER_TELEMETRY_DEBUG=1` env var

**PostHog API key:** `phc_viCzBS9wW3iaNF0jG0j9mR6IApVnTc62jDkfxPNGUIP` (hardcoded, line 23)

**Configuration:** `~/.context-os/telemetry.yaml` with `enabled: bool` and `uuid: String` (auto-generated v4 UUID).

**Async context issue (lines 81-93):**
- `posthog_rs::client()` creates a blocking runtime internally
- If called from within a tokio async context, it would panic
- Code detects async context via `tokio::runtime::Handle::try_current()` and skips PostHog init
- This means **telemetry is disabled during daemon mode** (which runs in tokio) but enabled for CLI commands

**Event capture:**
- `capture()` is fire-and-forget -- never blocks, never panics, never fails user operation
- Adds standard properties: `$lib`, `platform`, `version`
- Typed helpers: `capture_command()`, `capture_sync()`, `capture_error()`, `capture_feature()`

**Dead code:** `capture_sync()`, `capture_error()`, `capture_feature()` -- these typed helpers exist but are **never called from main.rs**. Only `capture_command()` is called.

---

### 3.2 `telemetry/events.rs` (Lines 1-294)

**Purpose:** Typed event definitions for telemetry.

**Event types:**
| Event | Fields | When emitted |
|-------|--------|-------------|
| `CommandExecutedEvent` | command, duration_ms, success, result_count, time_range_bucket | Every CLI command (from main.rs) |
| `SyncCompletedEvent` | sessions_parsed, chains_built, duration_ms | After daemon sync |
| `ErrorOccurredEvent` | error_code, command | On failures |
| `FeatureUsedEvent` | feature, first_use | Feature adoption |

**Error codes:** DbConnection, DbQuery, ParseFailed, GitSync, ConfigLoad, FileWatch, NetworkError, Unknown

**Dead code:**
- `SyncCompletedEvent` -- defined but **never constructed or emitted**. The daemon does NOT call `capture_sync()`.
- `ErrorOccurredEvent` -- defined but **never constructed or emitted**. No error handlers call `capture_error()`.
- `FeatureUsedEvent` -- defined but **never constructed or emitted**. No feature tracking calls `capture_feature()`.

Only `CommandExecutedEvent` is actually used (from main.rs line 586-594 and 1303-1319).

---

## 4. HTTP Module (`core/src/http.rs`) (Lines 1-151)

**Purpose:** HTTP API server for browser-based development. NOT for production -- binds to localhost only.

**Endpoints:**
| Method | Path | Handler | Input Type | Output Type |
|--------|------|---------|------------|-------------|
| GET | `/api/health` | `health_handler` | None | `HealthStatus` |
| POST | `/api/query/flex` | `query_flex_handler` | `QueryFlexInput` | `QueryResult` |
| POST | `/api/query/timeline` | `query_timeline_handler` | `QueryTimelineInput` | `TimelineData` |
| POST | `/api/query/sessions` | `query_sessions_handler` | `QuerySessionsInput` | `SessionQueryResult` |
| POST | `/api/query/chains` | `query_chains_handler` | `QueryChainsInput` | `ChainQueryResult` |

**Architecture:**
- Uses `axum` framework with `tower_http` CORS middleware
- Shared state via `Arc<AppState>` containing `QueryEngine` and `start_time`
- CORS is optional (enabled via `--cors` flag)
- All query handlers delegate to `QueryEngine` methods

**Health check response:** status, version (from Cargo.toml), database status ("connected"), uptime_seconds.

**Error handling:** `CoreError` converts to `(StatusCode::BAD_REQUEST, Json<ApiError>)` -- all query errors return 400.

**Gaps:**
- No `/api/query/search`, `/api/query/file`, `/api/query/co-access`, `/api/query/heat`, `/api/query/verify`, `/api/query/receipts` endpoints -- these are CLI-only
- No rate limiting
- No authentication (localhost-only is the security model)
- The frontend (Tauri) bypasses HTTP and calls QueryEngine directly via Tauri commands, so this server is purely for browser dev mode

**Dead code:** None. All handlers are wired into the router.

---

## 5. Root Files

### 5.1 `main.rs` (Lines 1-1397)

**Purpose:** CLI entry point using clap. Dispatches all commands.

**Command tree:**
```
tastematter
  query
    flex, chains, timeline, sessions, search, file, co-access, heat, verify, receipts
  serve          -- HTTP server
  sync-git       -- Git commit sync
  parse-sessions -- JSONL parsing
  build-chains   -- Chain graph building
  index-files    -- Inverted index
  watch          -- File watcher
  daemon
    once, start, status, install, uninstall
  intel
    health, name-chain
```

**Telemetry integration:** Every command is wrapped with telemetry capture -- start time measured, command name extracted, result count tracked, time range bucketed.

**Daemon commands handled separately (lines 456-597):** Daemon commands manage their own DB lifecycle (create directory, create schema on fresh install). Non-daemon commands require an existing database.

**Key architectural decision:** Daemon commands use `run_sync()` which creates/opens the DB. Non-daemon commands use `Database::open()` / `Database::open_default()` which expects the DB to exist.

**Dead code:** None in main.rs. However, several CLI commands (`parse-sessions`, `build-chains`, `index-files`) duplicate functionality that `daemon once` already provides. These are useful for debugging but could be considered redundant.

---

### 5.2 `lib.rs` (Lines 1-34)

**Purpose:** Library exports. Makes all modules public.

**Modules:** capture, daemon, error, http, index, intelligence, query, storage, telemetry, types.

**Re-exports:** CoreError, CommandError, QueryEngine, Database, telemetry types.

**Dead code:** None.

---

### 5.3 `error.rs` (Lines 1-73)

**Purpose:** Error types for core operations.

**`CoreError` variants:**
| Variant | Source | Used by |
|---------|--------|---------|
| `Database(sqlx::Error)` | DB operations | query.rs, storage.rs |
| `Query { message }` | Query logic errors | query.rs |
| `Config(String)` | Configuration errors | daemon/config.rs, intelligence/cache.rs |
| `Serialization(serde_json::Error)` | JSON serialization | types.rs |
| `IntelServiceUnavailable` | Intel service down | intelligence/client.rs |
| `IntelServiceError(String)` | Intel service errors | intelligence/client.rs |

**`CommandError`:** Tauri-compatible error format with code, message, details.

**Gap:** `IntelServiceUnavailable` and `IntelServiceError` are defined but **never returned** by the intelligence client -- it uses the graceful degradation pattern (returns `Ok(None)` instead). These error variants could be used by a future non-graceful code path.

---

## 6. Cross-Cutting Analysis

### 6.1 Sync Orchestration: How and When Re-indexing Happens

**Single sync (`daemon once`):**
1. Load config from `~/.context-os/config.yaml`
2. Open/create DB at `~/.context-os/context_os_events.db`
3. Run all 5 phases sequentially (git, sessions, chains, intel, index)
4. Print results and exit

**Daemon loop (`daemon start`):**
1. Load config, set interval
2. Loop forever: run_sync(), sleep for remaining interval time
3. **State is NOT persisted between iterations** (only logged to stderr)

**Incremental vs Full:**
- Sessions: Incremental by file size (skip unchanged files)
- Chains: **Full rebuild every time** (DROP + recreate tables in persist_chains)
- Git: Incremental (`since` parameter limits lookback)
- Index: Full rebuild every time (in-memory only)
- Intel: Cache-first (skip already-named/summarized chains)

### 6.2 Windows Support Status

**Verdict: FUNCTIONAL with caveats.**

| Component | Windows Status |
|-----------|---------------|
| Daemon sync | Works -- uses platform-agnostic code |
| Platform registration | Works -- VBS script in Startup folder |
| Process detection | Works -- uses `tasklist` command |
| Binary path | Checks `~/.local/bin/tastematter.exe` |
| DB path | `~/.context-os/context_os_events.db` via `dirs::home_dir()` |
| Config path | `~/.context-os/config.yaml` |
| Git operations | Platform-agnostic (uses git2 or shell) |

**Caveat:** The `build_daemon_command()` function double-escapes backslashes (line 55 in windows.rs: `.replace("\\", "\\\\")`) which is correct for VBS string literals. No path issues observed.

### 6.3 Intelligence Layer: What LLM Calls Are Made

| Operation | When | Data Sent to LLM | Cost |
|-----------|------|-------------------|------|
| Chain naming | Per uncached chain during sync | files_touched, session_count, first_user_message, conversation_excerpt (~8K chars) | 1 API call per chain |
| Chain summary | Per uncached "interesting" chain | conversation_excerpt (aggregated, ~8K), files_touched, session_count, duration, existing workstreams | 1 API call per chain |
| Health check | Before any enrichment | None (GET request) | Free |

**Cost mitigation:**
1. Cache-first: Only uncached chains are enriched
2. "Interesting" filter: Only chains with >= 2 sessions, > 30 min, or > 10 files get summarized
3. Timeout: 10 second HTTP timeout prevents hung connections
4. Graceful degradation: Intel service down = 0 API calls

**Data analyzed:** conversation_excerpt contains user messages from Claude sessions (up to 8K chars aggregated across up to 10 sessions). This is the primary signal for naming/summarizing.

### 6.4 GitOps: Connection to Real Data

**Status: Signals collected, decisions NOT automated.**

`collect_gitops_signals()` is fully implemented and tested -- it queries real git status, computes time-since-commit/push, and accepts session/chain context. However:
- The `GitOpsDecision` type exists but no Intel endpoint is called
- The Intel service has a `/api/intel/gitops-decide` endpoint (implied by types) but no Rust client method calls it
- No CLI command exposes GitOps functionality
- **This is a Level 0 implementation** -- infrastructure for future automation

### 6.5 Telemetry: What Is Tracked and Where

**Events sent to PostHog:**
| Event | Data | When |
|-------|------|------|
| `command_executed` | command name, duration_ms, success, result_count, time_range_bucket, platform, version | Every CLI command |

**NOT sent (despite types existing):**
- `sync_completed` -- daemon syncs are not telemetry-tracked
- `error_occurred` -- errors are not telemetry-tracked
- `feature_used` -- feature adoption is not tracked

**PostHog destination:** Cloud (API key hardcoded). No self-hosted option.

**Critical note:** Telemetry is disabled in async context (daemon mode) due to PostHog client blocking runtime conflict. This means **daemon operations are invisible to telemetry**.

### 6.6 HTTP API: Contract with Frontend

The HTTP server exposes 5 endpoints (health + 4 queries). The Tauri frontend does NOT use this server -- it calls QueryEngine directly via Tauri commands. The HTTP server exists for:
1. Browser-based development (Svelte dev server at different port)
2. External tool integration
3. Future MCP publishing (Phase 5 from CLAUDE.md)

**Missing from HTTP that exists in CLI:**
- search, file, co-access, heat, verify, receipts queries
- All write operations (sync, parse, build, index)
- Daemon management
- Intel operations

### 6.7 Dead Code Summary

| Module | Dead Code | Severity |
|--------|-----------|----------|
| `intelligence/cache.rs` | 4 empty tables (commit_analysis, session_summaries, insights_cache, intelligence_costs) | Low -- aspirational schema |
| `intelligence/types.rs` | GitOpsDecision, GitOpsAction, GitOpsUrgency types | Low -- Level 0 infrastructure |
| `daemon/gitops.rs` | `collect_gitops_signals()` not called from sync/main | Medium -- exported but unused |
| `daemon/platform/windows.rs` | `build_daemon_command()` | Low -- marked allow(dead_code) |
| `telemetry/mod.rs` | `capture_sync()`, `capture_error()`, `capture_feature()` | Medium -- typed helpers never called |
| `telemetry/events.rs` | `SyncCompletedEvent`, `ErrorOccurredEvent`, `FeatureUsedEvent` | Medium -- defined but never emitted |
| `error.rs` | `IntelServiceUnavailable`, `IntelServiceError` variants | Low -- graceful degradation prevents their use |
| `daemon/state.rs` | DaemonState itself -- not wired into daemon loop | High -- tested but not used in production |

---

## 7. Key Findings Summary

### 7.1 What Works Well

1. **Sync orchestration is robust** -- 5 phases with graceful degradation at each step
2. **Intelligence cache-first pattern** -- prevents redundant LLM API calls
3. **Cross-platform daemon registration** -- Windows (VBS), macOS (launchd), Linux (systemd) all implemented
4. **Incremental session sync** -- file size-based change detection avoids re-parsing unchanged sessions
5. **Fresh install handling** -- DB directory creation, schema init, zero-session graceful handling all tested
6. **Privacy-first telemetry** -- no file paths, no content, opt-out supported

### 7.2 Critical Gaps

1. **DaemonState not wired into daemon loop** -- State persistence is implemented and tested but never called from `daemon start`. Daemon restarts lose all accumulated counters.

2. **Chains rebuilt destructively every sync** -- `persist_chains()` drops and recreates tables. No incremental chain updates. This is O(n) on every sync where n is total sessions.

3. **Inverted index not persisted** -- Built in memory, result count logged, then discarded. Must be rebuilt every sync.

4. **Telemetry disabled in daemon mode** -- PostHog blocking client conflicts with tokio. Daemon operations (the most important to track) are invisible.

5. **GitOps signals collected but not consumed** -- Full signal collection infrastructure exists but no decision endpoint is called. Dead feature.

6. **4 intelligence tables empty** -- commit_analysis, session_summaries, insights_cache, intelligence_costs tables created but never populated.

7. **HTTP API missing 6 query types** -- search, file, co-access, heat, verify, receipts are CLI-only.

### 7.3 Alignment with Data Model Spec (07_CLAUDE_CODE_DATA_MODEL_V2.md)

The sync orchestrator correctly uses:
- `leafUuid` for session continuation (via chain_graph module)
- `sessionId` for agent linking (via chain_graph module)
- File size for incremental sync
- `**/*.jsonl` recursive glob for subdirectory agents

The sync orchestrator does NOT use:
- `parentUuid` conversation tree
- `logicalParentUuid` compaction bridges
- `system` records (turn duration, compaction tracking)
- `progress` records (27,483 records ignored)
- `tool-results/` overflow files (1,714 files invisible)
- Token usage from `message.usage`

This matches the findings from `_system/temp/code_audit_report.md` Section 5.2.

---

## 8. Cross-Check: Data Pipeline vs Runtime Services (Task #4)

**Cross-checked against:** `_system/temp/audit_data_pipeline.md`
**Date:** 2026-02-06

This section verifies the data pipeline audit findings against the runtime services code, answering three specific questions.

### 8.1 Does daemon/sync.rs correctly invoke the parser and indexers?

**VERIFIED: YES, with caveats.**

The sync orchestrator (`daemon/sync.rs:run_sync()`) correctly calls all pipeline stages:

| Phase | Function Called | Module | Invocation Line | Correct? |
|-------|---------------|--------|-----------------|----------|
| 1. Git sync | `sync_commits()` | `capture::git_sync` | sync.rs:126 | YES |
| 2. Session parsing | `sync_sessions()` | `capture::jsonl_parser` | sync.rs:158 | YES |
| 3. Chain building | `build_chain_graph()` | `index::chain_graph` | sync.rs:211 | YES |
| 3.5. Intel enrichment | `IntelClient::name_chain()` / `summarize_chain()` | `intelligence::client` | sync.rs:364, 398 | YES |
| 4. Inverted index | `build_inverted_index()` | `index::inverted_index` | sync.rs:243 | YES |

**Cross-check with data-pipeline findings:**

1. **Parser invocation is correct.** sync.rs passes `claude_dir` (the `~/.claude` path) and `ParseOptions` to `sync_sessions()`. The data-pipeline report confirms `sync_sessions()` correctly uses `**/*.jsonl` recursive glob (jsonl_parser.rs line 191). The daemon correctly passes the `~/.claude` path (not `~/.claude/projects`), and the parser internally appends `projects/` -- this is explicitly documented in sync.rs line 85-86 comment.

2. **Chain builder invocation is correct.** sync.rs calls `build_chain_graph(&claude_dir)` at line 211. The data-pipeline report confirms `build_chain_graph()` uses `**/*.jsonl` (chain_graph.rs line 217) and correctly implements leafUuid + sessionId linking.

3. **Inverted index invocation is correct but output is wasted.** sync.rs calls `build_inverted_index(&claude_dir, chains.as_ref())` at line 243. The chains are passed correctly. However, as noted in my runtime services audit (Section 1.4), the result is only used to set `result.files_indexed` and then discarded. The data-pipeline report confirms the inverted index is built in-memory (inverted_index.rs line 347 sets chain_id on FileAccess records).

**Caveats confirmed by both audits:**

- **BUG-07 from data-pipeline: `persist_chains()` is destructive.** CONFIRMED in query.rs lines 1426-1434: `DROP TABLE IF EXISTS chain_graph` followed by `DROP TABLE IF EXISTS chains`. Both audits agree this is a high-severity bug.

- **BUG-02 from data-pipeline: Chain files_list always empty.** CONFIRMED by sync.rs line 213: `result.chains_built = chains.len() as i32` uses the chain count, and query.rs line 1473 writes `chain.files_list.len() as i32` to `files_count`. Since `build_chain_graph()` never populates `files_list` (chain_graph.rs line 405-408 per data-pipeline report), `files_count` in the `chains` table is always 0.

- **Session persistence is correct.** sync.rs lines 163-191 iterate over parsed `SessionSummary` objects and call `engine.upsert_session()` for each. The data-pipeline report's BUG-08 (files_created dropped in SessionSummary->SessionInput conversion at types.rs:520-538) is confirmed -- the daemon's upsert path uses this conversion, so `files_created` data is parsed by jsonl_parser but lost before database insertion.

### 8.2 Does the HTTP server serve the data that query.rs produces?

**VERIFIED: PARTIALLY.**

The HTTP server (`http.rs`) exposes 4 query endpoints that directly delegate to `QueryEngine` methods:

| HTTP Endpoint | QueryEngine Method | Data Source | Aligned? |
|--------------|-------------------|-------------|----------|
| `POST /api/query/flex` | `query_flex()` | `claude_sessions.files_read` via `json_each()` | YES |
| `POST /api/query/timeline` | `query_timeline()` | `claude_sessions.files_read` + `started_at` | YES |
| `POST /api/query/sessions` | `query_sessions()` | `claude_sessions.*` + `chain_graph` | YES |
| `POST /api/query/chains` | `query_chains()` | `chain_graph` + `claude_sessions` + `chain_metadata` | YES |

**The data-pipeline report confirms all 4 queries are aligned with what the parser writes** (data-pipeline Section 1.5, "Query-data alignment analysis" table). The HTTP server correctly serves exactly what query.rs produces -- it is a thin wrapper with no data transformation.

**However, the HTTP server is MISSING 6 query types that query.rs supports:**

| QueryEngine Method | Available via CLI | Available via HTTP | Gap |
|-------------------|-------------------|-------------------|-----|
| `query_search()` | YES | NO | Missing endpoint |
| `query_file()` | YES | NO | Missing endpoint |
| `query_co_access()` | YES | NO | Missing endpoint |
| `query_heat()` | YES | NO | Missing endpoint |
| `query_verify()` | YES | NO | Missing endpoint |
| `query_receipts()` | YES | NO | Missing endpoint |

This means **the Tauri frontend (which bypasses HTTP and calls QueryEngine directly via Tauri commands) has access to all 10 query types, but any browser-based consumer only gets 4**. If Phase 5 (MCP publishing) builds on the HTTP server, it will be missing 60% of the query surface.

**Cross-check with data-pipeline BUG-05 and BUG-06:** The data-pipeline report found that `query_flex` and `query_heat` only query `files_read`, never `files_written`. The HTTP server inherits this limitation transparently since it delegates directly to QueryEngine. Write-only file access patterns are invisible through both CLI and HTTP.

### 8.3 Are there pipeline stages the daemon skips?

**VERIFIED: YES. Three stages are skipped or incomplete.**

| Stage | Status | Evidence |
|-------|--------|---------|
| **DaemonState persistence** | SKIPPED | sync.rs `run_sync()` returns `SyncResult` but never updates `DaemonState`. main.rs `daemon start` loop (lines 497-520) logs results to stderr but never calls `state.save()`. State accumulation is tested (mod.rs integration tests) but not wired into the actual daemon loop. |
| **Inverted index persistence** | SKIPPED | sync.rs line 243-244 builds the index and records `result.files_indexed` count, but the index is a `HashMap` in memory -- no DB write. The data-pipeline report confirms no `file_accesses` table exists in the Rust schema (storage.rs ensure_schema has no such table). The Python schema had this table via `context_index.py` but Rust never ported it. |
| **Telemetry for daemon sync** | SKIPPED | telemetry/mod.rs lines 81-93: PostHog client is not initialized in async context (tokio runtime conflict). Since `daemon start` and `daemon once` both run in tokio, `TelemetryClient.client` is `None`, making `is_enabled()` return `false`. All `capture_*` calls silently no-op. The `SyncCompletedEvent` type exists but is never emitted from any code path. |

**Additional stages present in the pipeline but partially executed:**

| Stage | What Happens | What's Missing |
|-------|-------------|----------------|
| **Intel enrichment** | Chain naming and summarization work correctly when the service is available | GitOps decision endpoint is never called. 4 additional intel tables (commit_analysis, session_summaries, insights_cache, intelligence_costs) are created but never populated. |
| **Chain metadata population** | chain_graph.rs builds chains with correct linking | `files_list` and `time_range` are always empty (data-pipeline BUG-02). This means `chains.files_count` is always 0 in the database. |
| **Session file_size tracking** | Incremental sync uses `get_session_file_sizes()` to skip unchanged files | Works correctly. Both audits confirm this. |

### 8.4 Cross-Audit Concordance Matrix

Both audits independently found these issues. This table confirms agreement:

| Finding | Data Pipeline Report | Runtime Services Report | Agreement |
|---------|---------------------|------------------------|-----------|
| persist_chains() destructive DROP | BUG-07 (High) | Section 1.4 + 6.1 | AGREE |
| files_written never queried | BUG-05, BUG-06, BUG-10, GAP-10 | Not in scope (query.rs is data-pipeline domain) | CONFIRMED by cross-check |
| Chain files_list always empty | BUG-02 | Section 1.4 via chain enrichment | AGREE |
| DaemonState not wired | Not in scope | Section 1.3 (Gap) | NEW from runtime audit |
| Telemetry disabled in daemon | Not in scope | Section 3.1 + 6.5 | NEW from runtime audit |
| Inverted index not persisted | GAP-9 (timing) | Section 1.4 (never written to DB) | COMPLEMENTARY findings |
| GitOps dead code | Not in scope | Section 1.5 + 6.4 | NEW from runtime audit |
| 4 Intel tables empty | Not in scope | Section 2.3 | NEW from runtime audit |
| schema divergence ensure_schema vs persist_chains | BUG-09 (Medium) | Not explicitly called out | CONFIRMED by cross-check at query.rs:1437-1461 |
| files_created dropped in conversion | BUG-08 (Low) | Confirmed via daemon upsert path | AGREE |
| total_messages undercounts | BUG-01 (Medium) | Not in scope | CONFIRMED -- daemon stores this value |

### 8.5 Cross-Check Summary

**The daemon sync pipeline is architecturally sound but has three categories of issues:**

1. **Write path gaps** (data never reaches the database):
   - Inverted index results discarded after each sync
   - DaemonState never persisted in daemon loop
   - `files_created` and `grep_patterns` dropped in type conversion
   - Chain `files_list` and `time_range` never populated

2. **Destructive operations** (data lost during normal operations):
   - `persist_chains()` DROP+recreates tables on every sync
   - Schema divergence between `ensure_schema()` and `persist_chains()` (additive but confusing)

3. **Observability gaps** (cannot measure what matters):
   - Telemetry disabled during daemon mode (the primary execution context)
   - `SyncCompletedEvent`, `ErrorOccurredEvent`, `FeatureUsedEvent` never emitted
   - `files_written` never queryable despite being stored
