# Rust Core Query Engine -- Deep Audit

**Date:** 2026-02-07
**Scope:** `apps/tastematter/core/src/` -- all source files
**Auditor:** Claude Opus 4.6 (automated)

---

## 1. Complete Function Inventory

### query.rs -- QueryEngine Methods

| # | Function | Visibility | Signature | Inputs | Outputs | Lines |
|---|----------|-----------|-----------|--------|---------|-------|
| 1 | `QueryEngine::new` | `pub` | `fn new(db: Database) -> Self` | `Database` | `QueryEngine` | 82-84 |
| 2 | `QueryEngine::database` | `pub` | `fn database(&self) -> &Database` | `&self` | `&Database` | 87-89 |
| 3 | `QueryEngine::query_flex` | `pub async` | `fn query_flex(&self, input: QueryFlexInput) -> Result<QueryResult, CoreError>` | time, chain, session, files, agg, limit, sort | `QueryResult {receipt_id, timestamp, result_count, results: Vec<FileResult>, aggregations}` | 100-205 |
| 4 | `QueryEngine::query_chains` | `pub async` | `fn query_chains(&self, input: QueryChainsInput) -> Result<ChainQueryResult, CoreError>` | limit | `ChainQueryResult {chains: Vec<ChainData>, total_chains}` | 210-284 |
| 5 | `QueryEngine::query_timeline` | `pub async` | `fn query_timeline(&self, input: QueryTimelineInput) -> Result<TimelineData, CoreError>` | time, files, chain, limit | `TimelineData {time_range, start_date, end_date, buckets, files, summary}` | 289-520 |
| 6 | `QueryEngine::query_sessions` | `pub async` | `fn query_sessions(&self, input: QuerySessionsInput) -> Result<SessionQueryResult, CoreError>` | time, chain, limit | `SessionQueryResult {time_range, sessions, chains, summary}` | 525-713 |
| 7 | `QueryEngine::query_search` | `pub async` | `fn query_search(&self, input: QuerySearchInput) -> Result<SearchResult, CoreError>` | pattern, limit | `SearchResult {receipt_id, timestamp, pattern, total_matches, results}` | 728-780 |
| 8 | `QueryEngine::query_file` | `pub async` | `fn query_file(&self, input: QueryFileInput) -> Result<FileQueryResult, CoreError>` | file_path, limit | `FileQueryResult {receipt_id, timestamp, file_path, found, matched_path, sessions}` | 786-914 |
| 9 | `QueryEngine::query_co_access` | `pub async` | `fn query_co_access(&self, input: QueryCoAccessInput) -> Result<CoAccessResult, CoreError>` | file_path, limit | `CoAccessResult {receipt_id, timestamp, query_file, results: Vec<CoAccessItem>}` | 920-1015 |
| 10 | `QueryEngine::query_heat` | `pub async` | `fn query_heat(&self, input: QueryHeatInput) -> Result<HeatResult, CoreError>` | time, files, limit, sort | `HeatResult {receipt_id, timestamp, time_range, results: Vec<HeatItem>, summary}` | 1027-1173 |
| 11 | `QueryEngine::query_verify` | `pub async` | `fn query_verify(&self, input: QueryVerifyInput) -> Result<VerifyResult, CoreError>` | receipt_id | `VerifyResult {receipt_id, status, original_timestamp, verified_at, drift_summary}` | 1179-1230 |
| 12 | `QueryEngine::query_receipts` | `pub async` | `fn query_receipts(&self, input: QueryReceiptsInput) -> Result<ReceiptsResult, CoreError>` | limit | `ReceiptsResult {receipts, total_count}` | 1235-1314 |
| 13 | `QueryEngine::insert_commit` | `pub async` | `fn insert_commit(&self, commit: &GitCommitInput) -> Result<WriteResult, CoreError>` | `&GitCommitInput` | `WriteResult` | 1327-1354 |
| 14 | `QueryEngine::insert_commits_batch` | `pub async` | `fn insert_commits_batch(&self, commits: &[GitCommitInput]) -> Result<WriteResult, CoreError>` | `&[GitCommitInput]` | `WriteResult` | 1366-1402 |
| 15 | `QueryEngine::insert_session` | `pub async` | `fn insert_session(&self, session: &SessionInput) -> Result<WriteResult, CoreError>` | `&SessionInput` | `WriteResult` | 1411-1443 |
| 16 | `QueryEngine::insert_file_event` | `pub async` | `fn insert_file_event(&self, event: &FileEvent) -> Result<WriteResult, CoreError>` | `&FileEvent` | `WriteResult` | 1452-1478 |
| 17 | `QueryEngine::insert_file_events` | `pub async` | `fn insert_file_events(&self, events: &[FileEvent]) -> Result<WriteResult, CoreError>` | `&[FileEvent]` | `WriteResult` | 1487-1519 |
| 18 | `QueryEngine::upsert_session` | `pub async` | `fn upsert_session(&self, session: &SessionInput) -> Result<WriteResult, CoreError>` | `&SessionInput` | `WriteResult` | 1536-1568 |
| 19 | `QueryEngine::get_session_file_sizes` | `pub async` | `fn get_session_file_sizes(&self) -> Result<HashMap<String, i64>, CoreError>` | `&self` | `HashMap<String, i64>` | 1574-1589 |
| 20 | `QueryEngine::persist_chains` | `pub async` | `fn persist_chains(&self, chains: &HashMap<String, Chain>) -> Result<WriteResult, CoreError>` | `&HashMap<String, Chain>` | `WriteResult` | 1602-1710 |

