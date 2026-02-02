---
title: "Tastematter Core Context Package 00"
package_number: 00
date: 2026-02-01
status: current
previous_package: null
related:
  - "[[core/src/daemon/sync.rs]]"
  - "[[core/src/query.rs]]"
  - "[[core/src/types.rs]]"
  - "[[core/src/main.rs]]"
tags:
  - context-package
  - tastematter
  - database-write-path
  - rust-daemon
---

# Tastematter Core - Context Package 00

## Executive Summary

**Critical fix:** The Rust daemon parsed 587K+ tool uses but **never persisted to the database**. Wired existing INSERT methods to sync phases. 989 sessions now being persisted per sync. All 254 tests passing.

## Global Context

### Architecture Overview

Tastematter is a Rust CLI for context intelligence, replacing Python implementation. Key components:

```
┌─────────────────────────────────────────────────────────────────┐
│                     tastematter daemon                          │
├─────────────────────────────────────────────────────────────────┤
│ run_sync() orchestrates:                                        │
│   1. Git sync          → sync_git()                             │
│   2. Session parsing   → sync_sessions_phase() [NOW PERSISTS]   │
│   3. Chain building    → build_chains_phase()  [NOW PERSISTS]   │
│   4. Intelligence      → enrich_chains_phase()                  │
│   5. Inverted index    → build_index_phase()                    │
├─────────────────────────────────────────────────────────────────┤
│                     QueryEngine                                  │
│   - query_flex()       : File activity queries                  │
│   - query_chains()     : Chain metadata queries                 │
│   - upsert_session()   : [NEW] Session persistence              │
│   - persist_chains()   : [NEW] Chain graph persistence          │
└─────────────────────────────────────────────────────────────────┘
```

### Key Design Decisions

1. **Async run_sync()**: Made async to support SQLx database operations
   - [VERIFIED: [[sync.rs]]:51 - `pub async fn run_sync()`]

2. **INSERT OR REPLACE**: Use upsert semantics for re-sync graceful handling
   - [VERIFIED: [[query.rs]]:1144-1151 - SQL uses INSERT OR REPLACE]

3. **Drop-recreate chain tables**: Old Python schema had FK constraints that fail on Rust sync
   - [VERIFIED: [[query.rs]]:1193-1203 - DROP TABLE IF EXISTS]

4. **Multi-thread Tokio runtime for tests**: Blocking I/O in sync phases requires multi-threaded runtime
   - [VERIFIED: [[sync.rs]]:763 - `#[tokio::test(flavor = "multi_thread")]`]

## Local Problem Set

### Completed This Session

- [X] Added `From<SessionSummary> for SessionInput` conversion [VERIFIED: [[types.rs]]:518-535]
- [X] Added `Default` for `SessionInput` [VERIFIED: [[types.rs]]:538-555]
- [X] Made `run_sync()` async with QueryEngine integration [VERIFIED: [[sync.rs]]:51-98]
- [X] Updated `sync_sessions_phase()` to persist via `upsert_session()` [VERIFIED: [[sync.rs]]:119-176]
- [X] Updated `build_chains_phase()` to persist via `persist_chains()` [VERIFIED: [[sync.rs]]:178-211]
- [X] Added `upsert_session()` method to QueryEngine [VERIFIED: [[query.rs]]:1143-1174]
- [X] Added `persist_chains()` method to QueryEngine [VERIFIED: [[query.rs]]:1176-1272]
- [X] Updated main.rs daemon commands for async [VERIFIED: [[main.rs]]:1047, 1091, 1112]
- [X] Fixed path bug: `~/.claude` not `~/.claude/projects` [VERIFIED: [[sync.rs]]:69-73]
- [X] Fixed FK constraint: Drop/recreate chain tables [VERIFIED: [[query.rs]]:1193-1230]
- [X] Updated 5 async tests to use multi-thread Tokio [VERIFIED: [[sync.rs]]:763, 777, 788]

### Bug Fixes During Implementation

1. **Path Bug (Root Cause of 0 sessions):**
   - `find_session_files()` expected `~/.claude` but daemon passed `~/.claude/projects`
   - The function internally joins "projects" to base path
   - Fix: Changed `claude_dir` from `.join(".claude").join("projects")` to `.join(".claude")`
   - [VERIFIED: [[sync.rs]]:69-73]

