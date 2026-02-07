# Schema Unification Specification

**Status:** Proposed
**Priority:** High
**Bug IDs:** XCHECK-1 (schema conflict), BUG-09 (chain_graph divergence), XCHECK-4 (chain_summaries fragility)
**Estimated Effort:** 2-3 hours

---

## Problem Statement

The `chain_metadata` table has **two incompatible schema definitions** in the codebase. Both use `CREATE TABLE IF NOT EXISTS`, so whichever module initializes first "wins" and the other module's columns silently don't exist.

### Definition 1: `storage.rs:207-214` (`ensure_schema()`)

```sql
CREATE TABLE IF NOT EXISTS chain_metadata (
    chain_id TEXT PRIMARY KEY,
    generated_name TEXT,
    summary TEXT,              -- ONLY in storage.rs
    key_topics TEXT,           -- ONLY in storage.rs
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP  -- ONLY in storage.rs
);
```

### Definition 2: `intelligence/cache.rs:403-411` (`MIGRATION_SQL`)

```sql
CREATE TABLE IF NOT EXISTS chain_metadata (
    chain_id TEXT PRIMARY KEY,
    generated_name TEXT,
    category TEXT,             -- ONLY in cache.rs
    confidence REAL,           -- ONLY in cache.rs
    generated_at TEXT,         -- ONLY in cache.rs
    model_used TEXT,           -- ONLY in cache.rs
    created_at TEXT DEFAULT (datetime('now'))
    -- NO updated_at, NO summary, NO key_topics
);
```

### What Actually Happens at Runtime

`sync.rs` calls `db.ensure_schema()` first (line 67), then `MetadataStore::new()` later (line 275). Since storage.rs runs first, it creates the table with `summary`, `key_topics`, `updated_at`. When cache.rs runs, `IF NOT EXISTS` is a no-op — the table already exists with the storage.rs schema.

The intelligence cache then attempts to INSERT into columns that don't exist:

```rust
// cache.rs:58-60
INSERT OR REPLACE INTO chain_metadata
(chain_id, generated_name, category, confidence, generated_at, model_used, created_at)
VALUES (?, ?, ?, ?, ?, ?, ?)
```

The columns `category`, `confidence`, `generated_at`, and `model_used` do not exist in the actual table, causing the INSERT to fail. Depending on SQLite/sqlx error handling, this either throws a runtime error or silently fails.

### Secondary Issue: `chain_graph` schema divergence (BUG-09)

`ensure_schema()` creates `chain_graph` with only 2 columns (`storage.rs:200-203`):

```sql
CREATE TABLE IF NOT EXISTS chain_graph (
    session_id TEXT PRIMARY KEY,
    chain_id TEXT NOT NULL
);
```

But `persist_chains()` DROP+recreates `chain_graph` with 5 columns (`query.rs:1450-1457`):

```sql
CREATE TABLE chain_graph (
    session_id TEXT PRIMARY KEY,
    chain_id TEXT,
    parent_session_id TEXT,
    is_root BOOLEAN,
    indexed_at TEXT
);
```

Since `persist_chains()` always runs after `ensure_schema()` and uses DROP+recreate (not `IF NOT EXISTS`), the final schema always has the 5-column version. The `ensure_schema()` definition is misleading dead code for this table.

### Tertiary Issue: `chain_summaries` only in cache.rs (XCHECK-4)

The `chain_summaries` table (`cache.rs:454-463`) is only created by the intelligence cache migration. If `MetadataStore::new()` never runs (e.g., Intel service not configured), queries against `chain_summaries` will fail because the table doesn't exist.

---

## Root Cause

Two modules (`storage.rs` and `intelligence/cache.rs`) independently define the same table (`chain_metadata`) with different column sets. Neither module is aware of the other's schema expectations. There is no single source of truth for the database schema.

---

## Canonical Schema

### Unified `chain_metadata` table

Merge ALL columns from both definitions:

```sql
CREATE TABLE IF NOT EXISTS chain_metadata (
    chain_id TEXT PRIMARY KEY,
    generated_name TEXT,
    -- From storage.rs
    summary TEXT,
    key_topics TEXT,
    -- From cache.rs
    category TEXT,
    confidence REAL,
    generated_at TEXT,
    model_used TEXT,
    -- Timestamps
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP
);
```

