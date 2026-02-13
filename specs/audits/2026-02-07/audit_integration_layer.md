# Audit: Rust-Intel Integration Layer & Daemon

**Date:** 2026-02-07
**Scope:** `core/src/intelligence/` and `core/src/daemon/`
**Auditor:** integration-auditor agent

---

## 1. IntelClient Inventory

**File:** `core/src/intelligence/client.rs`

The `IntelClient` struct wraps a `reqwest::Client` with a configurable `base_url` (public field) and a private `http_client`.

### Methods

| Method | Signature | Intel Endpoint | Error Handling |
|--------|-----------|----------------|----------------|
| `new(base_url: &str) -> Self` | Constructor | N/A | `expect()` on HTTP client build (panics on failure -- acceptable for one-time init) |
| `default() -> Self` | Default impl | N/A | Delegates to `new("http://localhost:3002")` |
| `name_chain(&self, request: &ChainNamingRequest) -> Result<Option<ChainNamingResponse>, CoreError>` | Async | `POST /api/intel/name-chain` | Graceful: `Ok(None)` on any failure (network, non-2xx, parse error). Never returns `Err` for service failures. |
| `summarize_chain(&self, request: &ChainSummaryRequest) -> Result<Option<ChainSummaryResponse>, CoreError>` | Async | `POST /api/intel/summarize-chain` | Identical graceful pattern: `Ok(None)` on any failure. |
| `health_check(&self) -> bool` | Async | `GET /api/intel/health` | Returns `false` on any error. |

### Observability

Both `name_chain` and `summarize_chain` use structured logging with:
- Unique `correlation_id` (UUID v4) per request
- `X-Correlation-ID` HTTP header sent to Intel service
- Request start log (operation, chain_id, relevant count)
- Request completion log (correlation_id, duration_ms, success status)
- Warning log on failures (with correlation_id, duration_ms, error)
- Log target: `"intelligence"`

### Notable design choices
- The `Result<Option<T>, CoreError>` return type means `Err` is reserved for truly unexpected errors (none currently produced). In practice, these methods always return `Ok`.
- Both methods share identical error handling structure (success path -> parse -> warn on failure). This is duplicated code but clear.

---

## 2. Cache Layer

**File:** `core/src/intelligence/cache.rs`

### MetadataStore

Backed by SQLite via `sqlx`. The store opens its own connection pool (max 5 connections) to the database file passed at construction time.

### Schema (owned by cache.rs MIGRATION_SQL)

| Table | Primary Key | Purpose |
|-------|-------------|---------|
| `commit_analysis` | `commit_hash TEXT` | Commit analysis cache (agent commit detection, summary, risk level) |
| `session_summaries` | `session_id TEXT` | Session-level summaries |
| `insights_cache` | `id INTEGER AUTOINCREMENT` | Time-limited insights (has `expires_at`) |
| `intelligence_costs` | `id INTEGER AUTOINCREMENT` | Cost tracking per operation/model |

**Important note from code comment (line 402):** `chain_metadata` and `chain_summaries` tables are NOT created by this migration -- they are owned by `storage.rs ensure_schema()`. The test helper `create_test_store()` calls `Database::open_rw()` + `ensure_schema()` before `MetadataStore::new()` to ensure these prerequisite tables exist.

### Cache Operations

| Method | Signature | Behavior |
|--------|-----------|----------|
| `new(db_path: &Path) -> Result<Self, CoreError>` | Opens pool, runs migration | Creates `commit_analysis`, `session_summaries`, `insights_cache`, `intelligence_costs` tables |
| `cache_chain_name(&self, response: &ChainNamingResponse) -> Result<(), CoreError>` | `INSERT OR REPLACE` into `chain_metadata` | Upsert by chain_id. Serializes category to kebab-case string. |
| `get_chain_name(&self, chain_id: &str) -> Result<Option<ChainMetadata>, CoreError>` | `SELECT ... WHERE chain_id = ?` | Returns `None` on cache miss. Logs `debug` on hit/miss. |
| `get_all_chain_names(&self) -> Result<Vec<ChainMetadata>, CoreError>` | `SELECT ... ORDER BY generated_at DESC` | Returns all cached names |
| `delete_chain_name(&self, chain_id: &str) -> Result<bool, CoreError>` | `DELETE WHERE chain_id = ?` | Returns `true` if row was affected |
| `clear_chain_names(&self) -> Result<u64, CoreError>` | `DELETE FROM chain_metadata` | Returns count of rows deleted |
| `cache_chain_summary(&self, response: &ChainSummaryResponse) -> Result<(), CoreError>` | `INSERT OR REPLACE` into `chain_summaries` | Serializes arrays (`accomplishments`, `key_files`, `workstream_tags`) as JSON TEXT |
| `get_chain_summary(&self, chain_id: &str) -> Result<Option<ChainSummaryResponse>, CoreError>` | `SELECT ... WHERE chain_id = ?` | Deserializes JSON fields, defaults to empty Vec/InProgress on parse failure |
| `get_all_chain_summaries(&self) -> Result<Vec<ChainSummaryResponse>, CoreError>` | `SELECT ... ORDER BY created_at DESC` | Returns all cached summaries |