**Standalone functions in query.rs:**

| # | Function | Visibility | Signature | Lines |
|---|----------|-----------|-----------|-------|
| 21 | `generate_receipt_id` | private | `fn generate_receipt_id() -> String` | 17-25 |
| 22 | `compute_display_name` | private | `fn compute_display_name(chain_id, generated_name, first_user_message) -> String` | 33-71 |
| 23 | `compute_aggregations` | `pub` | `fn compute_aggregations(results: &[FileResult], agg_types: &[String]) -> Aggregations` | 1714-1743 |

### storage.rs -- Database Methods

| # | Function | Visibility | Signature | Lines |
|---|----------|-----------|-----------|-------|
| 1 | `Database::open` | `pub async` | `fn open(path: impl AsRef<Path>) -> Result<Self, CoreError>` | 50-84 |
| 2 | `Database::open_rw` | `pub async` | `fn open_rw(path: impl AsRef<Path>) -> Result<Self, CoreError>` | 96-111 |
| 3 | `Database::ensure_schema` | `pub async` | `fn ensure_schema(&self) -> Result<(), CoreError>` | 130-275 |
| 4 | `Database::pool` | `pub` | `fn pool(&self) -> &SqlitePool` | 280-282 |
| 5 | `Database::path` | `pub` | `fn path(&self) -> &Path` | 285-287 |
| 6 | `Database::canonical_path` | `pub` | `fn canonical_path() -> Result<PathBuf, CoreError>` | 294-298 |
| 7 | `Database::find_database` | `pub` | `fn find_database(explicit_path: Option<&Path>) -> Result<PathBuf, CoreError>` | 310-339 |
| 8 | `Database::open_default` | `pub async` | `fn open_default() -> Result<Self, CoreError>` | 344-347 |
| 9 | `Database::close` | `pub async` | `fn close(self)` | 353-355 |

### types.rs -- Public Functions

| # | Function | Visibility | Signature | Lines |
|---|----------|-----------|-----------|-------|
| 1 | `parse_time_range` | `pub` | `fn parse_time_range(time: &str) -> Result<i64, CoreError>` | 647-665 |
| 2 | `classify_heat` | `pub` | `fn classify_heat(heat_score: f64) -> HeatLevel` | 675-685 |
| 3 | `compute_velocity` | `pub` | `fn compute_velocity(count_long: u32, first_access: &str, last_access: &str) -> f64` | 691-697 |
| 4 | `compute_heat_score` | `pub` | `fn compute_heat_score(velocity: f64, rcr: f64, last_access: &str) -> f64` | 768-772 |
| 5 | `compute_days_active` | private | `fn compute_days_active(first_access: &str, last_access: &str) -> i64` | 703-724 |
| 6 | `compute_recency_bonus` | private | `fn compute_recency_bonus(last_access: &str) -> f64` | 732-762 |