### Unified `chain_graph` table

Match the schema that `persist_chains()` actually creates:

```sql
CREATE TABLE IF NOT EXISTS chain_graph (
    session_id TEXT PRIMARY KEY,
    chain_id TEXT NOT NULL,
    parent_session_id TEXT,
    is_root BOOLEAN,
    indexed_at TEXT
);
CREATE INDEX IF NOT EXISTS idx_chain_graph_chain ON chain_graph(chain_id);
```

### `chain_summaries` table (moved to storage.rs)

```sql
CREATE TABLE IF NOT EXISTS chain_summaries (
    chain_id TEXT PRIMARY KEY,
    summary TEXT,
    accomplishments TEXT,
    status TEXT,
    key_files TEXT,
    workstream_tags TEXT,
    model_used TEXT,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP
);
```

---

## Implementation Steps

### Step 1: Update `ensure_schema()` in `storage.rs`

**File:** `core/src/storage.rs`
**Lines:** 190-214 (chain_graph + chain_metadata definitions)

Replace the current `chain_graph` and `chain_metadata` CREATE TABLE statements with the unified versions. Add `chain_summaries`.

**Before (lines 200-214):**
```sql
-- Layer 5: Chain Graph (session-to-chain mapping)
CREATE TABLE IF NOT EXISTS chain_graph (
    session_id TEXT PRIMARY KEY,
    chain_id TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_chain_graph_chain ON chain_graph(chain_id);

-- Layer 6: Chain Metadata (Intel-generated names and summaries)
CREATE TABLE IF NOT EXISTS chain_metadata (
    chain_id TEXT PRIMARY KEY,
    generated_name TEXT,
    summary TEXT,
    key_topics TEXT,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP
);
```

**After:**
```sql
-- Layer 5: Chain Graph (session-to-chain mapping)
CREATE TABLE IF NOT EXISTS chain_graph (
    session_id TEXT PRIMARY KEY,
    chain_id TEXT NOT NULL,
    parent_session_id TEXT,
    is_root BOOLEAN,
    indexed_at TEXT
);
CREATE INDEX IF NOT EXISTS idx_chain_graph_chain ON chain_graph(chain_id);

-- Layer 6: Chain Metadata (Intel-generated names and summaries)
CREATE TABLE IF NOT EXISTS chain_metadata (
    chain_id TEXT PRIMARY KEY,
    generated_name TEXT,
    summary TEXT,
    key_topics TEXT,
    category TEXT,
    confidence REAL,
    generated_at TEXT,
    model_used TEXT,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP
);

-- Layer 7: Chain Summaries (Intel-generated chain summaries)
CREATE TABLE IF NOT EXISTS chain_summaries (
    chain_id TEXT PRIMARY KEY,
    summary TEXT,
    accomplishments TEXT,
    status TEXT,
    key_files TEXT,
    workstream_tags TEXT,
    model_used TEXT,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP
);
```

Bump schema_version from `'2.1'` to `'2.2'` (line 222).

### Step 2: Remove competing `chain_metadata` from `cache.rs`

**File:** `core/src/intelligence/cache.rs`
**Lines:** 402-464 (`MIGRATION_SQL` constant)

Remove the `chain_metadata` and `chain_summaries` CREATE TABLE statements from `MIGRATION_SQL`. Keep all other tables (`commit_analysis`, `session_summaries`, `insights_cache`, `intelligence_costs`) since they are cache.rs-only tables.

**Before (lines 402-464):**
```rust
const MIGRATION_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS chain_metadata (
    chain_id TEXT PRIMARY KEY,
    generated_name TEXT,
    category TEXT,
    confidence REAL,
    generated_at TEXT,
    model_used TEXT,
    created_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS commit_analysis ( ... );
CREATE TABLE IF NOT EXISTS session_summaries ( ... );
CREATE TABLE IF NOT EXISTS insights_cache ( ... );
CREATE TABLE IF NOT EXISTS intelligence_costs ( ... );
CREATE TABLE IF NOT EXISTS chain_summaries ( ... );
"#;
```

