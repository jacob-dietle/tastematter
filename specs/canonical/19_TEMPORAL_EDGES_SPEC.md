---
title: "Temporal Edges Specification"
type: architecture-spec
created: 2026-02-17
last_updated: 2026-02-17
status: draft
foundation:
  - "[[canonical/07_CLAUDE_CODE_DATA_MODEL_V2]]"
  - "[[canonical/03_CORE_ARCHITECTURE]]"
  - "[[canonical/12_CONTEXT_RESTORATION_API_SPEC]]"
  - "[[context_packages/03_current/38_TEMPORAL_EDGES_DESIGN]]"
  - "[[context_packages/03_current/39_TEMPORAL_SIGNAL_VALIDATION_PASS]]"
related:
  - "[[core/src/storage.rs]]"
  - "[[core/src/daemon/sync.rs]]"
  - "[[core/src/capture/jsonl_parser.rs]]"
  - "[[core/src/query.rs]]"
  - "[[core/src/context_restore.rs]]"
  - "[[core/src/types.rs]]"
tags:
  - tastematter
  - temporal-edges
  - architecture
  - canonical
---

# Temporal Edges Specification

## Executive Summary

Add typed, directed behavioral edges to tastematter's context restoration pipeline. These edges capture **how files relate through work patterns** — not co-occurrence counts (PMI), but ordered relationships like "read A, then edited B" extracted deterministically from per-tool-call timestamps in session JSONL.

