# Non-Destructive Chain Persistence Specification

**Status:** Proposed
**Priority:** High
**Bug ID:** BUG-07
**Estimated Effort:** 3-4 hours
**Related Specs:** 05_SCHEMA_UNIFICATION_SPEC.md (schema alignment)

---

## Problem Statement

`persist_chains()` in `core/src/query.rs:1418-1507` executes `DROP TABLE IF EXISTS chain_graph` and `DROP TABLE IF EXISTS chains` on every daemon sync cycle, then recreates the tables and re-inserts all data. During the DROP-to-recreate window, any concurrent query against `chain_graph` or `chains` returns empty results or errors.

This is the highest-severity bug identified in the data pipeline audit (BUG-07). The chain_metadata table survives because it is NOT dropped by `persist_chains()`, but every query that JOINs on `chain_graph` (flex, sessions, chains, timeline with chain filter) produces empty or broken results during the sync window.

**Severity evidence:** Every daemon sync cycle opens a race condition window. The sync runs on a configurable interval (default: periodic). Any user query during that window sees zero chains.

**Source:** `audit_data_pipeline.md` section 1.5, line 332-335; `cross_check_data_pipeline.md` section 3, line 137.

---

## Root Cause

The original comment in the code (query.rs:1424-1425) explains the rationale:

```rust
// Drop and recreate tables to avoid FK constraint issues from old Python schema
// The old Python schema had FK constraints that cause issues during Rust sync
```

The Python indexer's schema included foreign key constraints between `chain_graph` and `chains`. When the Rust sync was introduced, it was simpler to DROP+recreate the tables without FK constraints than to handle constraint violations during upsert. This was a valid workaround during initial migration but is now a correctness bug since the Python indexer has been superseded.

Additionally, the Python schema had extra columns in `chain_graph` (`position_in_chain`, `children_count`, `parent_message_uuid`) that are not used by the Rust implementation, making the old table shape incompatible. Dropping ensured a clean slate. Now that the Rust schema is established, this is unnecessary.

---

## Current Code (query.rs:1418-1507)

```rust
pub async fn persist_chains(
    &self,
    chains: &std::collections::HashMap<String, crate::index::chain_graph::Chain>,
) -> Result<WriteResult, CoreError> {
    let mut rows = 0u64;

    // DROP TABLE IF EXISTS chain_graph
    // DROP TABLE IF EXISTS chains
    // CREATE TABLE chains (chain_id, root_session_id, session_count, files_count, updated_at)
    // CREATE TABLE chain_graph (session_id, chain_id, parent_session_id, is_root, indexed_at)

    for chain in chains.values() {
        // INSERT OR REPLACE INTO chains ...
        // INSERT OR REPLACE INTO chain_graph ...
    }

    Ok(WriteResult { rows_affected: rows })
}
```

**Schema divergence:** `persist_chains()` creates `chain_graph` with 5 columns (`session_id`, `chain_id`, `parent_session_id`, `is_root`, `indexed_at`), but `ensure_schema()` in storage.rs creates it with only 2 columns (`session_id`, `chain_id`). Whichever runs last determines the live schema. Since `ensure_schema()` uses `IF NOT EXISTS`, the DROP in `persist_chains()` causes the 5-column version to replace the 2-column version on every sync. This is also documented as BUG-09.

---

## Implementation Steps

### Step 1: Update `ensure_schema()` in storage.rs

Add the missing columns to `chain_graph` in the canonical schema definition so that the table created by `ensure_schema()` matches what `persist_chains()` needs to write.

```rust
// In storage.rs ensure_schema(), replace the chain_graph definition:

// BEFORE:
CREATE TABLE IF NOT EXISTS chain_graph (
    session_id TEXT PRIMARY KEY,
    chain_id TEXT NOT NULL
);

// AFTER:
CREATE TABLE IF NOT EXISTS chain_graph (
    session_id TEXT PRIMARY KEY,
    chain_id TEXT NOT NULL,
    parent_session_id TEXT,
    is_root BOOLEAN,
    indexed_at TEXT
);
```

**Note:** This overlaps with Spec 05 (Schema Unification). If Spec 05 is implemented first, this step may already be done. If this spec is implemented first, Spec 05 should reference these columns as already aligned.

For existing databases where `chain_graph` already exists with only 2 columns, add migration logic after the `CREATE TABLE IF NOT EXISTS` block:

```rust
// Add columns if they don't exist (handles existing databases)
// SQLite doesn't have IF NOT EXISTS for ALTER TABLE ADD COLUMN,
// so we attempt the ALTER and ignore "duplicate column" errors.
for alter_sql in &[
    "ALTER TABLE chain_graph ADD COLUMN parent_session_id TEXT",
    "ALTER TABLE chain_graph ADD COLUMN is_root BOOLEAN",
    "ALTER TABLE chain_graph ADD COLUMN indexed_at TEXT",
] {
    let _ = sqlx::query(alter_sql).execute(&self.pool).await;
    // Ignore errors (column already exists)
}
```

### Step 2: Replace DROP+CREATE with idempotent upsert in persist_chains()

Remove all four statements (2x DROP, 2x CREATE) and replace with transactional upsert + stale entry cleanup.

```rust
pub async fn persist_chains(
    &self,
    chains: &std::collections::HashMap<String, crate::index::chain_graph::Chain>,
) -> Result<WriteResult, CoreError> {
    let mut rows = 0u64;

    // Collect current chain IDs for stale detection
    let current_chain_ids: Vec<&str> = chains.keys().map(|s| s.as_str()).collect();

    // Begin an IMMEDIATE transaction to acquire a write lock upfront.
    // This prevents readers from seeing partial state.
    let mut tx = self.db.pool()
        .begin()
        .await
        .map_err(CoreError::Database)?;

    // Step 2a: Remove stale chains that no longer exist
    if current_chain_ids.is_empty() {
        // If no chains provided, clear all entries
        sqlx::query("DELETE FROM chain_graph")
            .execute(&mut *tx)
            .await
            .map_err(CoreError::Database)?;
        sqlx::query("DELETE FROM chains")
            .execute(&mut *tx)
            .await
            .map_err(CoreError::Database)?;
    } else {
        // Build a parameterized IN clause for stale deletion.
        // SQLite supports up to 999 bind parameters, so batch if needed.
        // For typical chain counts (<100), a single query suffices.
        let placeholders: String = current_chain_ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("?{}", i + 1))
            .collect::<Vec<_>>()
            .join(", ");

        let delete_chains_sql = format!(
            "DELETE FROM chains WHERE chain_id NOT IN ({})",
            placeholders
        );
        let delete_graph_sql = format!(
            "DELETE FROM chain_graph WHERE chain_id NOT IN ({})",
            placeholders
        );

        let mut q1 = sqlx::query(&delete_chains_sql);
        for id in &current_chain_ids {
            q1 = q1.bind(id);
        }
        q1.execute(&mut *tx).await.map_err(CoreError::Database)?;

        let mut q2 = sqlx::query(&delete_graph_sql);
        for id in &current_chain_ids {
            q2 = q2.bind(id);
        }
        q2.execute(&mut *tx).await.map_err(CoreError::Database)?;
    }

    // Step 2b: Upsert current chains and their graph entries
    for chain in chains.values() {
        sqlx::query(
            "INSERT OR REPLACE INTO chains (
                chain_id, root_session_id, session_count, files_count, updated_at
            ) VALUES (?, ?, ?, ?, datetime('now'))",
        )
        .bind(&chain.chain_id)
        .bind(&chain.root_session)
        .bind(chain.sessions.len() as i32)
        .bind(chain.files_list.len() as i32)
        .execute(&mut *tx)
        .await
        .map_err(CoreError::Database)?;
        rows += 1;

        // Also remove stale session entries for this chain before re-inserting
        sqlx::query("DELETE FROM chain_graph WHERE chain_id = ?")
            .bind(&chain.chain_id)
            .execute(&mut *tx)
            .await
            .map_err(CoreError::Database)?;

        for session_id in &chain.sessions {
            let is_root = *session_id == chain.root_session;
            let parent = chain
                .branches
                .iter()
                .find(|(_, children)| children.contains(session_id))
                .map(|(p, _)| p.clone());

            sqlx::query(
                "INSERT OR REPLACE INTO chain_graph (
                    session_id, chain_id, parent_session_id, is_root, indexed_at
                ) VALUES (?, ?, ?, ?, datetime('now'))",
            )
            .bind(session_id)
            .bind(&chain.chain_id)
            .bind(&parent)
            .bind(is_root)
            .execute(&mut *tx)
            .await
            .map_err(CoreError::Database)?;
            rows += 1;
        }
    }

    // Commit the transaction atomically
    tx.commit().await.map_err(CoreError::Database)?;

    Ok(WriteResult {
        rows_affected: rows,
    })
}
```

### Step 3: Remove dead code

Delete the four removed statements entirely. Do not leave them as comments. The rationale for the change is documented in this spec and in the git commit message.

