---
title: "Tastematter Context Package 22"
package_number: 22

migrated_from: "apps/tastematter/specs/context_packages/22_2026-01-11_CHAIN_LINKAGE_BUG_RCA.md"
status: current
previous_package: "[[21_2026-01-10_INTELLIGENCE_LAYER_SPEC]]"
related:
  - "[[06_CHAIN_LINKAGE_BUG_RCA.md]]"
  - "[[apps/context-os/core/src/query.rs]]"
  - "[[apps/context-os/core/tests/integration_test.rs]]"
tags:
  - context-package
  - tastematter
  - bug-fix
  - visual-testing
---

# Tastematter - Context Package 22: Chain-File Linkage Bug RCA

## Executive Summary

Visual debugging session using Chrome automation identified **critical data architecture bug**: chain-file linkage broken at database layer. All chains show `file_count: 0`. Root cause found in `query_chains()` reading stale `chains.files_json` instead of computing dynamically. Fix designed, failing test written, implementation blocked by locked executable.

## Global Context

**Project:** Tastematter - Context visualization desktop app
**Architecture:** Two-service design (Rust core on :3001, future Python intel on :3002)
**Test State:** 246 frontend tests passing [VERIFIED: previous package]

### Key Architecture Understanding

```
┌─────────────────────────────────────────┐
│         claude_sessions table           │
│  - session_id                           │
│  - files_read (JSON array)   ← SOURCE   │
│  - started_at, ended_at                 │
└───────────────┬─────────────────────────┘
                │
                │ LEFT JOIN
                ▼
┌─────────────────────────────────────────┐
│         chain_graph table               │
│  - session_id → chain_id mapping        │
└───────────────┬─────────────────────────┘
                │
                │ BUG: query_chains reads from stale table
                ▼
┌─────────────────────────────────────────┐
│           chains table                  │
│  - chain_id                             │
│  - session_count (correct)              │
│  - files_json (EMPTY/STALE) ← BUG       │
└─────────────────────────────────────────┘
```

## Local Problem Set

### Completed This Session

- [X] Chrome automation visual testing of all views [VERIFIED: screenshots captured]
- [X] Identified root cause of chain-file linkage bug [VERIFIED: [[query.rs]]:146-169]
- [X] Documented comprehensive bug report [VERIFIED: [[06_CHAIN_LINKAGE_BUG_RCA.md]]]
- [X] Wrote failing test `test_query_chains_file_count_not_zero` [VERIFIED: [[integration_test.rs]]:142-178]

### In Progress

- [ ] Fix `query_chains()` to compute file counts dynamically
  - **Blocker:** Rust executable locked (context-os.exe in use)
  - **Fix approach:** Change SQL from reading `chains.files_json` to joining:
    ```sql
    SELECT cg.chain_id,
           COUNT(DISTINCT cg.session_id) as session_count,
           COUNT(DISTINCT json_each.value) as file_count
    FROM chain_graph cg
    JOIN claude_sessions s ON cg.session_id = s.session_id
    LEFT JOIN json_each(s.files_read) ON s.files_read IS NOT NULL
    GROUP BY cg.chain_id
    ```

### Jobs To Be Done (Next Session)

1. [ ] **Kill locked process** - Close VS Code terminal or restart to release context-os.exe
2. [ ] **Run failing test** - `cargo test test_query_chains_file_count_not_zero` should FAIL
3. [ ] **Apply fix** - Update `query_chains()` in [[query.rs]]:146-186
4. [ ] **Run test again** - Should PASS (GREEN)
5. [ ] **Verify via CLI** - `context-os.exe query chains` should show file_count > 0
6. [ ] **Verify via UI** - Chrome automation: chains sidebar should show files

## Critical Bug Details

### BUG-001: Chain-to-File Linkage Broken

**Evidence from CLI:**
```json
{
  "chain_id": "7f389600",
  "session_count": 81,
  "file_count": 0  // Should NOT be 0!
}
```

**Evidence from UI:** All chains in sidebar show "X sessions 0 files"

**Root Cause:** `query_chains()` at [[query.rs]]:146-156 reads from `chains.files_json`:
```rust
let sql = "SELECT chain_id, session_count, files_json FROM chains..."
let file_count = files_json
    .and_then(|j| serde_json::from_str::<Vec<String>>(j).ok())
    .map(|v| v.len() as u32)
    .unwrap_or(0);  // Always 0 because files_json is empty!
```

