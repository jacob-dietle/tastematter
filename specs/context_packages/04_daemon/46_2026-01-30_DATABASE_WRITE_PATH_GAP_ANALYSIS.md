---
title: "Tastematter Context Package 46"
package_number: 46
date: 2026-01-30
status: current
previous_package: "[[45_2026-01-29_CHAIN_SUMMARY_PRACTICAL_TESTS_AND_DISTRIBUTION_STRATEGY]]"
related:
  - "[[core/src/query.rs]]"
  - "[[core/src/daemon/sync.rs]]"
  - "[[cli/src/context_os_events/index/chain_graph.py]]"
  - "[[cli/src/context_os_events/db/schema.sql]]"
  - "[[specs/canonical/07_CLAUDE_CODE_DATA_MODEL.md]]"
tags:
  - context-package
  - tastematter
  - database-persistence
  - bug-analysis
  - rust-port
---

# Tastematter - Context Package 46: Database Write Path Gap Analysis

## Executive Summary

**Critical bug discovered:** The Rust daemon port parses sessions, builds chains, and indexes files but **NEVER WRITES to the database**. INSERT methods exist in `query.rs` but are never called by the daemon orchestrator. The database has stale data from Jan 18 (last Python CLI run). Only intelligence enrichment (chain names/summaries) actually persists.

**Root Cause:** Package 28 declared Rust port "100% complete" but explicitly listed "Database Persistence" as an "optional Phase 8.5 enhancement" that was **never implemented**.

## Global Context

### Architecture Overview

```
EXPECTED DATA FLOW (per Python implementation):
┌──────────────────┐     ┌──────────────────┐     ┌──────────────────┐
│  JSONL Parser    │────▶│  INSERT into     │────▶│  SQLite DB       │
│  sync_sessions() │     │  claude_sessions │     │  (queryable)     │
└──────────────────┘     └──────────────────┘     └──────────────────┘

ACTUAL DATA FLOW (Rust implementation):
┌──────────────────┐     ┌──────────────────┐     ┌──────────────────┐
│  JSONL Parser    │────▶│  Returns         │────▶│  DISCARDED       │
│  sync_sessions() │     │  Vec<Summary>    │     │  (never persisted)│
└──────────────────┘     └──────────────────┘     └──────────────────┘
```

### Key Design Decision (the gap)

From Package 28 (2026-01-19):
```
## What's Next
The Rust port is **functionally complete**. Optional enhancements for Phase 8.5:
...
2. **Database Persistence** - Persist sync results to SQLite  ← NEVER IMPLEMENTED
```

[VERIFIED: [[28_2026-01-19_PHASE8_DAEMON_RUNNER_COMPLETE.md]]:165]

## Local Problem Set

### Completed This Session

- [X] Identified CLI not returning query results for recent activity [VERIFIED: `tastematter query flex --time 7d` returns 0]
- [X] Diagnosed DB stale - last indexed activity Jan 18 [VERIFIED: SQL query MAX(timestamp)]
- [X] Found parse-sessions counts 587K tool uses but inserts 0 file events [VERIFIED: DB file timestamp unchanged after parse]
- [X] Traced full write path in Rust code - INSERT methods exist but never called [VERIFIED: [[query.rs]]:1025-1097]
- [X] Found Python `persist_chains()` function that Rust lacks equivalent wiring for [VERIFIED: [[chain_graph.py]]:422]
- [X] Enumerated all tables and their write sources (see table below)
- [X] Confirmed intelligence enrichment DOES write (only thing that works) [VERIFIED: [[sync.rs]]:177-339]

### In Progress

- [ ] Fix not yet implemented - awaiting decision on approach

### Jobs To Be Done (Next Session)

1. [ ] **Wire INSERT calls in daemon/sync.rs** - Priority: HIGH
   - In `sync_sessions_phase()`: Call `engine.insert_session()` for each parsed session
   - Estimated: ~30 lines
   - Success criteria: `tastematter daemon once` updates `claude_sessions` table