**After:**
```rust
const MIGRATION_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS commit_analysis (
    commit_hash TEXT PRIMARY KEY,
    is_agent_commit INTEGER NOT NULL DEFAULT 0,
    summary TEXT,
    risk_level TEXT,
    review_focus TEXT,
    related_files TEXT,
    analyzed_at TEXT,
    model_used TEXT,
    created_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS session_summaries (
    session_id TEXT PRIMARY KEY,
    summary TEXT,
    key_files TEXT,
    focus_area TEXT,
    model_used TEXT,
    created_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS insights_cache (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    insight_type TEXT,
    title TEXT,
    description TEXT,
    evidence TEXT,
    action TEXT,
    generated_at TEXT,
    expires_at TEXT,
    model_used TEXT
);

CREATE TABLE IF NOT EXISTS intelligence_costs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    operation TEXT NOT NULL,
    model TEXT NOT NULL,
    cost_usd REAL NOT NULL,
    timestamp TEXT DEFAULT (datetime('now'))
);
"#;
```

### Step 3: Add migration for existing databases

**File:** `core/src/storage.rs`
**Location:** After the main `SCHEMA_SQL` execution in `ensure_schema()`, before the function returns.

Add ALTER TABLE statements wrapped in individual try-catch blocks. Each ALTER is idempotent: if the column already exists, SQLite returns "duplicate column name" which we silently ignore.

```rust
// Migration: add columns that may be missing from older schemas
let migrations = vec![
    // chain_metadata columns from cache.rs
    "ALTER TABLE chain_metadata ADD COLUMN category TEXT",
    "ALTER TABLE chain_metadata ADD COLUMN confidence REAL",
    "ALTER TABLE chain_metadata ADD COLUMN generated_at TEXT",
    "ALTER TABLE chain_metadata ADD COLUMN model_used TEXT",
    // chain_metadata columns from storage.rs (in case cache.rs ran first historically)
    "ALTER TABLE chain_metadata ADD COLUMN summary TEXT",
    "ALTER TABLE chain_metadata ADD COLUMN key_topics TEXT",
    "ALTER TABLE chain_metadata ADD COLUMN updated_at TEXT DEFAULT CURRENT_TIMESTAMP",
    // chain_graph columns from persist_chains
    "ALTER TABLE chain_graph ADD COLUMN parent_session_id TEXT",
    "ALTER TABLE chain_graph ADD COLUMN is_root BOOLEAN",
    "ALTER TABLE chain_graph ADD COLUMN indexed_at TEXT",
];

for migration in migrations {
    // Ignore "duplicate column name" errors -- column already exists
    let _ = sqlx::query(migration)
        .execute(&self.pool)
        .await;
}
```

**Why ignore errors:** SQLite does not support `ADD COLUMN IF NOT EXISTS`. The only way to make this idempotent is to attempt the ALTER and ignore the "duplicate column name" error. This is the standard SQLite migration pattern.

### Step 4: Verify INSERT statements in cache.rs

After Steps 1-3, the INSERT at `cache.rs:58-60` will succeed because all target columns (`category`, `confidence`, `generated_at`, `model_used`) now exist in the table.

No changes needed to the INSERT statements themselves.

---

## Migration Strategy

### New databases

`ensure_schema()` creates the unified schema directly. All columns present from the start.

### Existing databases (storage.rs schema won)

The ALTER TABLE migrations in Step 3 add the 4 missing columns (`category`, `confidence`, `generated_at`, `model_used`). The `summary`, `key_topics`, and `updated_at` columns already exist.

### Existing databases (hypothetical: cache.rs schema won)

The ALTER TABLE migrations add the 3 missing columns (`summary`, `key_topics`, `updated_at`). The `category`, `confidence`, `generated_at`, and `model_used` columns already exist.

### `chain_summaries` table

`CREATE TABLE IF NOT EXISTS` in `ensure_schema()` creates it if missing. If it already exists from a previous cache.rs migration, the statement is a no-op.

### `chain_graph` table

The ALTER TABLE migrations add `parent_session_id`, `is_root`, `indexed_at` if they don't exist. Note: `persist_chains()` still DROP+recreates this table on every sync, so the migration only matters for the brief window between `ensure_schema()` and the first `persist_chains()` call.

---

## TDD Test Plan

### test_unified_schema_has_all_columns