### http.rs -- Public API

| # | Function | Visibility | Signature | Lines |
|---|----------|-----------|-----------|-------|
| 1 | `create_router` | `pub` | `fn create_router(state: Arc<AppState>, enable_cors: bool) -> Router` | 70-89 |
| 2 | `health_handler` | private | `async fn health_handler(State) -> Json<HealthStatus>` | 92-99 |
| 3 | `query_flex_handler` | private | `async fn query_flex_handler(State, Json<QueryFlexInput>) -> Result<Json<QueryResult>, ...>` | 102-112 |
| 4 | `query_timeline_handler` | private | `async fn query_timeline_handler(State, Json<QueryTimelineInput>) -> Result<Json<TimelineData>, ...>` | 115-125 |
| 5 | `query_sessions_handler` | private | `async fn query_sessions_handler(State, Json<QuerySessionsInput>) -> Result<Json<SessionQueryResult>, ...>` | 128-138 |
| 6 | `query_chains_handler` | private | `async fn query_chains_handler(State, Json<QueryChainsInput>) -> Result<Json<ChainQueryResult>, ...>` | 141-151 |

### main.rs -- CLI-Only Functions

| # | Function | Visibility | Signature | Lines |
|---|----------|-----------|-----------|-------|
| 1 | `main` | (entry) | `async fn main() -> Result<(), Box<dyn std::error::Error>>` | 398-1325 |
| 2 | `output` | private | `fn output<T: Serialize>(data: &T, format: &str) -> Result<...>` | 1328-1335 |
| 3 | `output_heat_table` | private | `fn output_heat_table(result: &HeatResult)` | 1338-1379 |
| 4 | `output_chains_table` | private | `fn output_chains_table(result: &ChainQueryResult)` | 1382-1412 |
| 5 | `output_heat_csv` | private | `fn output_heat_csv(result: &HeatResult)` | 1415-1433 |

### error.rs -- Public Types

| # | Type | Variants | Lines |
|---|------|----------|-------|
| 1 | `CoreError` (enum) | `Database`, `Query`, `Config`, `Serialization`, `IntelServiceUnavailable`, `IntelServiceError` | 8-27 |
| 2 | `CommandError` (struct) | `code`, `message`, `details` | 31-36 |

---

## 2. Composition Patterns

### Internal Query Chaining

**`query_sessions` is the most compositional query.** It internally executes:

1. **Main sessions query** (lines 536-581): Fetches session rows with chain_id, chain_name, file_count, total_accesses via LEFT JOIN on chain_graph and chain_metadata.
2. **Per-session file sub-query** (lines 605-621): For each session, fetches top 5 files (UNION of files_read and files_written). This is an N+1 pattern -- one additional SQL query per session row.
3. **Chain summaries query** (lines 650-677): A CTE-based aggregation across all_files and chain_graph to compute per-chain file counts and last_active.

**`query_timeline` runs 3 SQL queries sequentially:**

1. **Daily bucket query** (lines 298-333): Aggregates access_count and files_touched by date.
2. **Per-file aggregation** (lines 362-408): Aggregates total_accesses, first/last access per file.
3. **Per-file per-date buckets** (lines 411-462): Granular counts for heatmap rendering.

Then merges results in Rust (lines 447-486, 488-519).

**`query_flex`** is a single-query pattern: builds one dynamic SQL with optional WHERE clauses and runs it once.

**`query_co_access`** uses a 2-step pattern:
1. Find sessions touching the anchor file (lines 929-943).
2. Find all files in those sessions, excluding the anchor (lines 960-991).
PMI scoring is computed in Rust after SQL returns raw co-occurrence counts.