---

## Transaction Strategy

**Why `BEGIN IMMEDIATE`:** SQLite's default transaction mode is DEFERRED, which only acquires a write lock when the first write statement executes. With `BEGIN IMMEDIATE`, the write lock is acquired at transaction start, preventing other writers from interleaving. Readers using WAL mode (SQLite default for concurrent access) can still read the pre-transaction state until COMMIT.

**Atomicity guarantee:** Either all stale entries are removed and all current entries are inserted, or none are. If any step fails, the transaction rolls back and the previous chain data remains intact.

**Behavior during sync:**
- Before COMMIT: Readers see the old (complete) chain data
- After COMMIT: Readers see the new (complete) chain data
- No window where readers see empty tables

**Note on sqlx transaction behavior:** `sqlx::Pool::begin()` returns a `Transaction` that automatically rolls back on drop if not committed. This provides implicit rollback on any error path, including panics.

---

## Stale Entry Cleanup

**Problem:** Chains can disappear between syncs if sessions are deleted or if the chain graph algorithm produces different groupings. Without cleanup, orphan entries accumulate.

**Strategy:** Compare the set of chain_ids in the input HashMap against what exists in the database, and DELETE entries whose chain_id is not in the current set.

**Why DELETE per-chain for chain_graph:** A session can only belong to one chain (session_id is PRIMARY KEY in chain_graph). When a chain's membership changes, we DELETE all chain_graph entries for that chain_id and re-INSERT the current membership. This handles:
- Sessions removed from a chain
- Sessions moved between chains
- Chains merged or split

**Edge case -- empty input:** If `persist_chains()` is called with an empty HashMap, all chain and chain_graph entries are removed. This is correct behavior: if the chain builder finds no chains, the database should reflect that.

---

## TDD Test Plan

### test_persist_chains_idempotent

Run `persist_chains()` twice with the same input. Assert that:
- Row counts in `chains` and `chain_graph` are identical after both calls
- All column values match between first and second call
- `updated_at` / `indexed_at` timestamps are refreshed (expected behavior)

```rust
#[tokio::test]
async fn test_persist_chains_idempotent() {
    let db = setup_test_db().await;
    let engine = QueryEngine::new(db);
    let chains = make_test_chains(3); // 3 chains, multiple sessions each

    let result1 = engine.persist_chains(&chains).await.unwrap();
    let count1 = count_rows(&engine, "chains").await;
    let graph1 = count_rows(&engine, "chain_graph").await;

    let result2 = engine.persist_chains(&chains).await.unwrap();
    let count2 = count_rows(&engine, "chains").await;
    let graph2 = count_rows(&engine, "chain_graph").await;

    assert_eq!(count1, count2);
    assert_eq!(graph1, graph2);
    assert_eq!(result1.rows_affected, result2.rows_affected);
}
```

### test_persist_chains_removes_stale_chains

Persist 3 chains, then persist only 2 (removing one). Assert that:
- The removed chain no longer exists in `chains`
- All `chain_graph` entries for the removed chain are gone
- The remaining 2 chains are intact

```rust
#[tokio::test]
async fn test_persist_chains_removes_stale_chains() {
    let db = setup_test_db().await;
    let engine = QueryEngine::new(db);

    let mut chains = make_test_chains(3);
    engine.persist_chains(&chains).await.unwrap();
    assert_eq!(count_rows(&engine, "chains").await, 3);

    let removed_id = chains.keys().next().unwrap().clone();
    chains.remove(&removed_id);

    engine.persist_chains(&chains).await.unwrap();
    assert_eq!(count_rows(&engine, "chains").await, 2);

    // Verify the removed chain's graph entries are also gone
    let stale = query_chain_graph(&engine, &removed_id).await;
    assert!(stale.is_empty());
}
```

### test_persist_chains_does_not_drop_tables

Verify that the `chains` and `chain_graph` tables are never dropped during `persist_chains()`. This can be tested by:
1. Inserting a canary row with a known chain_id before calling `persist_chains()`
2. Calling `persist_chains()` with chains that include the canary chain_id
3. Verifying the table schema (column count) is unchanged
4. Confirming no `DROP TABLE` appears in the function (static analysis / code review)

```rust
#[tokio::test]
async fn test_persist_chains_does_not_drop_tables() {
    let db = setup_test_db().await;
    let engine = QueryEngine::new(db);

    // Verify tables exist before persist
    let schema_before = get_table_schema(&engine, "chain_graph").await;
    assert!(schema_before.contains("session_id"));

    let chains = make_test_chains(2);
    engine.persist_chains(&chains).await.unwrap();

    // Verify tables still exist with same schema after persist
    let schema_after = get_table_schema(&engine, "chain_graph").await;
    assert_eq!(schema_before, schema_after);
}
```