```rust
#[tokio::test]
async fn test_unified_schema_has_all_columns() {
    let db = Database::new_in_memory().await.unwrap();
    db.ensure_schema().await.unwrap();

    // Verify chain_metadata has ALL expected columns
    let rows = sqlx::query("PRAGMA table_info(chain_metadata)")
        .fetch_all(db.pool())
        .await
        .unwrap();

    let col_names: Vec<String> = rows.iter()
        .map(|r| r.get::<String, _>("name"))
        .collect();

    assert!(col_names.contains(&"chain_id".to_string()));
    assert!(col_names.contains(&"generated_name".to_string()));
    assert!(col_names.contains(&"summary".to_string()));
    assert!(col_names.contains(&"key_topics".to_string()));
    assert!(col_names.contains(&"category".to_string()));
    assert!(col_names.contains(&"confidence".to_string()));
    assert!(col_names.contains(&"generated_at".to_string()));
    assert!(col_names.contains(&"model_used".to_string()));
    assert!(col_names.contains(&"created_at".to_string()));
    assert!(col_names.contains(&"updated_at".to_string()));
}
```

### test_cache_writes_succeed_after_schema_update

```rust
#[tokio::test]
async fn test_cache_writes_succeed_after_schema_update() {
    let db = Database::new_in_memory().await.unwrap();
    db.ensure_schema().await.unwrap();

    // Simulate the exact INSERT that cache.rs performs
    let result = sqlx::query(
        r#"INSERT OR REPLACE INTO chain_metadata
        (chain_id, generated_name, category, confidence, generated_at, model_used, created_at)
        VALUES (?, ?, ?, ?, ?, ?, ?)"#,
    )
    .bind("test-chain-id")
    .bind("Test Chain Name")
    .bind("development")
    .bind(0.95)
    .bind("2026-02-06T12:00:00Z")
    .bind("claude-sonnet-4-5-20250929")
    .bind("2026-02-06T12:00:00Z")
    .execute(db.pool())
    .await;

    assert!(result.is_ok(), "Cache INSERT should succeed with unified schema");

    // Verify the row was written
    let row = sqlx::query("SELECT category, confidence FROM chain_metadata WHERE chain_id = ?")
        .bind("test-chain-id")
        .fetch_one(db.pool())
        .await
        .unwrap();

    assert_eq!(row.get::<String, _>("category"), "development");
    assert_eq!(row.get::<f64, _>("confidence"), 0.95);
}
```

### test_migration_adds_missing_columns

```rust
#[tokio::test]
async fn test_migration_adds_missing_columns() {
    let db = Database::new_in_memory().await.unwrap();

    // Create the OLD storage.rs schema (without cache.rs columns)
    sqlx::query(
        "CREATE TABLE chain_metadata (
            chain_id TEXT PRIMARY KEY,
            generated_name TEXT,
            summary TEXT,
            key_topics TEXT,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT DEFAULT CURRENT_TIMESTAMP
        )"
    )
    .execute(db.pool())
    .await
    .unwrap();

    // Run ensure_schema which includes migrations
    db.ensure_schema().await.unwrap();

    // Verify new columns were added
    let rows = sqlx::query("PRAGMA table_info(chain_metadata)")
        .fetch_all(db.pool())
        .await
        .unwrap();

    let col_names: Vec<String> = rows.iter()
        .map(|r| r.get::<String, _>("name"))
        .collect();

    assert!(col_names.contains(&"category".to_string()));
    assert!(col_names.contains(&"confidence".to_string()));
    assert!(col_names.contains(&"generated_at".to_string()));
    assert!(col_names.contains(&"model_used".to_string()));
}
```

### test_schema_idempotent

```rust
#[tokio::test]
async fn test_schema_idempotent() {
    let db = Database::new_in_memory().await.unwrap();

    // Run ensure_schema twice - should not error
    db.ensure_schema().await.unwrap();
    db.ensure_schema().await.unwrap();

    // Verify schema is still correct
    let rows = sqlx::query("PRAGMA table_info(chain_metadata)")
        .fetch_all(db.pool())
        .await
        .unwrap();

    assert_eq!(rows.len(), 10, "chain_metadata should have exactly 10 columns");
}
```

### test_chain_summaries_table_exists

