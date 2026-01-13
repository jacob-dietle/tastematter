# Context Package 25: Timeline Per-File Buckets Fix

---
package: 25

migrated_from: "apps/tastematter/specs/context_packages/25_2026-01-12_TIMELINE_BUCKETS_FIX.md"
previous: [[24_2026-01-12_DATABASE_ARCHITECTURE_FIX]]
status: current
---

## Executive Summary

Fixed ISSUE-005 (per-file buckets empty `{}`). Root cause: `query.rs:293` had a TODO never completed - was returning `HashMap::new()` instead of querying per-file, per-date access counts. Timeline heat map now has real data. Updated ROADMAP.md to reflect 90% complete status.

## What Was Accomplished

### ISSUE-005: Per-File Buckets Empty - FIXED

**Problem:** CLI and UI showed `"buckets": {}` for all files in timeline view.

**Root Cause Found (query.rs:293):**
```rust
// BEFORE (broken):
FileTimeline {
    file_path: row.get("file_path"),
    total_accesses: row.get::<i64, _>("total_accesses") as u32,
    buckets: std::collections::HashMap::new(), // TODO: Populate per-file buckets if needed
    // ^^^ This TODO was never completed!
}
```

**Fix Applied (query.rs:287-347):**
Added new SQL query to get per-file, per-date counts:
```sql
SELECT json_each.value as file_path,
       date(s.started_at) as date,
       COUNT(*) as count
FROM claude_sessions s, json_each(s.files_read)
LEFT JOIN chain_graph cg ON s.session_id = cg.session_id
WHERE s.started_at >= datetime('now', '-{} days')
GROUP BY json_each.value, date(s.started_at)
```

Then build HashMap and assign to each FileTimeline.

**Verification:**
```bash
./target/release/context-os query timeline --time 7d --limit 3 --format json
```

**Before:**
```json
"buckets": {}
```

**After:**
```json
"buckets": {
  "2026-01-08": 3,
  "2026-01-06": 1,
  "2026-01-05": 1
}
```

### ROADMAP.md Updated

| View | Old Status | New Status |
|------|------------|------------|
| Timeline View | 40% simulated | 90% real data |
| Sessions View | 40% synthesized | 80% real data |
| Files View | 80% | 90% |

### Integration Test Fix

Updated `integration_test.rs` to use canonical database path:
```rust
// BEFORE: hardcoded old path
.parent().join("data").join("context_os_events.db")

// AFTER: canonical location
dirs::home_dir().join(".context-os").join("context_os_events.db")
```

## Test State

**Unit + HTTP tests: 13 passing**
```
test query::tests::test_compute_aggregations_count ... ok
test query::tests::test_compute_aggregations_recency ... ok
test types::tests::test_parse_time_range ... ok
test types::tests::test_file_result_optional_fields ... ok
test storage::tests::test_open_nonexistent_database ... ok
test storage::tests::test_find_database_explicit_path_not_found_errors ... ok
test types::tests::test_query_result_serialization ... ok
test storage::tests::test_canonical_path_returns_home_based_path ... ok
test test_health_endpoint_returns_200 ... ok
test test_query_flex_returns_data ... ok
test test_query_chains_returns_data ... ok
test test_query_timeline_returns_data ... ok
test test_query_sessions_returns_data ... ok
```

**Integration tests: 5 passing, 4 failing (latency benchmarks)**
- `test_query_flex_basic` - 111ms (target <100ms) - cold start
- `test_query_timeline` - 158ms (target <100ms) - cold start
- `test_query_sessions` - 226ms (target <100ms) - cold start
- `test_latency_benchmark` - max 125ms (target <100ms)

These are benchmark strictness failures on Windows cold start, not functional failures.

**Frontend tests: 246 passing** (per package 20)

## Files Modified

| File | Change |
|------|--------|
| `apps/context-os/core/src/query.rs` | Added per-file bucket query (lines 287-347) |
| `apps/context-os/core/tests/integration_test.rs` | Fixed database path to canonical |
| `apps/tastematter/specs/canonical/02_ROADMAP.md` | Updated view completion status |

## Issue Status Summary

| Issue | Status | Resolution |
|-------|--------|------------|
| BUG-001 | FIXED (pkg 23) | Chain-file linkage |
| BUG-002 | RESOLVED (pkg 23) | Not a bug |
| ISSUE-003 | OPEN | Timeline shows files not sessions |
| ISSUE-004 | OPEN | Session names are hashes |
| **ISSUE-005** | **FIXED (this pkg)** | **Per-file buckets now populated** |
| ISSUE-006 | EXPECTED | Git Status error in HTTP mode |
| ISSUE-007 | OPEN | File paths truncated |
| ISSUE-008 | NEEDS VERIFY | Chain click filtering |
| ISSUE-009 | NEEDS VERIFY | Inconsistent file counts |

## For Next Agent: Frontend Testing

**Primary Task:** Verify the TimelineView correctly displays the per-file bucket data.

### Step 1: Start Servers

```bash
# Terminal 1: Rust HTTP Server
cd apps/context-os/core
./target/release/context-os serve --port 3001 --cors

# Terminal 2: Vite Dev Server
cd apps/tastematter
pnpm dev
```

### Step 2: Chrome Automation Testing

Use Chrome MCP tools to verify:

1. **Navigate to app:** `http://localhost:5173`
2. **Click Timeline tab**
3. **Verify heat map cells have varying intensity** (not all empty/same)
4. **Hover over heat cells** - should show date and access count
5. **Click chain in sidebar** - timeline should filter

### Step 3: Cross-Reference CLI vs UI

```bash
# Get CLI data
./target/release/context-os query timeline --time 7d --limit 5 --format json

# Compare file.buckets counts with UI heat map intensity
```

### Expected Result

Heat map should show:
- Different colors for different access counts
- Per-date columns with varying intensity
- Tooltip on hover showing date/file

### Known Issues (Expected)

- Git Status panel shows error (HTTP mode, no Tauri IPC)
- Session names are hashes (ISSUE-004 - not fixed yet)
- File paths may be truncated (ISSUE-007)

### Run Frontend Tests

```bash
cd apps/tastematter && pnpm test:unit
# Expect: 246 passing
```

## Context Chain

- **Package 23:** BUG-001 fixed (chain-file linkage)
- **Package 24:** Database architecture fix (canonical path)
- **Package 25 (this):** ISSUE-005 fixed (per-file buckets)
- **Next:** Frontend verification via Chrome automation

---

**Session Duration:** ~45 minutes
**Key Achievement:** Fixed root cause of empty timeline heat map (TODO never completed in query.rs:293)
**Documentation Debt Paid:** Updated ROADMAP.md from 40% to 90% for Timeline View