### TTL / Expiry

**There is NO TTL or cache invalidation for chain_metadata or chain_summaries.** Once cached, a chain name or summary persists indefinitely. The only invalidation is:
- Manual deletion via `delete_chain_name()`
- `INSERT OR REPLACE` when re-enrichment occurs (but the `needs_naming` / `needs_summary` check in `sync.rs` means re-enrichment is skipped if cache exists)

The `insights_cache` table has an `expires_at` column, but there's no code that reads or enforces it -- this appears to be forward-declared schema.

### Hit/Miss Logic

In `sync.rs enrich_chains_phase()`:
- **Chain naming:** `cache.get_chain_name(chain_id)` is called. If `Ok(None)` (cache miss), naming is attempted. If `Ok(Some(_))` (cache hit), chain is skipped.
- **Chain summary:** Same pattern via `cache.get_chain_summary(chain_id)`. Only attempted if `should_summarize_chain()` returns `true` AND cache misses.

---

## 3. Daemon Sync Flow

**File:** `core/src/daemon/sync.rs`

### Entry Point: `run_sync(config: &DaemonConfig) -> Result<SyncResult, String>`

Complete flow, start to finish:

```
run_sync(config)
  |
  |-- 0. Setup
  |     - Open DB at ~/.context-os/context_os_events.db (open_rw)
  |     - Run ensure_schema() (idempotent)
  |     - Create QueryEngine
  |     - Resolve ~/.claude directory
  |
  |-- 1. Git Sync  [sync_git()]
  |     - sync_commits() from capture::git_sync
  |     - Result: git_commits_synced count
  |
  |-- 2. Session Parsing  [sync_sessions_phase()]
  |     - Load existing session file sizes from DB (incremental)
  |     - sync_sessions() from capture::jsonl_parser
  |     - For each parsed session: engine.upsert_session() (INSERT OR REPLACE)
  |     - Result: sessions_parsed count
  |
  |-- 3. Chain Building  [build_chains_phase()]
  |     - build_chain_graph() from index::chain_graph
  |     - engine.persist_chains() to DB
  |     - Result: chains_built count, HashMap<String, Chain>
  |
  |-- 3.5. Intelligence Enrichment  [enrich_chains_phase()]  (OPTIONAL)
  |     - IntelClient::default() (localhost:3002)
  |     - Health check -> skip if unavailable
  |     - Open MetadataStore cache
  |     - Open separate SqlitePool for session data queries
  |     - Load workstreams from _system/state/workstreams.yaml
  |     - For each chain:
  |       a. Chain Naming: check cache -> query session intent -> call name_chain -> cache result
  |       b. Chain Summary: if should_summarize_chain() && cache miss -> aggregate excerpts -> call summarize_chain -> cache result
  |
  |-- 4. Inverted Index  [build_index_phase()]
  |     - build_inverted_index()
  |     - Result: files_indexed count
  |
  |-- Duration measurement and return SyncResult
```

### What gets synced, in what order:

1. **Git commits** - From configured repo, last N days, incremental
2. **Claude sessions** - From ~/.claude/projects/*.jsonl, incremental by file size comparison
3. **Chains** - Built from session data (linking related sessions)
4. **Intel enrichment** - AI-generated names and summaries for chains (optional)
5. **Inverted index** - File-to-session mapping for search

### Key data flow:

- Sessions are parsed from JSONL files on disk, then persisted to `claude_sessions` table
- Chains are computed from session data, then persisted to `chains` and `chain_graph` tables
- Intel enrichment reads chain data from the in-memory HashMap (NOT from DB) and caches results to `chain_metadata` and `chain_summaries` tables
- The query engine later reads from all these tables for display

---

## 4. Chain Naming Integration

### When does naming happen?

During `enrich_chains_phase()`, which is step 3.5 in the sync cycle -- AFTER chains are built and persisted, but BEFORE the inverted index is built.

### Full naming flow:

1. `enrich_chains_phase()` iterates over all chains in the `HashMap<String, Chain>`
2. For each chain, checks `cache.get_chain_name(chain_id)` -- if cached, skips
3. If NOT cached (needs naming):
   a. Queries `claude_sessions` table for root session's `conversation_excerpt` and `first_user_message` (via `query_session_intent()`)
   b. Builds `ChainNamingRequest` with:
      - `chain_id`
      - `files_touched` (from chain.files_list)
      - `session_count`
      - `recent_sessions` (first 5 session IDs)
      - `first_user_intent` (set to conversation_excerpt for backward compat)
      - `first_user_message` (for A/B testing)
      - `conversation_excerpt` (all user messages, ~8K chars)
      - `tools_used`: always `None` (TODO comment in code)
      - `commit_messages`: always `None` (TODO comment in code)
   c. Calls `client.name_chain(&request)` -> `POST /api/intel/name-chain`
   d. On success, caches via `cache.cache_chain_name(&response)`
4. Named count is accumulated and logged

### Chain Summary flow (Phase 6):

1. For each chain, also checks `should_summarize_chain()`:
   - `>=2` sessions, OR
   - `>30 minutes` duration, OR
   - `>10 files` touched
2. If "interesting" and not cached:
   a. Aggregates conversation excerpts from up to 10 sessions via `aggregate_chain_excerpts()`
   b. Loads existing workstream keys from `_system/state/workstreams.yaml` for hybrid tagging
   c. Calls `client.summarize_chain(&request)` -> `POST /api/intel/summarize-chain`
   d. Caches result

### How naming surfaces in query:

`query_chains()` in `query.rs` does a `LEFT JOIN chain_metadata cm ON cg.chain_id = cm.chain_id` to pull `generated_name` and `summary` into query results. The `compute_display_name()` function builds a display name preferring: `generated_name` > `first_user_message` > truncated `chain_id`.

---

## 5. Error Handling -- Intel Service Down

### Graceful degradation is the core design principle.

**Level 1: IntelClient methods** (`client.rs`)
- `name_chain()` and `summarize_chain()` NEVER return `Err` for network/service failures
- On timeout, connection refused, non-2xx status, or JSON parse failure: returns `Ok(None)`
- Logs warning with correlation_id and duration

**Level 2: enrich_chains_phase()** (`sync.rs`)
- First calls `client.health_check()` -- if `false`, pushes "Intel: Service unavailable - skipping enrichment" to `result.errors` and returns 0
- If health check passes but individual calls fail, the `Ok(None)` result means that chain is simply skipped
- Cache open failure: pushes error message, returns 0 (no enrichment)

**Level 3: run_sync()** (`sync.rs`)
- Intel enrichment is step 3.5 -- it runs conditionally (`if let Some(ref chains) = chains`)
- The function never fails the entire sync due to Intel issues
- `SyncResult.errors` acts as a message log (not just errors -- also "Intel: Named X chains, Summarized Y chains" success messages)

**Assessment:** The degradation is genuinely graceful. A completely offline Intel service results in:
- No chain names or summaries generated
- A single "Service unavailable" message in the error log
- All other sync phases (git, sessions, chains, index) complete normally
- Previously cached names/summaries remain available from SQLite

### Timeout behavior:
- HTTP client has a 10-second timeout (`Duration::from_secs(10)`)
- Tests verify requests complete within 15 seconds (buffer)
- No retry logic -- single attempt per chain per sync cycle

---

## 6. Configuration

### IntelClient Configuration

| Setting | Value | Source | Configurable? |
|---------|-------|--------|---------------|
| Base URL | `http://localhost:3002` | Hardcoded default in `IntelClient::default()` | Yes, via `IntelClient::new(url)` but daemon always uses `default()` |
| HTTP timeout | 10 seconds | Hardcoded in `IntelClient::new()` | No |
| Health endpoint | `/api/intel/health` | Hardcoded in `health_check()` | No |
| Name chain endpoint | `/api/intel/name-chain` | Hardcoded in `name_chain()` | No (derived from base_url) |
| Summarize endpoint | `/api/intel/summarize-chain` | Hardcoded in `summarize_chain()` | No (derived from base_url) |

### Daemon Configuration (`config.rs`)

Loaded from `~/.context-os/config.yaml`. Auto-created with defaults if missing.

| Config Key | Type | Default | Purpose |
|------------|------|---------|---------|
| `version` | u32 | 1 | Config version |
| `sync.interval_minutes` | u32 | 30 | Sync cycle interval |
| `sync.git_since_days` | u32 | 7 | How far back for git sync |
| `watch.enabled` | bool | true | File watching enabled |
| `watch.paths` | Vec<String> | ["."] | Paths to watch |
| `watch.debounce_ms` | u64 | 100 | Debounce window |
| `project.path` | Option<String> | None | Project path filter |
| `logging.level` | String | "INFO" | Log level |

### Daemon State (`state.rs`)

Persisted to JSON file. Tracks:
- `started_at`, `last_git_sync`, `last_session_parse`, `last_chain_build` (timestamps)
- `file_events_captured`, `git_commits_synced`, `sessions_parsed`, `chains_built` (counters)

### Database path

Hardcoded: `~/.context-os/context_os_events.db` (line 59 of sync.rs)

### Workstreams path

Derived from CWD: `{cwd}/_system/state/workstreams.yaml`

### GitOps rules path

Hardcoded: `~/.context-os/gitops-rules.yaml`

### Environment Variables

None used directly by the integration layer. The `APPDATA` env var is used by the Windows platform module for startup folder detection.

### What is NOT configurable (and probably should be):

- Intel service URL (hardcoded to localhost:3002 in daemon usage)
- HTTP timeout (hardcoded 10s)
- Database path (hardcoded ~/.context-os/)
- Max sessions to aggregate for excerpts (const `MAX_SESSIONS_TO_AGGREGATE = 10`)
- Max excerpt characters (const `MAX_EXCERPT_CHARS = 8000`)
- `should_summarize_chain` thresholds (hardcoded: >=2 sessions, >1800s, >10 files)

---

## 7. Context Restore: Could query_context() Call IntelClient Directly?

### Current state

`query.rs` has NO reference to `IntelClient` or the `intelligence` module. All query methods are purely deterministic SQL queries against the local SQLite database. Intel-generated data (names, summaries) reaches query results via `LEFT JOIN chain_metadata` / `chain_summaries` -- tables that were populated during daemon sync.

### Could it work?

**Technically yes.** `query_context()` (or any query method) could instantiate `IntelClient` and call endpoints directly, bypassing the daemon sync cycle. The pattern would be:

```rust
// Hypothetical: query_context with inline Intel call
pub async fn query_context(&self, chain_id: &str) -> Result<ContextResult, CoreError> {
    // 1. Deterministic data from DB
    let chain_data = self.query_chains_by_id(chain_id).await?;

    // 2. Check cache for Intel-generated metadata
    let cache = MetadataStore::new(&self.db_path).await?;
    let name = cache.get_chain_name(chain_id).await?;
    let summary = cache.get_chain_summary(chain_id).await?;

    // 3. If cache miss, call Intel directly
    if name.is_none() {
        let client = IntelClient::default();
        if let Ok(Some(response)) = client.name_chain(&build_request(chain_data)).await {
            cache.cache_chain_name(&response).await?;
        }
    }

    // 4. Assemble context result
    Ok(ContextResult { ... })
}
```

### Should it?

**Recommendation: Context restore should remain deterministic-only.** Reasons:

1. **Latency:** Intel calls add 1-10 seconds per chain. Context restore is a foreground operation where users expect immediate results. The daemon sync handles enrichment in the background where latency is invisible.

2. **Availability coupling:** If Intel service is down during a context restore request, the user experience degrades. The current architecture ensures context restore always works (from cached data).

3. **Separation of concerns:** The current architecture cleanly separates:
   - **Write path** (daemon sync): Handles all external service calls, caching, and data enrichment
   - **Read path** (query engine): Pure SQL queries, always fast, always available

4. **Cache coherence:** Having two code paths that can populate the cache (daemon sync AND query-time) creates ordering/consistency concerns.

5. **Already solved:** The daemon sync populates `chain_metadata` and `chain_summaries` tables. Query methods already JOIN against these tables. If Intel data exists, it appears in results. If not, results degrade to chain_id or first_user_message.

**The one exception worth considering:** A "lazy enrichment" pattern where the query layer checks cache, and if miss, queues a work item for the daemon rather than calling Intel directly. This would maintain the separation of concerns while ensuring chains get named on first access rather than waiting for the next sync cycle.

---

## Summary of Architecture

```
                    +------------------+
                    | Intel Service    |
                    | (TS, port 3002)  |
                    +--------+---------+
                             |
                    POST /api/intel/*
                             |
                    +--------+---------+
                    | IntelClient      |
                    | (Rust, client.rs)|
                    | - name_chain()   |
                    | - summarize()    |
                    | - health_check() |
                    +--------+---------+
                             |
                    Ok(Some(data)) or Ok(None)
                             |
            +----------------+----------------+
            |                                 |
   +--------v---------+            +----------v---------+
   | MetadataStore    |            | Daemon Sync        |
   | (cache.rs)       |            | (sync.rs)          |
   | - chain_metadata |            | - enrich_chains()  |
   | - chain_summaries|            | - query_intent()   |
   | - commit_analysis|            | - aggregate()      |
   | - insights_cache |            | - load_workstreams |
   +---------+--------+            +----+----+----+-----+
             |                          |    |    |
             |                     Phase 1  2  3  4
             |                     Git  Sess Chain Index
             |                          |    |
             +------+-------------------+----+
                    |
          +---------v-----------+
          | SQLite Database     |
          | (~/.context-os/)    |
          | - claude_sessions   |
          | - chains            |
          | - chain_graph       |
          | - chain_metadata    |
          | - chain_summaries   |
          | - git_commits       |
          +---------+-----------+
                    |
            SQL queries (read-only)
                    |
          +---------v-----------+
          | QueryEngine         |
          | (query.rs)          |
          | - query_chains()    |
          | - query_flex()      |
          | - query_search()    |
          +---------------------+
```

### Strengths

1. **Robust graceful degradation** -- Intel service outage never breaks core functionality
2. **Clean separation** -- write path (daemon) vs read path (query engine)
3. **Good observability** -- correlation IDs, structured logging, duration tracking
4. **Comprehensive test coverage** -- unit tests for all types, cache operations, sync phases, and graceful failure modes
5. **Incremental sync** -- session file size comparison avoids re-parsing unchanged data

### Risks / Technical Debt

1. **No cache TTL** -- chain names and summaries are never invalidated. If a chain grows (more sessions added), the cached name/summary may become stale.
2. **Two TODO items** in enrich_chains_phase: `tools_used` and `commit_messages` are always `None`. These enrichment signals are defined in types but never populated.
3. **Hardcoded Intel URL** -- daemon always uses `IntelClient::default()` (localhost:3002). No way to configure via config.yaml or environment variable.
4. **Duplicate connection pool** -- `enrich_chains_phase()` opens a SEPARATE `SqlitePool` (line 307) alongside the `QueryEngine`'s pool. This is because the engine doesn't expose its pool for raw queries. Works but wasteful.
5. **Error log misuse** -- `SyncResult.errors` is used for both actual errors AND informational messages (e.g., "Intel: Named 5 chains, Summarized 3 chains"). Should be separate fields.
6. **GitOps types defined but not wired into sync** -- `GitOpsSignals`, `GitOpsDecision`, `GitOpsAction` etc. are fully defined in types.rs, and `collect_gitops_signals()` exists in gitops.rs, but the sync cycle does NOT call any GitOps endpoints. The Intel service presumably has a `/api/intel/gitops-decide` endpoint (mentioned in types.rs comment) but no client method exists for it.

---

## Platform Module Summary

The `daemon/platform/` module provides cross-platform daemon registration:

| Platform | Implementation | Registration Method |
|----------|---------------|---------------------|
| Windows | `WindowsPlatform` | VBS script in Startup folder (no admin required). Fallback: Task Scheduler. |
| macOS | `MacOsPlatform` | launchd plist in `~/Library/LaunchAgents/` |
| Linux | `LinuxPlatform` | systemd user service in `~/.config/systemd/user/` |

All platforms implement the `DaemonPlatform` trait: `install()`, `uninstall()`, `is_installed()`, `status()`.

The platform module is independent from the Intel integration -- it simply starts/stops the daemon binary with `daemon start --interval N` arguments.