**`query_heat`** uses a single SQL + Rust computation pattern:
1. One CTE query fetches count_7d and count_long per file (lines 1042-1074).
2. RCR, velocity, heat_score, heat_level computed in Rust (lines 1077-1113).
3. Sort and truncate in Rust (lines 1116-1136).

### Where One Query Calls Another

**No query currently calls another query.** All query methods are independent -- they share only the `Database` pool via `self.db.pool()`. This is by design (each query is a standalone SQL operation).

However, `compute_aggregations` is used by `query_flex` (line 193), and the heat metric functions (`compute_velocity`, `compute_heat_score`, `classify_heat`) are used by `query_heat` (lines 1097-1099).

### Shared SQL Patterns

All queries that touch file access data use the same CTE pattern (introduced by BUG-05 fix):

```sql
WITH all_files AS (
    SELECT s.session_id, s.started_at, json_each.value as file_path
    FROM claude_sessions s, json_each(s.files_read)
    WHERE s.files_read IS NOT NULL AND s.files_read != '[]'
    UNION ALL
    SELECT s.session_id, s.started_at, json_each.value as file_path
    FROM claude_sessions s, json_each(s.files_written)
    WHERE s.files_written IS NOT NULL AND s.files_written != '[]'
)
```

This CTE is duplicated across: `query_flex`, `query_timeline` (3x), `query_sessions` (2x), `query_search`, `query_file`, `query_co_access` (2x), `query_heat`.

---

## 3. HTTP API Surface

**File:** `http.rs` (152 lines)
**Framework:** Axum + tower_http (CORS)
**State:** `Arc<AppState>` containing `QueryEngine` + `start_time`

### Endpoints

| Method | Path | Handler | Input Type | Output Type | Query Method |
|--------|------|---------|------------|-------------|--------------|
| GET | `/api/health` | `health_handler` | -- | `HealthStatus` | -- |
| POST | `/api/query/flex` | `query_flex_handler` | `QueryFlexInput` (JSON body) | `QueryResult` | `engine.query_flex()` |
| POST | `/api/query/timeline` | `query_timeline_handler` | `QueryTimelineInput` (JSON body) | `TimelineData` | `engine.query_timeline()` |
| POST | `/api/query/sessions` | `query_sessions_handler` | `QuerySessionsInput` (JSON body) | `SessionQueryResult` | `engine.query_sessions()` |
| POST | `/api/query/chains` | `query_chains_handler` | `QueryChainsInput` (JSON body) | `ChainQueryResult` | `engine.query_chains()` |

### Missing from HTTP

The following CLI query commands have **no HTTP endpoint**:

- `query search` (`query_search`)
- `query file` (`query_file`)
- `query co-access` (`query_co_access`)
- `query heat` (`query_heat`)
- `query verify` (`query_verify`)
- `query receipts` (`query_receipts`)

### Error Handling

`CoreError` converts to `(StatusCode::BAD_REQUEST, Json<ApiError>)` via the `From` impl (lines 59-67). All query errors surface as 400 with `error: "QueryError"`.

### CORS

Enabled via `--cors` flag. When enabled, allows `Any` origin, methods, and headers. The server binds to localhost only and is documented as "NOT for production use."

---

## 4. CLI Command Structure

### Clap Hierarchy

```
tastematter [--db PATH]
  query
    flex    --time --chain --files --session --agg --limit --sort --format
    chains  --limit --format
    timeline --time --files --chain --limit --format
    sessions --time --chain --limit --format
    search  PATTERN --limit --format
    file    FILE_PATH --limit --format
    co-access FILE_PATH --limit --format
    heat    --time --files --limit --sort --format
    verify  RECEIPT_ID --format
    receipts --limit --format
  serve     --port --host --cors
  sync-git  --since --until --repo --format
  parse-sessions --claude-dir --project --incremental --format
  build-chains   --claude-dir --project --format
  index-files    --claude-dir --project --format --query
  watch     --path --debounce-ms --duration --recursive
  daemon
    once    --project
    start   --interval --project
    status
    install --interval
    uninstall
  intel
    health
    name-chain CHAIN_ID --files --session-count
```