**Correct pattern** (from `query_sessions()` at [[query.rs]]:439-467):
```rust
// Joins to compute file count dynamically from session data
"SELECT cg.chain_id,
        COUNT(DISTINCT json_each.value) as file_count
 FROM chain_graph cg
 JOIN claude_sessions s ON cg.session_id = s.session_id
 LEFT JOIN json_each(s.files_read)..."
```

### BUG-002: Sessions Missing chain_id

**Evidence:** Session query returns no `chain_id` field exposed to frontend
**Impact:** All sessions show "No chain" badge in UI
**Fix:** Add chain_id to session response (already in SQL, just not surfaced properly)

## File Locations

| File | Purpose | Status |
|------|---------|--------|
| [[apps/context-os/core/src/query.rs]] | Query engine with bug | Needs fix at L146-186 |
| [[apps/context-os/core/tests/integration_test.rs]] | Integration tests | Failing test added |
| [[apps/tastematter/specs/06_CHAIN_LINKAGE_BUG_RCA.md]] | Bug report | Created |

## Test State

- Frontend tests: 246 passing [VERIFIED: previous session]
- Rust core tests: **BLOCKED** (executable locked)
- New failing test: `test_query_chains_file_count_not_zero` [WRITTEN: [[integration_test.rs]]:142-178]

### Test Commands for Next Agent

```bash
# 1. First kill any locked processes
taskkill /F /IM context-os.exe

# 2. Run failing test (should FAIL before fix)
cd apps/context-os/core
cargo test test_query_chains_file_count_not_zero -- --nocapture

# 3. After fix, run all tests
cargo test -- --nocapture

# 4. Verify via CLI
./target/debug/context-os.exe query chains --format json
# Expect: file_count > 0 for chains with sessions
```

## For Next Agent

**Context Chain:**
- Previous: [[21_2026-01-10_INTELLIGENCE_LAYER_SPEC]] (Intelligence Layer architecture)
- This package: Visual debugging, BUG-001 identified and test written
- Next action: Apply fix to query_chains(), verify test passes

**Start here:**
1. Kill locked context-os.exe process (close terminals/VS Code)
2. Run `cargo test test_query_chains_file_count_not_zero` - confirm FAILS
3. Apply fix in [[query.rs]]:142-186 (change SQL to join pattern)
4. Run test again - confirm PASSES
5. Run CLI verification
6. Test in Chrome UI

**The Fix (apply to [[query.rs]]:142-186):**
```rust
pub async fn query_chains(&self, input: QueryChainsInput) -> Result<ChainQueryResult, CoreError> {
    let start = Instant::now();
    let limit = input.limit.unwrap_or(20);

    // FIX: Compute file_count dynamically by joining to session data
    // instead of reading from stale chains.files_json
    let sql = format!(
        "SELECT
            cg.chain_id,
            COUNT(DISTINCT cg.session_id) as session_count,
            COUNT(DISTINCT json_each.value) as file_count
         FROM chain_graph cg
         JOIN claude_sessions s ON cg.session_id = s.session_id
         LEFT JOIN json_each(s.files_read) ON s.files_read IS NOT NULL AND s.files_read != '[]'
         GROUP BY cg.chain_id
         ORDER BY session_count DESC
         LIMIT {}",
        limit
    );

    let rows = sqlx::query(&sql)
        .fetch_all(self.db.pool())
        .await?;

    let chains: Vec<ChainData> = rows
        .iter()
        .map(|row| {
            ChainData {
                chain_id: row.get("chain_id"),
                session_count: row.get::<i64, _>("session_count") as u32,
                file_count: row.get::<i64, _>("file_count") as u32,
                time_range: None,
            }
        })
        .collect();

    // ... rest unchanged
}
```

**Do NOT:**
- Skip the failing test step (TDD: must see RED before GREEN)
- Edit the `chains` table directly (it's populated by Python daemon)
- Assume fix works without CLI + UI verification

**Key insight:**
The `chains` table `files_json` column is a denormalized cache that's never populated. The correct data lives in `claude_sessions.files_read` JSON, which must be joined via `chain_graph`. [VERIFIED: [[query.rs]] analysis]

---

**Package Created:** 2026-01-11
**Chrome Tab ID:** 1286546403
**Dev Server:** localhost:5176