**Empirical validation:** 7/7 sampled sessions show clear temporal signal. 62 avg read→edit patterns/session. Explore burst noise is 1-9%, trivially filterable. Every tool call gets its own JSONL record with a unique millisecond timestamp (0% multi-tool records). [VERIFIED: [[_system/scripts/temporal_signal_validation.py]], context package #39]

**Design principle** (from CodeGraph teardown): "Extract typed, directional relationships from deterministic data and make them queryable as a graph." CodeGraph does this for AST → symbol graph. Tastematter does this for sessions → behavior graph.

---

## Problem Statement

### Current State

The context restoration pipeline (`query_context()`, query.rs:1350-1454) uses **undirected, unordered** file relationships:

- **PMI co-access** (Phase 2): "A and B appeared in the same sessions" — no direction, no ordering
- **WorkCluster** output: `{ files: [...], pmi_score: 0.55 }` — a bag of files
- **Continuity**: `left_off_at` based on filesystem tier, not actual behavioral sequence

The JSONL parser already extracts per-tool-call data with timestamps:

```rust
// capture/jsonl_parser.rs:34-50
pub struct ToolUse {
    pub id: String,
    pub name: String,                    // "Read", "Edit", "Grep"
    pub timestamp: DateTime<Utc>,        // MILLISECOND precision, UNIQUE per record
    pub file_path: Option<String>,
    pub is_read: bool,
    pub is_write: bool,
}
```

But during session summarization (jsonl_parser.rs:559-695), this collapses to:

```rust
files_read_set: HashSet<String>,         // DEDUPLICATED — no order, no timestamps
files_written_set: HashSet<String>,      // DEDUPLICATED — no order, no timestamps
tools_used: HashMap<String, i32>,        // COUNTS ONLY — no sequence
```

**The entire within-session ordering is destroyed.** The database knows "session touched A, B, C" but not "A was read at 14:30:01, then B at 14:30:15, then C was edited at 14:31:02."

### What We're Missing

From the validation (package #39), real session temporal sequences encode clear work patterns:

```
Session 5ae59 (implementation):
  R: SKILL.md → R: context.md → R: architecture.md → R: source.js
  → W: plan.md → W: entry.js → W: entry.js (×4) → W: spec.md (×3)
  Pattern: Load context → Read architecture → Read sources → Write plan → Implement → Iterate

Session 8aa92 (TDD):
  R: SKILL.md → R: sync.rs → R: query.rs → R: parser.rs
  → W: plan.md → R: main.rs → W: plan.md
  → R: storage.rs → W: integration_test.rs → W: common/mod.rs
  Pattern: Load TDD skill → Read 4 files → Write plan → Write tests first → Implement
```

These sequences answer questions PMI cannot:
- **Which file should I read FIRST?** (entry points — consistently `source` in read_before edges)
- **What will I end up editing?** (work targets — consistently `target` in read_then_edit edges)
- **What's the typical workflow?** (topological sort of read_before chains)
- **Am I mid-workflow?** (compare last session's sequence against typical pattern)

---

## Architecture: Three-Layer Rollup

```
Layer 1: file_access_events (~190K rows)     ← Stored at parse time
    │    Raw per-tool-call records
    │    One row per ToolUse with timestamp + session + file
    ▼
Layer 2: file_edges (~10K-50K rows)          ← Batch extracted during daemon sync
    │    Aggregated behavioral edges
    │    source_file → target_file with type + confidence
    ▼
Layer 3: work_patterns (~3-8 per query)      ← Computed at query time
         entry_points, work_targets, typical_sequence, incomplete_sequence
         Enhances existing WorkCluster and Continuity output
```

**Why three layers:**
- Layer 1 is write-once, read-many (190K rows, indexed, fast)
- Layer 2 is batch-computed (amortized cost, incremental)
- Layer 3 is lightweight query-time computation (50 rows → 3-8 patterns)

**Precedent:** Same architecture as existing chain_graph (sessions → chains → chain names). Layer 1 = claude_sessions. Layer 2 = chain_graph + chains. Layer 3 = chain_metadata.

---

## Phase 1: Schema Migration

**Complexity:** Low
**Files:** `core/src/storage.rs`
**Depends on:** Nothing (first step)

### 1.1 New Tables

Add to `ensure_schema()` SCHEMA_SQL (storage.rs:132-242):

```sql
-- Layer 1: File Access Events (per-tool-call temporal data)
CREATE TABLE IF NOT EXISTS file_access_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    file_path TEXT NOT NULL,
    tool_name TEXT NOT NULL,
    access_type TEXT NOT NULL,
    sequence_position INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_fae_session ON file_access_events(session_id);
CREATE INDEX IF NOT EXISTS idx_fae_file ON file_access_events(file_path);
CREATE INDEX IF NOT EXISTS idx_fae_session_seq ON file_access_events(session_id, sequence_position);

-- Layer 2: File Edges (aggregated behavioral relationships)
CREATE TABLE IF NOT EXISTS file_edges (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_file TEXT NOT NULL,
    target_file TEXT NOT NULL,
    edge_type TEXT NOT NULL,
    session_count INTEGER NOT NULL DEFAULT 0,
    total_sessions_with_source INTEGER NOT NULL DEFAULT 0,
    avg_time_delta_seconds REAL,
    confidence REAL NOT NULL DEFAULT 0.0,
    first_seen TEXT,
    last_seen TEXT
);
CREATE INDEX IF NOT EXISTS idx_fe_source ON file_edges(source_file, edge_type);
CREATE INDEX IF NOT EXISTS idx_fe_target ON file_edges(target_file, edge_type);
CREATE INDEX IF NOT EXISTS idx_fe_type_conf ON file_edges(edge_type, confidence DESC);
CREATE UNIQUE INDEX IF NOT EXISTS idx_fe_unique ON file_edges(source_file, target_file, edge_type);
```

### 1.2 Schema Version

Update `_metadata` schema version from `2.2` to `2.3` (storage.rs:241):

```sql
INSERT OR REPLACE INTO _metadata (key, value) VALUES ('schema_version', '2.3');
```

### 1.3 Migration Safety

Both tables use `CREATE TABLE IF NOT EXISTS` — idempotent on existing databases. No ALTER TABLE needed. Existing data untouched.

The unique index on `file_edges(source_file, target_file, edge_type)` enables `INSERT OR REPLACE` for batch updates without duplicates.

### 1.4 Design Decisions

**Why `access_type TEXT` instead of `is_read BOOLEAN, is_write BOOLEAN`:**
- Three values: `read`, `write`, `search`
- Single column simplifies queries (`WHERE access_type = 'read'`)
- Matches the validation script's classification
- Extensible without schema change

**Why `sequence_position INTEGER` instead of relying on timestamp ordering:**
- Explicit ordering is cheaper to query than `ORDER BY timestamp`
- Pre-computed during insert (parser already iterates in order)
- Handles edge case of identical timestamps (shouldn't happen per validation, but defensive)

**Why `confidence REAL` on file_edges:**
- `confidence = session_count / total_sessions_with_source`
- Pre-computed to avoid division at query time
- Enables `WHERE confidence >= 0.5` filter without subquery

**Why UNIQUE index on (source_file, target_file, edge_type):**
- Each directional relationship has exactly one aggregated row
- `INSERT OR REPLACE` enables full rebuild without DELETE
- Matches CodeGraph's principle: one edge per typed relationship

### 1.5 Tests

```rust
#[tokio::test]
async fn test_ensure_schema_creates_temporal_tables() {
    // Fresh DB → ensure_schema → both tables exist
    // Verify file_access_events columns via PRAGMA table_info
    // Verify file_edges columns via PRAGMA table_info
}

#[tokio::test]
async fn test_ensure_schema_preserves_existing_data_with_temporal_tables() {
    // Insert data into claude_sessions → ensure_schema again → data intact
    // Insert data into file_access_events → ensure_schema again → data intact
}

#[tokio::test]
async fn test_file_edges_unique_constraint() {
    // INSERT same (source, target, type) twice → only 1 row (OR REPLACE)
}

#[tokio::test]
async fn test_file_access_events_insert_and_query() {
    // Insert 5 events for a session → query by session_id → 5 rows ordered by sequence_position
}
```

---

## Phase 2: Parser Integration (Event Persistence)

**Complexity:** Medium
**Files:** `core/src/capture/jsonl_parser.rs`, `core/src/daemon/sync.rs`, `core/src/query.rs`
**Depends on:** Phase 1 (schema must exist)

### 2.1 Change `sync_sessions` to Return ToolUse Records

The key insight: `sync_sessions()` (jsonl_parser.rs:856-945) already parses all ToolUse records from messages. They're counted (line 922) but not returned. We need to return them alongside the session summaries.

**Option A (chosen): Return tool uses per session alongside summaries**

Add a new return type that pairs summaries with their tool uses:

```rust
/// A parsed session with both summary and raw tool uses for temporal storage.
pub struct ParsedSessionData {
    pub summary: SessionSummary,
    pub tool_uses: Vec<ToolUse>,
}
```

Modify `sync_sessions` to collect tool uses per session:

```rust
// In sync_sessions, after parse_session_file returns messages:
let tool_uses: Vec<ToolUse> = messages
    .iter()
    .flat_map(|m| m.tool_uses.iter().cloned())
    .filter(|tu| tu.file_path.is_some())  // Only file-relevant tool uses
    .filter(|tu| {
        // Skip GREP:/GLOB: pseudo-paths
        tu.file_path.as_ref()
            .map(|p| !p.starts_with("GREP:") && !p.starts_with("GLOB:"))
            .unwrap_or(false)
    })
    .collect();
```

**Why Option A over alternatives:**
- **Option B** (re-parse JSONL for tool uses separately) wastes I/O — we already have the data
- **Option C** (store in SessionSummary) pollutes the summary type with temporal data
- Option A cleanly separates concerns: summary for session-level, tool_uses for event-level

### 2.2 Persist Events in Sync Phase

In `sync_sessions_phase()` (daemon/sync.rs:140-201), after upserting the session summary, also batch-insert the tool uses:

```rust
// After engine.upsert_session(&input):
if !parsed.tool_uses.is_empty() {
    match engine.insert_file_access_events(
        &parsed.summary.session_id,
        &parsed.tool_uses,
    ).await {
        Ok(count) => { /* log */ }
        Err(e) => {
            result.errors.push(format!(
                "File access events for {}: {}",
                &parsed.summary.session_id[..8], e
            ));
        }
    }
}
```

### 2.3 Batch Insert Function

Add to `QueryEngine` (query.rs):

```rust
/// Insert file access events for a session (batch, transactional).
///
/// Deletes existing events for the session first (idempotent re-sync),
/// then inserts all new events in a single transaction.
pub async fn insert_file_access_events(
    &self,
    session_id: &str,
    tool_uses: &[ToolUse],
) -> Result<usize, CoreError> {
    let pool = self.database().pool();

    // Delete existing events for this session (re-sync safe)
    sqlx::query("DELETE FROM file_access_events WHERE session_id = ?")
        .bind(session_id)
        .execute(pool)
        .await
        .map_err(CoreError::Database)?;

    // Batch insert with explicit transaction
    let mut tx = pool.begin().await.map_err(CoreError::Database)?;

    for (seq, tu) in tool_uses.iter().enumerate() {
        let access_type = if tu.is_write { "write" }
            else if tu.is_read { "read" }
            else { "other" };

        sqlx::query(
            "INSERT INTO file_access_events \
             (session_id, timestamp, file_path, tool_name, access_type, sequence_position) \
             VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(session_id)
        .bind(tu.timestamp.to_rfc3339())
        .bind(tu.file_path.as_ref().unwrap())
        .bind(&tu.name)
        .bind(access_type)
        .bind(seq as i32)
        .execute(&mut *tx)
        .await
        .map_err(CoreError::Database)?;
    }

    tx.commit().await.map_err(CoreError::Database)?;
    Ok(tool_uses.len())
}
```

### 2.4 Performance Considerations

**Storage:** ~190K tool uses × ~120 bytes/row = ~23MB. Negligible for SQLite.

**Insert speed:** 190K rows in transactions of ~50-500 per session. SQLite handles this trivially within a transaction. Benchmark target: <50ms per session.

**Incremental sync:** The existing `session_needs_update()` check (jsonl_parser.rs:896) skips unchanged sessions. Only new/modified sessions get tool uses persisted. The `DELETE + INSERT` pattern ensures idempotent re-sync.

### 2.5 Tests

```rust
#[tokio::test]
async fn test_insert_file_access_events_basic() {
    // Create DB with schema → insert 3 tool uses → query back → verify order
}

#[tokio::test]
async fn test_insert_file_access_events_idempotent() {
    // Insert 3 events → insert again (same session) → still 3 rows (DELETE + INSERT)
}

#[tokio::test]
async fn test_insert_file_access_events_batch_performance() {
    // Insert 500 events for a session → should complete in <50ms
}

#[tokio::test]
async fn test_sync_persists_file_access_events() {
    // Integration: run sync_sessions_phase with a test JSONL → verify events in DB
}
```

---

## Phase 3: Edge Extraction Module

**Complexity:** Medium-High
**Files:** New `core/src/index/file_edges.rs`, `core/src/index/mod.rs`, `core/src/daemon/sync.rs`
**Depends on:** Phase 2 (events must be in DB)

### 3.1 Module Structure

New file: `core/src/index/file_edges.rs`

```rust
//! Temporal edge extraction from file access events.
//!
//! Implements deterministic edge type rules applied to per-session
//! tool call sequences. Runs as batch job during daemon sync.

pub struct EdgeExtractionResult {
    pub edges_created: usize,
    pub sessions_processed: usize,
    pub duration_ms: u64,
}
```

Register in `core/src/index/mod.rs`:
```rust
pub mod file_edges;
pub use file_edges::*;
```

### 3.2 Edge Types

Five deterministic edge types, extracted from ordered tool call sequences:

| Edge Type | Rule | Direction | Example |
|-----------|------|-----------|---------|
| `read_then_edit` | File A read, then file B edited in same session, within 5 min | A → B | Read types.rs → Edit query.rs |
| `read_before` | File A read before file B read in >50% of shared sessions, within 5 min | A → B | CLAUDE.md → _synthesis/summary.md |
| `co_edited` | Both files edited in same session | A ↔ B (stored as A→B where A < B) | query.rs ↔ types.rs |
| `reference_anchor` | File read in first 2 min of >3 sessions | File → (self-edge) | CLAUDE.md |
| `debug_chain` | File read after a Bash tool call in same session | Bash → File | (error output) → storage.rs |

### 3.3 Extraction Algorithm

```rust
/// Extract edges from file_access_events for sessions since last extraction.
pub async fn extract_file_edges(
    pool: &SqlitePool,
    since: Option<&str>,  // Last extraction timestamp
) -> Result<EdgeExtractionResult, CoreError> {
    let start = Instant::now();

    // 1. Get sessions to process (new since last extraction)
    let sessions = get_sessions_since(pool, since).await?;

    // 2. For each session, load ordered events
    //    and extract per-session edge candidates
    let mut all_candidates: Vec<EdgeCandidate> = Vec::new();
    for session_id in &sessions {
        let events = load_session_events(pool, session_id).await?;
        let filtered = filter_explore_bursts(&events);
        let candidates = extract_session_edges(&filtered);
        all_candidates.extend(candidates);
    }

    // 3. Aggregate candidates across sessions → final edges
    let edges = aggregate_edge_candidates(&all_candidates);

    // 4. Apply noise filters
    let filtered_edges = apply_noise_filters(pool, &edges).await?;

    // 5. Upsert to file_edges table
    let count = upsert_edges(pool, &filtered_edges).await?;

    Ok(EdgeExtractionResult {
        edges_created: count,
        sessions_processed: sessions.len(),
        duration_ms: start.elapsed().as_millis() as u64,
    })
}
```

### 3.4 Noise Filtering

Three deterministic filters (from package #38 design):

**Filter 1: Explore Burst Detection**
```rust
/// Detect explore agent bursts: >5 reads in 30 seconds with no edits.
/// Tag these events as `is_explore_burst = true` and exclude from edge extraction.
fn filter_explore_bursts(events: &[FileAccessEvent]) -> Vec<&FileAccessEvent> {
    // Sliding window: if >5 consecutive reads span <30s with no writes, mark as burst
}
```

Empirical basis: Explore bursts are 1-9% of tool calls (package #39). Simple velocity filter suffices.

**Filter 2: Universal Anchor Dampening**
```rust
/// Files appearing as source in >80% of all sessions are "universal anchors."
/// Don't create read_before edges FROM these files (too noisy).
/// Do mark them as reference_anchor edge type.
async fn get_universal_anchors(pool: &SqlitePool) -> HashSet<String> {
    // SELECT file_path, COUNT(DISTINCT session_id) as session_count
    // FROM file_access_events WHERE access_type = 'read'
    // GROUP BY file_path
    // HAVING session_count > (SELECT COUNT(DISTINCT session_id) * 0.8 FROM file_access_events)
}
```

**Filter 3: Minimum Session Count**
```rust
/// Edges must appear in >= 3 sessions to be stored.
/// Coincidences happen once. Patterns repeat.
const MIN_SESSION_COUNT: i32 = 3;
```

### 3.5 Integration with Daemon Sync

Add new phase to `run_sync()` (daemon/sync.rs:51-116), between chain building (step 3) and index update (step 4):

```rust
// 3.5b: Temporal edge extraction (after chain building)
if let Some(ref engine) = engine {
    match extract_file_edges(engine.database().pool(), None).await {
        Ok(edge_result) => {
            debug!(
                target: "daemon.sync",
                "Extracted {} edges from {} sessions",
                edge_result.edges_created, edge_result.sessions_processed
            );
        }
        Err(e) => {
            result.errors.push(format!("Edge extraction: {}", e));
        }
    }
}
```

### 3.6 Incremental Extraction

Track last extraction time in `_metadata`:

```sql
INSERT OR REPLACE INTO _metadata (key, value) VALUES ('last_edge_extraction', '2026-02-17T...');
```

Only process sessions with `parsed_at > last_edge_extraction`. Full rebuild available via `tastematter build-edges --full`.

### 3.7 Tests

```rust
// Unit tests (no DB)
#[test]
fn test_filter_explore_bursts_removes_rapid_reads() {
    // 8 reads in 10 seconds → all filtered
}

#[test]
fn test_filter_explore_bursts_preserves_normal_reads() {
    // 3 reads over 2 minutes → all preserved
}

#[test]
fn test_extract_session_edges_finds_read_then_edit() {
    // R(A) → R(B) → W(C) → sequence contains read_then_edit(A→C) and read_then_edit(B→C)
}

#[test]
fn test_extract_session_edges_finds_co_edited() {
    // W(A) → W(B) → co_edited(A, B)
}

#[test]
fn test_aggregate_edge_candidates_counts_sessions() {
    // Same edge from 3 sessions → session_count = 3
}

#[test]
fn test_noise_filter_min_session_count() {
    // Edge in 2 sessions → filtered out (< MIN_SESSION_COUNT)
}

// Integration tests (with DB)
#[tokio::test]
async fn test_extract_file_edges_end_to_end() {
    // Insert events for 5 sessions → extract → verify edges in DB
}

#[tokio::test]
async fn test_extract_file_edges_incremental() {
    // Extract once → add more events → extract again → only new sessions processed
}

#[tokio::test]
async fn test_universal_anchor_dampening() {
    // File in 90% of sessions → no read_before edges FROM it, but IS reference_anchor
}
```

---

## Phase 4: Context Restore Integration

**Complexity:** Medium
**Files:** `core/src/types.rs`, `core/src/query.rs`, `core/src/context_restore.rs`
**Depends on:** Phase 3 (edges must be in DB)

### 4.1 New Types (types.rs)

```rust
/// A directed behavioral edge between two files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEdge {
    pub source_file: String,
    pub target_file: String,
    pub edge_type: String,
    pub session_count: i32,
    pub confidence: f64,
}

/// Work pattern derived from temporal edges.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkPattern {
    /// Files consistently at the start of work sequences
    pub entry_points: Vec<String>,
    /// Files consistently at the end (edited after reads)
    pub work_targets: Vec<String>,
    /// Topological sort of read_before chain
    #[serde(skip_serializing_if = "Option::is_none")]
    pub typical_sequence: Option<Vec<String>>,
    /// Comparison of last session against typical pattern
    #[serde(skip_serializing_if = "Option::is_none")]
    pub incomplete_sequence: Option<IncompleteSequence>,
}

/// Detected incomplete work sequence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncompleteSequence {
    /// What was observed in last session
    pub observed: Vec<String>,
    /// What typically comes next
    pub typical_next: String,
    /// How confident is the prediction
    pub confidence: f64,
}
```

### 4.2 Enhance Existing Types

Add to `WorkCluster` (types.rs:738-749):

```rust
pub struct WorkCluster {
    pub name: Option<String>,
    pub files: Vec<String>,
    pub pmi_score: f64,
    pub interpretation: Option<String>,
    pub access_pattern: String,
    // NEW: temporal edge enrichment
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_pattern: Option<WorkPattern>,
}
```

Add to `Continuity` (types.rs:709-715):

```rust
pub struct Continuity {
    pub left_off_at: Option<LeftOffAt>,
    pub pending_items: Vec<PendingItem>,
    pub chain_context: Option<ChainContext>,
    // NEW: detected incomplete work sequence
    #[serde(skip_serializing_if = "Option::is_none")]
    pub incomplete_sequence: Option<IncompleteSequence>,
}
```

### 4.3 Edge Query (query.rs)

Add new query method to `QueryEngine`:

```rust
/// Query file edges matching a pattern, filtered by minimum confidence.
pub async fn query_file_edges(
    &self,
    pattern: &str,
    min_session_count: i32,
    min_confidence: f64,
    limit: u32,
) -> Result<Vec<FileEdge>, CoreError> {
    let rows: Vec<FileEdge> = sqlx::query_as(
        r#"SELECT source_file, target_file, edge_type, session_count, confidence
           FROM file_edges
           WHERE (source_file LIKE ? OR target_file LIKE ?)
             AND session_count >= ?
             AND confidence >= ?
           ORDER BY session_count DESC
           LIMIT ?"#,
    )
    .bind(pattern)
    .bind(pattern)
    .bind(min_session_count)
    .bind(min_confidence)
    .bind(limit)
    .fetch_all(self.database().pool())
    .await
    .map_err(CoreError::Database)?;

    Ok(rows)
}
```

### 4.4 Pipeline Integration (query.rs)

Modify `query_context()` (query.rs:1350-1454):

**Phase 2 enhancement** — add edge query alongside co-access:

```rust
// Phase 2: Sequential co-access for top 5 hot files + edge query
let anchors: Vec<String> = flex.results.iter().take(5).map(|f| f.file_path.clone()).collect();
let mut co_access_results = Vec::new();
for anchor in &anchors {
    if let Ok(co) = self.query_co_access(QueryCoAccessInput {
        file_path: anchor.clone(),
        limit: Some(10),
    }).await {
        co_access_results.push(co);
    }
}

// NEW: Query temporal edges for matching files
let edges = self.query_file_edges(
    &pattern,
    3,    // min_session_count
    0.3,  // min_confidence
    50,   // limit
).await.unwrap_or_default();
```

**Phase 4 enhancement** — pass edges to builder:

```rust
work_clusters: crate::context_restore::build_work_clusters(
    &flex,
    &co_access_results,
    &edges,  // NEW: pass edges for pattern enrichment
),
```

### 4.5 Builder Function (context_restore.rs)

New builder: `build_work_patterns()` — called within `build_work_clusters()`:

```rust
/// Extract work patterns from temporal edges for a set of cluster files.
pub fn build_work_patterns(
    cluster_files: &[String],
    edges: &[FileEdge],
) -> Option<WorkPattern> {
    let file_set: HashSet<&str> = cluster_files.iter().map(|s| s.as_str()).collect();

    // 1. Entry points: files that are source in read_before/read_then_edit but rarely target
    let entry_points: Vec<String> = edges.iter()
        .filter(|e| e.edge_type == "read_before" || e.edge_type == "read_then_edit")
        .filter(|e| file_set.contains(e.source_file.as_str()))
        .map(|e| e.source_file.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    // 2. Work targets: files that are target in read_then_edit edges
    let work_targets: Vec<String> = edges.iter()
        .filter(|e| e.edge_type == "read_then_edit")
        .filter(|e| file_set.contains(e.target_file.as_str()))
        .map(|e| e.target_file.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    // 3. Typical sequence: topological sort of read_before within cluster
    let typical_sequence = topological_sort_edges(
        edges.iter()
            .filter(|e| e.edge_type == "read_before")
            .filter(|e| file_set.contains(e.source_file.as_str())
                     && file_set.contains(e.target_file.as_str()))
            .collect::<Vec<_>>()
    );

    if entry_points.is_empty() && work_targets.is_empty() {
        return None;
    }

    Some(WorkPattern {
        entry_points,
        work_targets,
        typical_sequence,
        incomplete_sequence: None,  // Computed separately with last session data
    })
}
```

### 4.6 Output Enhancement

The enhanced `WorkCluster` output adds ~50-100 tokens per cluster:

```json
{
  "work_clusters": [
    {
      "name": "Rust Core Development",
      "files": ["types.rs", "storage.rs", "query.rs", "main.rs"],
      "pmi_score": 0.55,
      "access_pattern": "high_access_high_session",
      "work_pattern": {
        "entry_points": ["types.rs"],
        "work_targets": ["query.rs"],
        "typical_sequence": ["types.rs", "storage.rs", "query.rs", "main.rs"],
        "incomplete_sequence": null
      }
    }
  ],
  "continuity": {
    "left_off_at": { "file": "storage.rs" },
    "incomplete_sequence": {
      "observed": ["types.rs", "storage.rs", "query.rs"],
      "typical_next": "main.rs (4/5 prior completions)",
      "confidence": 0.80
    }
  }
}
```

### 4.7 Tests

```rust
// Unit tests (no DB)
#[test]
fn test_build_work_patterns_finds_entry_points() {
    // read_before edges: A→B, A→C → entry_point = [A]
}

#[test]
fn test_build_work_patterns_finds_work_targets() {
    // read_then_edit edges: A→C, B→C → work_target = [C]
}

#[test]
fn test_build_work_patterns_topological_sort() {
    // read_before: A→B, B→C → typical_sequence = [A, B, C]
}

#[test]
fn test_build_work_patterns_returns_none_when_no_edges() {
    // Empty edges → None
}

#[test]
fn test_build_work_patterns_handles_cycles() {
    // A→B, B→A → graceful handling (break cycle, return partial sequence)
}

// Integration test
#[tokio::test]
async fn test_query_context_includes_work_patterns() {
    // Full pipeline: insert events → extract edges → query_context → verify work_pattern in output
}
```

---

## Backward Compatibility

### API Surface

All new fields use `#[serde(skip_serializing_if = "Option::is_none")]`:
- `WorkCluster.work_pattern: Option<WorkPattern>` — absent when no temporal data
- `Continuity.incomplete_sequence: Option<IncompleteSequence>` — absent when no pattern detected

Existing consumers see identical JSON when temporal tables are empty. **Zero breaking changes.**

### Database

- `CREATE TABLE IF NOT EXISTS` — tables only created on fresh/upgraded DBs
- Existing tables and data untouched
- Schema version bump 2.2 → 2.3 tracked in `_metadata`

### Daemon Sync

- Edge extraction is a new phase, non-blocking
- If it fails, other sync phases continue (same graceful degradation as intel enrichment)
- Event persistence runs alongside session upsert — failure logged but doesn't block session persistence

---

## Operational Concerns

### Storage Impact

| Table | Est. Rows | Est. Size | Growth Rate |
|-------|-----------|-----------|-------------|
| file_access_events | ~190K (current) | ~23MB | ~500/day |
| file_edges | ~10K-50K | ~5MB | Rebuild weekly |

SQLite handles millions of rows. No concern.

### Performance Budget

| Operation | Target | When |
|-----------|--------|------|
| Event insert (per session) | <50ms | During sync, per session |
| Edge extraction (full) | <5s | First run only |
| Edge extraction (incremental) | <500ms | During sync |
| Edge query (per context call) | <10ms | Per `query_context()` |
| Pattern computation | <1ms | Per `build_work_patterns()` |

### Monitoring

- `SyncResult` gains `edges_extracted: i32` field
- `daemon status` shows last edge extraction timestamp
- `_metadata` stores `last_edge_extraction` for incremental tracking

---

## Implementation Sequence

| Phase | What | Files | Complexity | Tests |
|-------|------|-------|------------|-------|
| 1 | Schema migration | storage.rs | Low | 4 |
| 2 | Event persistence | jsonl_parser.rs, sync.rs, query.rs | Medium | 4 |
| 3 | Edge extraction | NEW file_edges.rs, mod.rs, sync.rs | Medium-High | 10+ |
| 4 | Context restore integration | types.rs, query.rs, context_restore.rs | Medium | 6+ |

**Total estimated tests:** ~24 new tests across 4 phases.

**Phase boundaries are clean:** Each phase is independently shippable and testable. Phase 1 changes no behavior. Phase 2 writes data but doesn't read it yet. Phase 3 reads events and writes edges. Phase 4 reads edges and enhances output.

---

## Open Questions

1. **Should `debug_chain` edge type be deferred?** It requires Bash tool call detection, which is noisier than Read/Edit. The other 4 edge types provide the core value. Recommendation: defer to Phase 5.

2. **Should edge extraction be a separate CLI command?** Currently planned as part of daemon sync. Could also be `tastematter build-edges` for on-demand rebuild. Recommendation: both — sync runs incremental, CLI runs full rebuild.

3. **Should `file_access_events` store Bash tool calls?** Current design excludes them (too noisy, no clear file_path). But debug_chain needs them later. Recommendation: store with `access_type = 'command'` but exclude from edge extraction until Phase 5.

---

## Verification Checklist

After each phase, verify:

- [ ] `cargo check` passes
- [ ] `cargo test -- --test-threads=2` passes (ALL existing + new tests)
- [ ] No regressions in existing `tastematter context` output
- [ ] New tables visible via `sqlite3 ~/.context-os/context_os_events.db ".tables"`
- [ ] Schema version updated in `_metadata`

After Phase 4 complete:

- [ ] `tastematter context "tastematter"` includes `work_pattern` in clusters
- [ ] `tastematter context "tastematter"` includes `incomplete_sequence` in continuity (when applicable)
- [ ] Output JSON is backward-compatible (Optional fields absent when empty)
- [ ] Performance: `tastematter context` completes in <3s (current ~1-2s, target <3s with edges)