2. **FK Constraint Failure:**
   - Old Python schema had `chain_graph.session_id` FK to `claude_sessions`
   - Rust sync order: sessions → chains, but FK checked against stale session data
   - Fix: DROP and recreate chain tables without FK constraints
   - [VERIFIED: [[query.rs]]:1193-1230]

3. **Blocking Runtime Error:**
   - Async tests using `#[tokio::test]` default to single-threaded runtime
   - Sync phases call blocking I/O (file system operations)
   - Fix: Use `#[tokio::test(flavor = "multi_thread")]`
   - [VERIFIED: [[sync.rs]]:763]

### Jobs To Be Done (Next Session)

1. [ ] Implement incremental sync from database - Success criteria: Only parse sessions where file_size changed
   - Currently uses empty HashMap for existing_sessions
   - Should query DB for session_id → file_size_bytes mapping
   - Location: [[sync.rs]]:134-136

2. [ ] Add file_events persistence - Success criteria: File watcher events persisted to DB
   - `insert_file_events()` method exists but not wired to daemon
   - Location: [[query.rs]]:1097-1126

3. [ ] Clean up tmp files in core directory - Priority: low
   - Multiple `tmpclaude-*-cwd` files cluttering core/
   - Should add to .gitignore or clean up

## File Locations

| File | Purpose | Status |
|------|---------|--------|
| [[core/src/daemon/sync.rs]] | Sync orchestrator with DB persistence | Modified |
| [[core/src/query.rs]] | QueryEngine with upsert_session/persist_chains | Modified |
| [[core/src/types.rs]] | SessionInput conversion + Default | Modified |
| [[core/src/main.rs]] | Daemon CLI with async run_sync calls | Modified |
| [[core/src/daemon/mod.rs]] | Integration tests with multi-thread runtime | Modified |
| [[core/src/capture/jsonl_parser.rs]] | Session parsing (unchanged, ref only) | Reference |
| [[core/src/index/chain_graph.rs]] | Chain building (unchanged, ref only) | Reference |

## Test State

- **Tests:** 254 passing, 0 failing, 3 ignored
- **Command:** `cargo test --lib`
- **Last run:** 2026-02-01 ~14:45 UTC
- **Duration:** ~390 seconds (sync tests take ~60s each due to full file parsing)

### Test Commands for Next Agent

```bash
# Navigate to core
cd apps/tastematter/core

# Run all library tests
cargo test --lib

# Run just sync tests (slower - parses real files)
cargo test --lib daemon::sync::tests

# Build release binary
cargo build --release

# Test daemon sync
./target/release/tastematter.exe daemon once

# Verify sessions persisted
./target/release/tastematter.exe query flex --time 7d --limit 5

# Verify chains persisted
./target/release/tastematter.exe query chains --limit 3
```

### Verification Output (2026-02-01)

```json
// daemon once output
{
  "git_commits_synced": 26,
  "sessions_parsed": 989,
  "chains_built": 334,
  "files_indexed": 3913,
  "duration_ms": 48645,
  "errors": [
    "Intel: Service unavailable - skipping enrichment"
  ]
}

// query flex --time 7d --limit 5
{
  "result_count": 5,
  "results": [
    {"file_path": "...synchronous-coalescing-harbor.md", "access_count": 108},
    ...
  ]
}

// query chains --limit 3
{
  "chains": [
    {"chain_id": "93a22459", "session_count": 312, "file_count": 2200},
    ...
  ]
}
```

## For Next Agent

**Context Chain:**
- Previous: None (first package for tastematter/core)
- This package: Database write path fix complete, 989 sessions persisting
- Next action: Implement incremental sync or file_events persistence

**Start here:**
1. Read this context package (you're doing it now)
2. Read [[sync.rs]]:119-176 for session persistence flow
3. Read [[query.rs]]:1143-1272 for database write methods
4. Run: `cargo test --lib` to confirm 254 tests pass

**Do NOT:**
- Pass `~/.claude/projects` to `find_session_files()` - it expects `~/.claude`
- Use `#[tokio::test]` without `flavor = "multi_thread"` for sync tests
- Rely on FK constraints between chain_graph and claude_sessions

**Key insight:**
The root cause of "0 sessions parsed" was a path bug: `sync.rs` passed `~/.claude/projects` but `find_session_files()` internally joins `/projects` again, creating `~/.claude/projects/projects/...` which doesn't exist.
[VERIFIED: [[sync.rs]]:69-73 - path fix from `.join(".claude").join("projects")` to `.join(".claude")`]