2. [ ] **Add chain persistence** - Priority: HIGH
   - Create `persist_chains()` equivalent in Rust
   - Write to `chains` and `chain_graph` tables
   - Estimated: ~50 lines
   - Success criteria: Query chains returns fresh data

3. [ ] **Open DB in read-write mode** - Priority: HIGH
   - Change `Database::open()` to `Database::open_rw()` for daemon commands
   - Estimated: ~5 lines
   - Location: [[main.rs]] daemon command handling

4. [ ] **Ensure tables exist** - Priority: MEDIUM
   - Add CREATE TABLE IF NOT EXISTS for `chains` and `chain_graph`
   - These tables aren't in schema.sql - created implicitly by Python
   - Estimated: ~20 lines

5. [ ] **Add incremental sync** - Priority: LOW (optimization)
   - Address TODO at [[sync.rs]]:111
   - Load existing sessions from DB before parsing

## Database Schema Analysis

### Tables by Write Source

| Table | Created By | Written By | Queried By |
|-------|------------|------------|------------|
| `file_events` | Python schema.sql | Python only | Rust query.rs |
| `claude_sessions` | Python schema.sql | Python only | Rust query.rs |
| `git_commits` | Python schema.sql | Python only | Rust query.rs |
| `chains` | Python implicit | Python persist_chains() | Rust query.rs |
| `chain_graph` | Python implicit | Python persist_chains() | Rust query.rs (LEFT JOIN) |
| `chain_metadata` | Rust cache.rs | Rust enrich_chains_phase() | Rust query.rs |
| `chain_summaries` | Rust cache.rs | Rust enrich_chains_phase() | Rust query.rs |

### Rust INSERT Methods (exist but uncalled)

| Method | Location | Called In Production |
|--------|----------|---------------------|
| `insert_session()` | [[query.rs]]:1025 | **NO** |
| `insert_file_event()` | [[query.rs]]:1065 | **NO** |
| `insert_file_events()` | [[query.rs]]:1097 | **NO** |
| `cache_chain_name()` | [[cache.rs]] | YES (works) |
| `cache_chain_summary()` | [[cache.rs]] | YES (works) |

### TODOs in Rust Code

| Location | TODO | Relevance |
|----------|------|-----------|
| [[sync.rs]]:111 | "Load from database for incremental sync" | Related - incremental sync |
| [[sync.rs]]:276 | "Aggregate from sessions" (tools_used) | Related - enrichment |
| [[sync.rs]]:278 | "Query from git_commits" | Related - enrichment |

## File Locations

| File | Purpose | Status |
|------|---------|--------|
| [[core/src/query.rs]] | Query engine with INSERT methods | Has methods, unused |
| [[core/src/daemon/sync.rs]] | Sync orchestrator | Parses but discards |
| [[core/src/storage.rs]] | DB connection (open vs open_rw) | Needs open_rw for writes |
| [[cli/src/context_os_events/index/chain_graph.py]] | Python persist_chains() | Reference implementation |
| [[cli/src/context_os_events/db/schema.sql]] | Canonical schema (missing chains tables) | Reference |

## Test State

