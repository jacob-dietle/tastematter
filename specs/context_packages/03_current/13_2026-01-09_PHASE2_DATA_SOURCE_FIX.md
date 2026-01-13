---
title: "Tastematter Context Package 13"
package_number: 13

migrated_from: "apps/tastematter/specs/context_packages/13_2026-01-09_PHASE2_DATA_SOURCE_FIX.md"
status: current
previous_package: "[[12_2026-01-08_PHASE1_CORE_COMPLETE]]"
related:
  - "[[apps/tastematter/src-tauri/src/commands.rs]]"
  - "[[apps/tastematter/src-tauri/src/lib.rs]]"
  - "[[apps/context-os/core/src/storage.rs]]"
  - "[[apps/context-os/core/src/query.rs]]"
tags:
  - context-package
  - tastematter
  - phase-2-tauri
  - data-source-fix
  - architectural-gap
---

# Tastematter - Context Package 13

## Executive Summary

Phase 2 Tauri Integration BUILDS SUCCESSFULLY. Replaced all 4 Python subprocess queries with direct Rust library calls. However, discovered **critical architectural gap**: `file_conversation_index` table is EMPTY (0 rows). Data exists only in `claude_sessions.files_read` as JSON arrays. Python CLI builds index from JSONL at runtime (18s), never uses SQLite. **Next agent must rewrite query.rs to query claude_sessions with json_each().**

## Global Context

### Architecture Overview

```
apps/tastematter/                    # Tauri desktop app
├── src-tauri/
│   ├── Cargo.toml                   # Now depends on context-os-core
│   └── src/
│       ├── lib.rs                   # AppState with QueryEngine (OnceCell)
│       └── commands.rs              # REWRITTEN - uses Rust core, not Python
│
apps/context-os/                     # Unified context system
├── core/                            # Rust library (Phase 1 complete)
│   └── src/
│       ├── query.rs                 # NEEDS REWRITE - queries wrong table
│       └── storage.rs               # FIXED - path resolution now correct
└── cli/                             # Python CLI (preserved)
```

### Critical Discovery: Database Schema Gap

**What we thought:**
```
file_conversation_index table has file access data
→ Query it directly for fast results
```

**What we found:**
```sql
SELECT COUNT(*) FROM file_conversation_index;  -- 0 rows
SELECT COUNT(*) FROM claude_sessions;          -- 920 rows with JSON data
```

**Root Cause Chain:**
1. Daemon writes to `claude_sessions` with `files_read` JSON column
2. Daemon NEVER calls `ContextIndex.persist()` to populate `file_conversation_index`
3. Python CLI builds index from JSONL files at runtime (18s startup)
4. Python CLI never queries SQLite for file data
5. Rust queries SQLite `file_conversation_index` → 0 results

[VERIFIED: sqlite3 queries on context_os_events.db]

### Database Population State

| Table | Rows | Has Data |
|-------|------|----------|
| claude_sessions | 920 | files_read (JSON array), files_written (JSON array) |
| chain_graph | 775 | session_id, chain_id relationships |
| chains | 643 | chain metadata, files_json, files_bloom |
| git_commits | 149 | commit data |
| file_events | 0 | EMPTY |
| file_conversation_index | 0 | EMPTY |

[VERIFIED: Python sqlite3 COUNT(*) on each table]

### Key Design Decisions

1. **Lazy QueryEngine initialization** - OnceCell for first-query init [VERIFIED: [[lib.rs]]:4-12]
2. **Database path resolution fixed** - Checks file size, prioritizes underscore path [VERIFIED: [[storage.rs]]:82-102]
3. **Direct SQLite over subprocess** - Commands now use Rust, not Python [VERIFIED: [[commands.rs]]:all]

## Local Problem Set

### Completed This Session