### Pattern for Adding a New Subcommand

1. **Add variant to `QueryCommands` enum** (line 221+):
   ```rust
   NewQuery {
       #[arg(short, long)]
       param: String,
       #[arg(long, default_value = "json")]
       format: String,
   },
   ```

2. **Add input type to `types.rs`**:
   ```rust
   pub struct QueryNewInput { pub param: String, pub limit: Option<u32> }
   ```

3. **Add output type to `types.rs`**:
   ```rust
   pub struct NewResult { pub receipt_id: String, ... }
   ```

4. **Add query method to `QueryEngine` in `query.rs`**:
   ```rust
   pub async fn query_new(&self, input: QueryNewInput) -> Result<NewResult, CoreError> { ... }
   ```

5. **Add dispatch in `main.rs` match arm** (inside `Commands::Query { query_type } => match query_type {`):
   ```rust
   QueryCommands::NewQuery { param, format } => {
       let input = QueryNewInput { param, limit: Some(20) };
       let result = engine.query_new(input).await?;
       result_count = Some(result.results.len() as u32);
       output(&result, &format)?;
   }
   ```

6. **Add telemetry command name** in the `command_name` match (line 410+):
   ```rust
   QueryCommands::NewQuery { .. } => "query_new",
   ```

7. **(Optional) Add HTTP endpoint** in `http.rs`:
   - Add handler function
   - Add `.route("/api/query/new", post(new_handler))` to `create_router`

---

## 5. Data Flow -- Complete Trace

### Example: `tastematter query heat --time 30d --files "*.rs" --limit 10`

```
CLI Input
    |
    v
[1] Clap parse (main.rs:399)
    Cli { db: None, command: Commands::Query { query_type: QueryCommands::Heat {
        time: "30d", files: Some("*.rs"), limit: 10, sort: "heat", format: "table"
    }}}
    |
    v
[2] Telemetry init + command_name extraction (main.rs:402-452)
    command_name = "query_heat"
    time_range_bucket = TimeRangeBucket::from_time_arg("30d")
    |
    v
[3] Database open (main.rs:600-604)
    Database::open_default() -> finds ~/.context-os/context_os_events.db
    Opens SQLite pool (5 connections, read-only mode)
    |
    v
[4] QueryEngine::new(db) (main.rs:606)
    |
    v
[5] Build QueryHeatInput (main.rs:728-733)
    QueryHeatInput { time: Some("30d"), files: Some("*.rs"), limit: Some(10), sort: Some(HeatSortBy::Heat) }
    |
    v
[6] engine.query_heat(input) (query.rs:1027)
    |
    v
[6a] parse_time_range("30d") -> 30 (types.rs:647)
    |
    v
[6b] Build SQL with CTE (query.rs:1042-1065)
    WITH all_files AS (
        SELECT s.session_id, s.started_at, json_each.value as file_path
        FROM claude_sessions s, json_each(s.files_read)
        WHERE s.started_at >= datetime('now', '-30 days')
          AND s.files_read IS NOT NULL AND s.files_read != '[]'
        UNION ALL
        SELECT s.session_id, s.started_at, json_each.value as file_path
        FROM claude_sessions s, json_each(s.files_written)
        WHERE s.started_at >= datetime('now', '-30 days')
          AND s.files_written IS NOT NULL AND s.files_written != '[]'
    )
    SELECT af.file_path,
            SUM(CASE WHEN af.started_at >= datetime('now', '-7 days') THEN 1 ELSE 0 END) as count_7d,
            COUNT(*) as count_long,
            MIN(af.started_at) as first_access,
            MAX(af.started_at) as last_access
     FROM all_files af
     WHERE 1=1
       AND af.file_path LIKE ?        -- bound to "%.rs"
     GROUP BY af.file_path
    |
    v
[6c] Bind "%.rs" (query.rs:1069-1072) -- glob "*" converted to SQL "%"
    |
    v
[6d] Execute SQL via sqlx (query.rs:1074)
    rows = query.fetch_all(self.db.pool()).await?
    |
    v
[6e] Compute metrics in Rust (query.rs:1077-1113)
    For each row:
      rcr = count_7d / count_long
      velocity = compute_velocity(count_long, first_access, last_access)
      heat_score = compute_heat_score(velocity, rcr, last_access)
      heat_level = classify_heat(heat_score)
    |
    v
[6f] Sort by heat_score DESC (query.rs:1117-1121)
    |
    v
[6g] Truncate to limit=10 (query.rs:1136)
    |
    v
[6h] Compute summary (query.rs:1139-1155)
    HeatSummary { total_files, hot_count, warm_count, cool_count, cold_count }
    |
    v
[6i] Return HeatResult (query.rs:1160-1172)
    HeatResult { receipt_id, timestamp, time_range: "30d", results: Vec<HeatItem>, summary }
    |
    v
[7] Output (main.rs:736-739)
    format == "table" -> output_heat_table(&query_result) (main.rs:1338-1379)
    Prints formatted ASCII table to stdout
    |
    v
[8] Telemetry capture (main.rs:1306-1322)
    CommandExecutedEvent { command: "query_heat", duration_ms, success: true, result_count: 10, time_range: "30d" }
    Fire-and-forget (non-blocking)
```

