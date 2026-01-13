# Phase 1: Core Foundation - Agent Task Specification

## Executive Summary

**Mission:** Create the `context-os-core` Rust library with direct SQLite queries that replace the Python CLI's 18-second query latency with <100ms responses.

**Why This Matters:** This is the highest-leverage phase. Every subsequent phase depends on this working. Success here proves the architecture is sound and unblocks all parallel work.

**Success Definition:** `cargo test` passes with all queries returning correct results in <100ms against the real database.

---

## Prerequisites

### Required Reading (In Order)

1. **This spec** (you're reading it)
2. **Architecture decisions:** `specs/canonical/03_CORE_ARCHITECTURE.md`
   - Focus on: Design Decisions 1-2 (No Redis, Direct SQL)
3. **Type contracts:** `phase_01_core_foundation/CONTRACTS.rs` (this folder)
   - These types MUST match exactly
4. **SQLite schema:** `apps/context_os_events/specs/context_os_intelligence/02_INDEX_STRUCTURES.md`
   - Focus on: Table schemas and indexes
5. **Current bottleneck:** `apps/tastematter/src-tauri/src/commands.rs:88-155`
   - Understand what we're replacing

### Environment Setup

```bash
# Verify Rust toolchain
rustc --version  # Should be 1.77+

# Verify database exists
ls -la ~/.context-os/context.db  # Or apps/context_os_events/data/context_os_events.db

# The database should be ~1.8MB with existing data
```

---

## Architecture Context

### The Problem We're Solving

```
Current Flow (18 seconds):
  Tauri Command
      ↓
  Command::new("context-os.cmd")  [500ms - process spawn]
      ↓
  Python CLI starts               [2-3s - imports]
      ↓
  ContextIndex.load()             [15s - loads ENTIRE DB into memory]
      ↓
  QueryEngine.execute()           [50ms - actual query]
      ↓
  Return JSON

Target Flow (<100ms):
  Tauri Command
      ↓
  context_os_core::query_flex()   [<100ms - direct SQL]
      ↓
  Return QueryResult
```

### What We're Building

```
apps/context-os-core/
├── Cargo.toml
└── src/
    ├── lib.rs          # Public API exports
    ├── error.rs        # Error types (CoreError)
    ├── types.rs        # Data types matching Tauri commands
    ├── storage.rs      # SQLite connection management
    └── query.rs        # Query functions with direct SQL
```

---

## Type Contracts

**CRITICAL:** These types must serialize to the EXACT same JSON as the current Tauri commands.

See `CONTRACTS.rs` in this folder for the complete type definitions. Key types:

### Query Input/Output

```rust
// Input to query_flex
pub struct QueryFlexInput {
    pub files: Option<String>,      // File path pattern filter
    pub time: Option<String>,       // "7d", "14d", "30d"
    pub chain: Option<String>,      // Chain ID filter
    pub session: Option<String>,    // Session ID filter
    pub agg: Vec<String>,           // Aggregations: "count", "recency"
    pub limit: Option<u32>,         // Result limit (default 20)
    pub sort: Option<String>,       // "count" or "recency"
}

// Output from query_flex
pub struct QueryResult {
    pub receipt_id: String,
    pub timestamp: String,
    pub result_count: usize,
    pub results: Vec<FileResult>,
    pub aggregations: Aggregations,
}
```

### Database Schema (Read-Only)

The Rust code reads from these tables (Python daemon writes):

```sql
-- Primary query table
CREATE TABLE file_accesses (
    id INTEGER PRIMARY KEY,
    file_path TEXT NOT NULL,
    session_id TEXT NOT NULL,
    chain_id TEXT,
    access_type TEXT NOT NULL,   -- 'read', 'write', 'create'
    tool_name TEXT NOT NULL,     -- 'Read', 'Edit', 'Write', 'Grep'
    timestamp TEXT NOT NULL
);

-- Chain metadata
CREATE TABLE chains (
    chain_id TEXT PRIMARY KEY,
    root_session_id TEXT NOT NULL,
    session_count INTEGER,
    files_json TEXT,             -- JSON array of file paths
    files_bloom BLOB             -- Bloom filter for O(1) membership
);

-- Session metadata
CREATE TABLE claude_sessions (
    session_id TEXT PRIMARY KEY,
    started_at TEXT,
    total_messages INTEGER
);
```

---

## Implementation Steps

### Step 1: Create Crate Structure (15 min)

```bash
# Create the crate
mkdir -p apps/context-os-core/src
cd apps/context-os-core
```

Create `Cargo.toml`:
```toml
[package]
name = "context-os-core"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["rlib"]

[dependencies]
# SQLite - async with compile-time checked queries
sqlx = { version = "0.8", features = [
    "runtime-tokio",
    "sqlite",
    "json",
    "chrono",
] }

# Async runtime
tokio = { version = "1.40", features = ["rt", "sync"] }

# Serialization (must match Tauri)
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Error handling
thiserror = "1.0"
anyhow = "1.0"

# Time handling
chrono = { version = "0.4", features = ["serde"] }

# Logging
log = "0.4"

# UUID for receipt_id
uuid = { version = "1.0", features = ["v4"] }

[dev-dependencies]
tokio = { version = "1.40", features = ["full", "test-util"] }
```

Create `src/lib.rs`:
```rust
//! context-os-core: Unified query engine for Tastematter
//!
//! Provides direct SQLite queries replacing the Python CLI bottleneck.

pub mod error;
pub mod query;
pub mod storage;
pub mod types;

pub use error::CoreError;
pub use query::QueryEngine;
pub use storage::Database;
pub use types::*;
```

### Step 2: Implement Error Types (10 min)

Create `src/error.rs` - See CONTRACTS.rs for exact definition.

Key requirements:
- Must convert to `CommandError` format for Tauri compatibility
- Include error codes: DATABASE_ERROR, QUERY_ERROR, CONFIG_ERROR

### Step 3: Implement Types (20 min)

Create `src/types.rs` - Copy EXACTLY from CONTRACTS.rs.

**CRITICAL VERIFICATION:**
```rust
// These must serialize identically to current Tauri output
let result = QueryResult { ... };
let json = serde_json::to_string(&result)?;
// JSON structure must match what frontend expects
```

### Step 4: Implement Storage Layer (30 min)

Create `src/storage.rs`:

```rust
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use std::path::Path;

pub struct Database {
    pool: SqlitePool,
}

impl Database {
    /// Open database with connection pooling
    pub async fn open(path: impl AsRef<Path>) -> Result<Self, CoreError> {
        let url = format!("sqlite:{}?mode=ro", path.as_ref().display());

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&url)
            .await?;

        Ok(Self { pool })
    }

    /// Get a reference to the connection pool
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}
```

**Key design decisions:**
- `mode=ro` - Read-only (Python daemon writes)
- Connection pooling for concurrent queries
- WAL mode compatibility (daemon uses WAL)

### Step 5: Implement Query Engine (2 hours)

Create `src/query.rs`:

This is the core of Phase 1. Implement 4 query functions:

#### 5a. query_flex (most important)

```rust
impl QueryEngine {
    pub async fn query_flex(&self, input: QueryFlexInput) -> Result<QueryResult, CoreError> {
        let start = std::time::Instant::now();

        // Build dynamic SQL based on filters
        let mut sql = String::from(
            "SELECT file_path,
                    COUNT(*) as access_count,
                    MAX(timestamp) as last_access,
                    COUNT(DISTINCT session_id) as session_count
             FROM file_accesses
             WHERE 1=1"
        );

        // Add time filter
        if let Some(time) = &input.time {
            let days = parse_time_range(time)?;
            sql.push_str(&format!(
                " AND timestamp >= datetime('now', '-{} days')", days
            ));
        }

        // Add chain filter
        if let Some(chain) = &input.chain {
            sql.push_str(&format!(" AND chain_id = '{}'", chain));
        }

        // Group and order
        sql.push_str(" GROUP BY file_path");

        match input.sort.as_deref() {
            Some("recency") => sql.push_str(" ORDER BY last_access DESC"),
            _ => sql.push_str(" ORDER BY access_count DESC"),
        }

        // Limit
        let limit = input.limit.unwrap_or(20);
        sql.push_str(&format!(" LIMIT {}", limit));

        // Execute query
        let rows = sqlx::query(&sql)
            .fetch_all(self.db.pool())
            .await?;

        // Transform to FileResult
        let results: Vec<FileResult> = rows.iter().map(|row| {
            FileResult {
                file_path: row.get("file_path"),
                access_count: row.get::<i64, _>("access_count") as u32,
                last_access: row.get("last_access"),
                session_count: Some(row.get::<i64, _>("session_count") as u32),
                sessions: None,
                chains: None,
            }
        }).collect();

        // Build aggregations
        let aggregations = self.compute_aggregations(&results, &input.agg)?;

        let elapsed = start.elapsed();
        log::info!("query_flex completed in {:?}", elapsed);

        Ok(QueryResult {
            receipt_id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            result_count: results.len(),
            results,
            aggregations,
        })
    }
}
```

#### 5b. query_chains

```rust
pub async fn query_chains(&self, limit: Option<u32>) -> Result<ChainQueryResult, CoreError> {
    let limit = limit.unwrap_or(20);

    let sql = format!(
        "SELECT chain_id, root_session_id, session_count, files_json
         FROM chains
         ORDER BY session_count DESC
         LIMIT {}", limit
    );

    let rows = sqlx::query(&sql)
        .fetch_all(self.db.pool())
        .await?;

    let chains: Vec<ChainData> = rows.iter().map(|row| {
        let files_json: Option<String> = row.get("files_json");
        let file_count = files_json
            .as_ref()
            .and_then(|j| serde_json::from_str::<Vec<String>>(j).ok())
            .map(|v| v.len() as u32)
            .unwrap_or(0);

        ChainData {
            chain_id: row.get("chain_id"),
            session_count: row.get::<i64, _>("session_count") as u32,
            file_count,
            time_range: None, // TODO: Add if needed
        }
    }).collect();

    let total_chains = chains.len() as u32;

    Ok(ChainQueryResult { chains, total_chains })
}
```

#### 5c. query_timeline

Transform query_flex results into timeline buckets.

#### 5d. query_sessions

Group file results by session.

### Step 6: Write Tests (1 hour)

See `TESTS.md` for the complete TDD test plan. Key tests:

```rust
#[tokio::test]
async fn test_query_flex_returns_files() {
    let db = Database::open("test_fixtures/test.db").await.unwrap();
    let engine = QueryEngine::new(db);

    let result = engine.query_flex(QueryFlexInput::default()).await.unwrap();

    assert!(result.result_count > 0);
    assert!(!result.results.is_empty());
}

#[tokio::test]
async fn test_query_flex_time_filter() {
    // ... tests time filtering works
}

#[tokio::test]
async fn test_query_latency_under_100ms() {
    let start = Instant::now();
    let result = engine.query_flex(input).await.unwrap();
    assert!(start.elapsed() < Duration::from_millis(100));
}
```

### Step 7: Verify Against Real Database (30 min)

```bash
# Run tests against the real database
CONTEXT_OS_DB=~/.context-os/context.db cargo test

# Run benchmark
cargo run --example benchmark
```

---

## Success Criteria

- [ ] `cargo build` succeeds with no warnings
- [ ] `cargo test` passes all unit tests
- [ ] `cargo test` passes all integration tests with real DB
- [ ] `query_flex` returns correct results matching CLI output
- [ ] `query_chains` returns correct results matching CLI output
- [ ] All queries complete in <100ms (measured)
- [ ] Types serialize to same JSON as current Tauri commands

---

## Common Pitfalls

### Pitfall 1: Type Mismatch with Frontend

**Problem:** Rust types serialize differently than current Python CLI output.

**Prevention:**
- Copy types EXACTLY from CONTRACTS.rs
- Use `#[serde(skip_serializing_if = "Option::is_none")]` for optional fields
- Test JSON output against current CLI output

**Verification:**
```bash
# Get current CLI output
context-os query flex --time 7d --limit 5 --format json > expected.json

# Get Rust output
cargo run --example query_flex > actual.json

# Compare
diff expected.json actual.json
```

### Pitfall 2: Database Path Issues

**Problem:** Different paths on different machines.

**Prevention:**
- Use config-driven paths
- Check multiple standard locations
- Clear error messages when DB not found

```rust
fn find_database() -> Result<PathBuf, CoreError> {
    let candidates = [
        dirs::home_dir().map(|h| h.join(".context-os/context.db")),
        Some(PathBuf::from("apps/context_os_events/data/context_os_events.db")),
    ];

    for path in candidates.into_iter().flatten() {
        if path.exists() {
            return Ok(path);
        }
    }

    Err(CoreError::Config("Database not found".into()))
}
```

### Pitfall 3: SQL Injection via String Interpolation

**Problem:** Building SQL with format! is unsafe.

**Prevention:** Use parameterized queries where possible:

```rust
// UNSAFE
let sql = format!("SELECT * FROM files WHERE chain_id = '{}'", chain_id);

// SAFE
sqlx::query("SELECT * FROM files WHERE chain_id = ?")
    .bind(&chain_id)
    .fetch_all(pool)
    .await
```

**Note:** Some dynamic parts (ORDER BY, LIMIT) need string building. Validate inputs.

### Pitfall 4: Async Runtime Mismatch

**Problem:** sqlx needs tokio runtime, but tests may not have it.

**Prevention:**
```rust
#[tokio::test]  // Not #[test]
async fn test_query() {
    // ...
}
```

---

## Integration Notes

### For Phase 2 (Tauri Integration)

Phase 2 will:
1. Add `context-os-core` as a dependency to Tastematter
2. Replace Command::new() calls with direct library calls
3. Pass the Database instance via AppState

**Contract for Phase 2:**
```rust
// Phase 2 will call:
let engine = QueryEngine::new(database);
let result = engine.query_flex(input).await?;
```

### Database Location

Phase 2 will pass the database path. Phase 1 should support both:
1. Path passed explicitly
2. Auto-discovery from standard locations

---

## Handoff Checklist

Before completing Phase 1, verify:

- [ ] All tests pass
- [ ] Latency < 100ms verified
- [ ] JSON output matches current CLI
- [ ] Code compiles without warnings
- [ ] Public API is documented
- [ ] COMPLETION_REPORT.md written

---

## File Listing

After Phase 1 completion:

```
apps/context-os-core/
├── Cargo.toml
├── src/
│   ├── lib.rs          (~30 lines)
│   ├── error.rs        (~50 lines)
│   ├── types.rs        (~150 lines)
│   ├── storage.rs      (~50 lines)
│   └── query.rs        (~300 lines)
└── tests/
    ├── integration.rs  (~100 lines)
    └── fixtures/
        └── test.db     (test database)
```

**Total:** ~680 lines of Rust code

---

**Spec Version:** 1.0
**Created:** 2026-01-08
**Phase:** 1 of 8
**Estimated Time:** 3-4 hours