- [X] Loaded context from package 12 via /context-foundation [VERIFIED: session start]
- [X] Added context-os-core dependency to Tauri Cargo.toml [VERIFIED: [[Cargo.toml]]]
- [X] Added tokio dependency for async runtime [VERIFIED: [[Cargo.toml]]]
- [X] Created AppState with QueryEngine OnceCell [VERIFIED: [[lib.rs]]:4-12]
- [X] Rewrote all 4 query commands to use Rust core [VERIFIED: [[commands.rs]]]
  - query_flex, query_timeline, query_sessions, query_chains
- [X] Fixed lifetime issues in get_query_engine helper [VERIFIED: build succeeds]
- [X] Fixed type annotation errors in closures [VERIFIED: build succeeds]
- [X] Removed all dead code (unused structs, functions, imports) [VERIFIED: git diff]
- [X] Fixed empty database file issue - was finding 0-byte ~/.context-os/context.db [VERIFIED: [[storage.rs]]:116-123]
- [X] Fixed underscore vs hyphen path issue - daemon writes to context_os_events/ [VERIFIED: [[storage.rs]]:89-102]
- [X] cargo build --release succeeds [VERIFIED: build output]
- [X] All 15 core tests still pass [VERIFIED: cargo test]
- [X] Root cause identified: file_conversation_index empty [VERIFIED: sqlite3 queries]

### In Progress

- [ ] Rewrite query.rs to query claude_sessions.files_read JSON
  - Current state: Queries file_conversation_index (0 rows)
  - Needs: Query claude_sessions with json_each() for file data
  - Evidence: [VERIFIED: sqlite3 shows data only in claude_sessions]

### Jobs To Be Done (Next Session)

1. [ ] Rewrite query_flex to use claude_sessions.files_read
   - Success criteria: Returns actual file data, not empty results
   - SQL pattern: `SELECT json_each.value FROM claude_sessions, json_each(files_read)`
   - Estimated: ~50 lines changed in query.rs

2. [ ] Rewrite query_sessions to use claude_sessions directly
   - Success criteria: Session list with file counts from JSON
   - File: [[apps/context-os/core/src/query.rs]]

3. [ ] Rewrite query_timeline to extract from session timestamps
   - Success criteria: Daily buckets populated from claude_sessions
   - File: [[apps/context-os/core/src/query.rs]]

4. [ ] Test end-to-end in Tauri app
   - Success criteria: Frontend displays real data
   - Evidence: DevTools shows query results with actual files

5. [ ] (Future) Fix daemon to populate file_conversation_index
   - This is Phase 2B - proper fix
   - For now, JSON parsing works

## File Locations

| File | Purpose | Status |
|------|---------|--------|
| [[apps/tastematter/src-tauri/Cargo.toml]] | Added context-os-core dep | Modified |
| [[apps/tastematter/src-tauri/src/lib.rs]] | AppState with QueryEngine | Modified |
| [[apps/tastematter/src-tauri/src/commands.rs]] | All 4 commands rewritten | Modified |
| [[apps/context-os/core/src/storage.rs]] | Fixed path resolution | Modified |
| [[apps/context-os/core/src/query.rs]] | NEEDS REWRITE for JSON | Pending |

## Test State

- **Core tests:** 15 passing (Phase 1)
- **Build:** cargo build --release succeeds
- **Tauri dev:** Starts but queries return empty (expected - wrong data source)
- **Command:** `cd apps/context-os/core && cargo test`
- **Last run:** 2026-01-09
- **Evidence:** [VERIFIED: cargo test output all green]

### Test Commands for Next Agent

```bash
# Verify core tests still pass
cd apps/context-os/core
cargo test

# Verify Tauri builds
cd apps/tastematter
cargo tauri build

# Check database has data (should show 920)
sqlite3 apps/context_os_events/data/context_os_events.db "SELECT COUNT(*) FROM claude_sessions;"

# Verify files_read has data
sqlite3 apps/context_os_events/data/context_os_events.db "SELECT files_read FROM claude_sessions LIMIT 1;"

# Test JSON parsing query pattern
sqlite3 apps/context_os_events/data/context_os_events.db "SELECT value FROM claude_sessions, json_each(files_read) LIMIT 5;"
```

## For Next Agent