### Type serialization chain

```
Rust HeatItem struct  ->  serde_json::to_string_pretty  ->  stdout (JSON)
                                                        ->  table formatter (table format)
                                                        ->  CSV formatter (csv format)
```

---

## 6. Reuse for `query_context()` -- Context Restore Composition

### Goal

A single `query_context(file_paths, time_range)` call that returns flex + heat + co-access + chains data in one response, suitable for restoring session context.

### Directly Reusable Functions

| Function | Reusable? | Notes |
|----------|-----------|-------|
| `query_flex` | YES | Pass file pattern, get access counts and recency. Already supports chain/session/time filters. |
| `query_heat` | YES | Pass file pattern, get heat metrics. Already supports time and file filters. |
| `query_co_access` | PARTIALLY | Only takes a single anchor file. Would need to be called N times (once per file) or refactored to accept multiple anchors. |
| `query_chains` | YES | Returns all chains with display names and summaries. Can be filtered client-side. |
| `query_sessions` | YES | Returns sessions with chain context and top files. Useful for temporal context. |
| `compute_aggregations` | YES | Standalone function, can be called on any `Vec<FileResult>`. |
| `compute_heat_score` / `classify_heat` | YES | Pure functions, no DB dependency. |

### Composition Strategy

```rust
pub struct ContextRestoreResult {
    pub flex: QueryResult,        // File access patterns
    pub heat: HeatResult,         // Heat classification
    pub co_access: Vec<CoAccessResult>,  // Per-file co-access
    pub chains: ChainQueryResult, // Chain metadata
    pub sessions: SessionQueryResult, // Recent sessions
}

pub async fn query_context(
    &self,
    files: &[String],     // anchor files to restore context for
    time: &str,           // e.g. "30d"
) -> Result<ContextRestoreResult, CoreError> {
    // All queries share the same DB pool -- can run concurrently via tokio::join!
    let file_pattern = files.join(",");  // or run multiple flex queries

    let (flex, heat, chains, sessions) = tokio::join!(
        self.query_flex(QueryFlexInput {
            time: Some(time.to_string()),
            files: Some(file_pattern.clone()),
            agg: vec!["count".into(), "recency".into()],
            limit: Some(50),
            ..Default::default()
        }),
        self.query_heat(QueryHeatInput {
            time: Some(time.to_string()),
            files: Some(file_pattern),
            limit: Some(50),
            ..Default::default()
        }),
        self.query_chains(QueryChainsInput { limit: Some(20) }),
        self.query_sessions(QuerySessionsInput {
            time: time.to_string(),
            chain: None,
            limit: Some(20),
        }),
    );

    // Co-access: run per anchor file (or batch)
    let mut co_access_results = Vec::new();
    for file in files {
        let co = self.query_co_access(QueryCoAccessInput {
            file_path: file.clone(),
            limit: Some(10),
        }).await?;
        co_access_results.push(co);
    }

    Ok(ContextRestoreResult {
        flex: flex?,
        heat: heat?,
        co_access: co_access_results,
        chains: chains?,
        sessions: sessions?,
    })
}
```