### test_query_during_persist_returns_data

Simulate concurrent read during write. This test verifies that SQLite WAL mode allows readers to see pre-transaction data while a write transaction is in progress.

```rust
#[tokio::test]
async fn test_query_during_persist_returns_data() {
    let db = setup_test_db().await;
    let engine = QueryEngine::new(db);

    // Pre-populate with initial chains
    let chains_v1 = make_test_chains(2);
    engine.persist_chains(&chains_v1).await.unwrap();

    // Verify query returns data (baseline)
    let result = engine.query_chains(Some(10)).await.unwrap();
    assert_eq!(result.chains.len(), 2);

    // Now persist new chains -- during this operation,
    // a concurrent read should still return data (WAL mode)
    let chains_v2 = make_test_chains(3);
    engine.persist_chains(&chains_v2).await.unwrap();

    // After persist, new data is visible
    let result = engine.query_chains(Some(10)).await.unwrap();
    assert_eq!(result.chains.len(), 3);
}
```

**Note:** True concurrency testing with SQLite requires spawning separate connections. A more rigorous test would use `tokio::spawn` to issue a read query on a separate connection while a write transaction is held open on another. This is an integration-level test.

### test_persist_chains_transaction_rollback_on_error

Verify that if an error occurs mid-persist, the transaction rolls back and previous data remains intact.

```rust
#[tokio::test]
async fn test_persist_chains_transaction_rollback_on_error() {
    let db = setup_test_db().await;
    let engine = QueryEngine::new(db);

    // Pre-populate with known good data
    let chains_v1 = make_test_chains(2);
    engine.persist_chains(&chains_v1).await.unwrap();
    let count_before = count_rows(&engine, "chains").await;

    // Attempt to persist chains with invalid data that triggers an error
    // (e.g., by corrupting the database connection or using a read-only DB)
    // If the transaction fails, count should remain unchanged
    // Implementation: use a mock or inject a failing connection mid-transaction

    let count_after = count_rows(&engine, "chains").await;
    assert_eq!(count_before, count_after, "Rollback should preserve previous data");
}
```

**Implementation note:** Testing rollback behavior precisely may require a test helper that wraps the database to inject failures at specific points, or using SQLite's `PRAGMA` to simulate constraints. The key assertion is: after a failed `persist_chains()`, the database contains the data from before the call, not a partial state.

---

## Success Criteria

1. **No DROP TABLE in persist_chains:** The strings `DROP TABLE` and `CREATE TABLE` do not appear in the `persist_chains()` function.

2. **Zero-downtime chain queries:** A query against `chain_graph` or `chains` NEVER returns empty results due to an in-progress sync. Readers always see either the old complete state or the new complete state.

3. **Stale cleanup works:** Chains removed between syncs are cleaned from both `chains` and `chain_graph` tables.

4. **Idempotent operation:** Running `persist_chains()` twice with the same input produces identical database state (row counts and values, modulo timestamps).

5. **Schema alignment:** `ensure_schema()` in storage.rs creates `chain_graph` with all 5 columns, eliminating the schema divergence (BUG-09).

6. **All existing tests pass:** No regressions in `cargo test`.

---

## Handoff Checklist

- [ ] Read this spec fully before starting implementation
- [ ] Verify `ensure_schema()` in storage.rs already has the 5-column `chain_graph` schema (may be done by Spec 05)
- [ ] If not, add the 3 missing columns + ALTER TABLE migration for existing DBs
- [ ] Remove all DROP TABLE and CREATE TABLE statements from `persist_chains()`
- [ ] Wrap all operations in a transaction (BEGIN IMMEDIATE via `pool.begin()`)
- [ ] Add stale entry cleanup (DELETE WHERE chain_id NOT IN current set)
- [ ] Keep INSERT OR REPLACE logic for both tables (already exists, just redirect to transaction)
- [ ] Run `cargo test` -- all existing tests must pass
- [ ] Add the 5 new tests from the TDD plan above
- [ ] Run `cargo test` again -- all new tests must pass
- [ ] Verify with a manual test: start a `serve` session, trigger sync, confirm chain queries return data throughout
- [ ] Coordinate with Spec 05 author on schema column ownership

---

**Created:** 2026-02-06
**Author:** Tastematter audit implementation planning