- **Rust tests:** 238 passing (but don't test DB persistence from daemon)
- **DB state:** Stale since Jan 18 [VERIFIED: query output timestamps]
- **Symptom:** `tastematter query flex --time 7d` returns 0 results

### Verification Commands for Next Agent

```bash
# Verify DB is stale (should show Jan 18 or earlier)
tastematter query flex --time 90d --limit 3 --format json

# Run daemon sync (will show counts but not persist)
tastematter daemon once

# Check DB file timestamp (should be unchanged after daemon once)
ls -la ~/.context-os/context_os_events.db

# Run tests (all pass but don't cover this gap)
cd apps/tastematter/core && cargo test
```

## Evidence Summary

### The 587K Tool Uses That Went Nowhere

```
parse-sessions output (2026-01-30):
├── Sessions found: 1,122
├── Tool uses parsed: 587,362
├── File events inserted: 0  ← THE BUG
└── DB file: unchanged
```

[VERIFIED: Bash command output during debugging session]

### Python Reference Implementation

```python
# cli/src/context_os_events/index/chain_graph.py:422-495
def persist_chains(db, chains: Dict[str, Chain]) -> Dict[str, int]:
    """Persist chain graph to database."""
    for chain in chains.values():
        # INSERT INTO chains (...) VALUES (...)
        db.execute("INSERT OR REPLACE INTO chains ...")
        # INSERT INTO chain_graph (...) VALUES (...)
        db.execute("INSERT OR REPLACE INTO chain_graph ...")
    db.commit()
```

[VERIFIED: [[chain_graph.py]]:422-495]

### Rust Sync Flow (what actually happens)

```rust
// daemon/sync.rs:45-79
pub fn run_sync(config: &DaemonConfig) -> Result<SyncResult, String> {
    // 1. Git sync - returns counts, discarded
    let git_result = sync_git(config, &mut result);

    // 2. Session parsing - returns Vec<String>, discarded
    let _session_ids = sync_sessions_phase(&claude_dir, config, &mut result);

    // 3. Chain building - returns HashMap, passed to enrichment only
    let chains = build_chains_phase(&claude_dir, &mut result);

    // 3.5 Intelligence enrichment - THIS ACTUALLY WRITES
    enrich_chains_phase(&chains, &mut result);  // ← Only thing that persists

    // 4. Inverted index - returns counts, discarded
    build_index_phase(&claude_dir, chains.as_ref(), &mut result);

    Ok(result)  // Returns stats but nothing was persisted except chain names
}
```

[VERIFIED: [[sync.rs]]:45-79]

## For Next Agent

**Context Chain:**
- Previous: [[45_2026-01-29_CHAIN_SUMMARY_PRACTICAL_TESTS_AND_DISTRIBUTION_STRATEGY]] (feature complete, distribution deferred)
- This package: DATABASE WRITE PATH BUG IDENTIFIED - Rust daemon parses but doesn't persist
- Next action: Wire INSERT calls in sync.rs to actually persist parsed data

**Start here:**
1. Read [[core/src/daemon/sync.rs]] lines 99-150 (sync_sessions_phase)
2. Read [[core/src/query.rs]] lines 1025-1100 (INSERT methods)
3. Understand the gap: sync_sessions_phase returns data, never calls insert
4. Wire: Add engine.insert_session() call in sync_sessions_phase

**The Fix Pattern:**

```rust
// In sync_sessions_phase(), after sync_sessions() returns:
fn sync_sessions_phase(...) -> Vec<String> {
    match sync_sessions(claude_dir, &options, &existing_sessions) {
        Ok((summaries, _parse_result)) => {
            result.sessions_parsed = summaries.len() as i32;

            // NEW: Actually persist to database
            for summary in &summaries {
                let session_input = SessionInput::from(summary);
                if let Err(e) = engine.insert_session(&session_input).await {
                    result.errors.push(format!("Insert error: {}", e));
                }
            }

            summaries.iter().map(|s| s.session_id.clone()).collect()
        }
        // ...
    }
}
```

**Do NOT:**
- Assume the fix is simple - need to also handle chains and chain_graph tables
- Skip creating missing tables (chains, chain_graph not in schema.sql)
- Forget to open DB in read-write mode (`open_rw()` not `open()`)
- Overcomplicate - the INSERT methods already exist and work (tested)

**Key Insight:**
The Rust port is architecturally sound. The gap is **wiring** - connecting existing components. The INSERT methods exist, the parsing works, the queries work. The missing piece is ~100 lines to connect parse output to database inserts.

[VERIFIED: INSERT methods exist at [[query.rs]]:1025-1097, parse methods work, only wiring is missing]

---

**Package Created:** 2026-01-30
**Session Duration:** ~2 hours (debugging + context enumeration)
**Key Finding:** "100% complete" Rust port missing critical Phase 8.5 database persistence