```rust
#[tokio::test]
async fn test_chain_summaries_table_exists() {
    let db = Database::new_in_memory().await.unwrap();
    db.ensure_schema().await.unwrap();

    // Verify chain_summaries table exists and has correct columns
    let rows = sqlx::query("PRAGMA table_info(chain_summaries)")
        .fetch_all(db.pool())
        .await
        .unwrap();

    let col_names: Vec<String> = rows.iter()
        .map(|r| r.get::<String, _>("name"))
        .collect();

    assert!(col_names.contains(&"chain_id".to_string()));
    assert!(col_names.contains(&"summary".to_string()));
    assert!(col_names.contains(&"accomplishments".to_string()));
    assert!(col_names.contains(&"status".to_string()));
    assert!(col_names.contains(&"key_files".to_string()));
    assert!(col_names.contains(&"workstream_tags".to_string()));
    assert!(col_names.contains(&"model_used".to_string()));
    assert!(col_names.contains(&"created_at".to_string()));

    // Verify INSERT works
    let result = sqlx::query(
        r#"INSERT OR REPLACE INTO chain_summaries
        (chain_id, summary, accomplishments, status, key_files, workstream_tags, model_used, created_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)"#,
    )
    .bind("test-chain")
    .bind("A test summary")
    .bind("[]")
    .bind("active")
    .bind("[]")
    .bind("[]")
    .bind("claude-sonnet-4-5-20250929")
    .bind("2026-02-06T12:00:00Z")
    .execute(db.pool())
    .await;

    assert!(result.is_ok(), "chain_summaries INSERT should succeed");
}
```

---

## Success Criteria

### 1. Runtime verification

After applying the fix, run:

```bash
sqlite3 ~/.context-os/context_os_events.db "PRAGMA table_info(chain_metadata);"
```

**Expected output** (all 10 columns present):

```
0|chain_id|TEXT|0||1
1|generated_name|TEXT|0||0
2|summary|TEXT|0||0
3|key_topics|TEXT|0||0
4|category|TEXT|0||0
5|confidence|REAL|0||0
6|generated_at|TEXT|0||0
7|model_used|TEXT|0||0
8|created_at|TEXT|0||0
9|updated_at|TEXT|0||0
```

### 2. chain_summaries exists

```bash
sqlite3 ~/.context-os/context_os_events.db "PRAGMA table_info(chain_summaries);"
```

Should return 8 columns.

### 3. Intel cache writes succeed

After a sync with intelligence enabled, verify:

```bash
sqlite3 ~/.context-os/context_os_events.db "SELECT chain_id, generated_name, category, confidence FROM chain_metadata LIMIT 5;"
```

All columns should have values (not NULL for category/confidence on Intel-enriched rows).

### 4. All tests pass

```bash
cd core && cargo test
```

---

## Handoff Checklist

- [ ] Read `storage.rs:130-230` and understand current `ensure_schema()` structure
- [ ] Read `intelligence/cache.rs:401-464` and understand `MIGRATION_SQL` tables
- [ ] Update `chain_graph` CREATE TABLE in `ensure_schema()` to include 5 columns
- [ ] Update `chain_metadata` CREATE TABLE in `ensure_schema()` to include all 10 columns
- [ ] Add `chain_summaries` CREATE TABLE to `ensure_schema()`
- [ ] Add ALTER TABLE migration block after SCHEMA_SQL execution
- [ ] Bump `schema_version` from `'2.1'` to `'2.2'`
- [ ] Remove `chain_metadata` CREATE TABLE from cache.rs `MIGRATION_SQL`
- [ ] Remove `chain_summaries` CREATE TABLE from cache.rs `MIGRATION_SQL`
- [ ] Write and pass `test_unified_schema_has_all_columns`
- [ ] Write and pass `test_cache_writes_succeed_after_schema_update`
- [ ] Write and pass `test_migration_adds_missing_columns`
- [ ] Write and pass `test_schema_idempotent`
- [ ] Write and pass `test_chain_summaries_table_exists`
- [ ] Run full `cargo test` -- all existing tests still pass
- [ ] Manual verification: run sync, check `PRAGMA table_info(chain_metadata)` shows 10 columns
- [ ] Manual verification: confirm Intel enrichment writes succeed (check for category/confidence values)

---

**Created:** 2026-02-06
**Source Audit:** `specs/audits/2026-02-06/cross_check_data_pipeline.md` (XCHECK-1), `specs/audits/2026-02-06/audit_data_pipeline.md` (BUG-09)