### Bottlenecks and Considerations

1. **Co-access N+1 problem**: `query_co_access` runs 2 SQL queries per anchor file. For 10 anchor files = 20 SQL round-trips. Consider a batch variant that accepts multiple anchors.

2. **CTE duplication**: The `all_files` CTE is duplicated 11 times across query functions. A shared SQL builder or view would reduce duplication and ensure consistency.

3. **No concurrent execution today**: All queries are called sequentially in the CLI. `tokio::join!` would let them run concurrently against the connection pool (5 connections).

4. **`query_sessions` N+1 pattern**: Runs a per-session sub-query for top files (lines 605-621). With 20 sessions, that's 20 additional queries. For context restore, consider a batch file query instead.

5. **Heat computation is CPU-bound**: The Rust-side computation (RCR, velocity, heat_score) for 50 files is negligible (<1ms), so this is not a bottleneck.

6. **Receipt IDs**: `query_flex` uses `uuid::Uuid::new_v4()` while `query_search/file/co_access/heat` use `generate_receipt_id()` (hash-based). Inconsistent but not blocking.

---

## Appendix A: Database Schema (from storage.rs ensure_schema)

| Table | Purpose | Primary Key |
|-------|---------|-------------|
| `file_events` | File system events | `id` (auto) |
| `claude_sessions` | Parsed session data | `session_id` |
| `git_commits` | Git history | `hash` |
| `chains` | Chain metadata | `chain_id` |
| `chain_graph` | Session-to-chain mapping | `session_id` |
| `chain_metadata` | Intel-generated names/summaries | `chain_id` |
| `chain_summaries` | Intel-generated chain summaries | `chain_id` |
| `_metadata` | Schema version tracking | `key` |

## Appendix B: Module Dependency Graph

```
lib.rs (re-exports)
  |
  +-- query.rs      depends on: storage.rs, types.rs, error.rs, index/chain_graph.rs
  +-- storage.rs     depends on: error.rs
  +-- types.rs       depends on: error.rs, capture/jsonl_parser.rs (From impl)
  +-- http.rs        depends on: query.rs, types.rs, error.rs
  +-- error.rs       standalone (thiserror)
  +-- main.rs        depends on: all of the above + capture/* + daemon/* + intelligence/* + telemetry/*
  +-- capture/
  |     +-- jsonl_parser.rs   (JSONL 3-source extraction, session aggregation)
  |     +-- git_sync.rs       (git log parsing)
  |     +-- git_status.rs     (git status parsing)
  |     +-- file_watcher.rs   (notify-based FS watcher)
  +-- index/
  |     +-- chain_graph.rs    (5-pass chain building from JSONL linking)
  |     +-- inverted_index.rs (file -> sessions mapping)
  +-- daemon/        (background sync, platform-specific install)
  +-- intelligence/  (Intel service client, chain naming)
  +-- telemetry/     (PostHog-compatible event capture)
```

## Appendix C: Test Coverage Summary

| File | Test Count | Key Test Areas |
|------|-----------|----------------|
| `query.rs` | 12 | aggregations, display_name, persist_chains idempotency/stale removal |
| `storage.rs` | 12 | open/open_rw, ensure_schema idempotency, migration, unified schema |
| `types.rs` | 22 | serialization, heat classification, velocity, recency, heat score, time parsing |
| `chain_graph.rs` | 18 | leafUuid extraction, agent parent, BFS chains, branching, disconnected |
| `inverted_index.rs` | 14 | access classification, path extraction, dedup, index building |
| `jsonl_parser.rs` | 31 | path encoding, file extraction, 3-source dispatch, aggregation, normalization |
| **Total** | **109** | |