**Context Chain:**
- Previous: [[12_2026-01-08_PHASE1_CORE_COMPLETE]] (Phase 1 done, 15 tests)
- This package: Phase 2 builds but queries wrong data source
- Next action: Rewrite query.rs to use claude_sessions.files_read JSON

**Start here:**
1. Read this context package (you're doing it now)
2. Run test commands above to verify database state
3. Read [[apps/context-os/core/src/query.rs]] - understand current queries
4. Rewrite to use `json_each(files_read)` pattern

**Critical SQL Pattern:**

```sql
-- Current (WRONG - table is empty):
SELECT file_path, COUNT(*) as access_count
FROM file_conversation_index
GROUP BY file_path

-- Needed (CORRECT - data exists here):
SELECT
    json_each.value as file_path,
    COUNT(*) as access_count
FROM claude_sessions, json_each(files_read)
GROUP BY json_each.value
```

**Do NOT:**
- Assume file_conversation_index has data - IT IS EMPTY (0 rows)
- Assume file_events has data - IT IS EMPTY (0 rows)
- Try to fix the daemon in this session - that's Phase 2B
- Edit Tauri commands.rs - they're correct, just need query.rs fixed

**Key insight:**
The Python CLI never queries SQLite for file data. It builds a `ContextIndex` from JSONL files on disk at startup (18 seconds). The SQLite `file_conversation_index` and `file_events` tables exist but are NEVER POPULATED. All file data is stored as JSON arrays in `claude_sessions.files_read` and `claude_sessions.files_written`.

[VERIFIED: sqlite3 queries + Python CLI source code inspection]

**Why this matters:**
- Phase 1 queries are syntactically correct but query empty tables
- Fix is simple: rewrite to query claude_sessions with json_each()
- ~50 lines of query.rs changes, no architectural changes needed
- This is the "simple fix" per debugging skill - query the data where it exists

## Architecture Gap Documentation

### Current State (Problems)

```
DAEMON (Python):
├── Writes claude_sessions with JSON blobs ✓
├── Writes chain_graph ✓
├── Writes chains ✓
├── NEVER calls ContextIndex.persist() ✗
└── NEVER populates file_conversation_index ✗

CLI (Python):
├── Reads JSONL files from disk (18s)
├── Builds ContextIndex in memory
├── Never queries SQLite for files
└── Should query SQLite for speed

RUST CORE:
├── Queries file_conversation_index (empty)
├── Returns 0 results
└── Needs to query claude_sessions JSON
```

### Fix Strategy (Agreed)

**Phase 2A (Now - Simple Fix):**
- Rewrite Rust queries to use `claude_sessions.files_read` JSON
- Uses SQLite json_each() function
- ~50 lines changed in query.rs
- No daemon changes needed

**Phase 2B (Later - Proper Fix):**
- Fix daemon to call `ContextIndex.persist()` after processing
- Populates file_conversation_index properly
- Makes Rust queries work against normalized data
- Better for long-term maintainability

## Code Changes Made This Session

### commands.rs - Key Pattern

```rust
// Before (subprocess):
let mut cmd = Command::new(&cli_path);
cmd.args(&["query", "flex", "--format", "json"]);
let output = cmd.output()?;  // 18 seconds

// After (direct):
let engine = get_query_engine(&state).await?;
let result = engine.query_flex(input).await?;  // 1.5ms
```

### storage.rs - Path Resolution Fix

```rust
// Added validity check for non-empty database
fn is_valid_database(path: &Path) -> bool {
    if let Ok(metadata) = std::fs::metadata(path) {
        metadata.len() > 0  // Must be non-empty
    } else {
        false
    }
}

// Prioritized underscore path (where daemon writes)
let candidates: Vec<PathBuf> = vec![
    PathBuf::from("apps/context_os_events/data/context_os_events.db"),
    // ... other fallbacks
];
```

### lib.rs - State Management

```rust
pub struct AppState {
    pub log_service: Arc<LogService>,
    pub query_engine: Arc<OnceCell<QueryEngine>>,  // Lazy init
}
```
