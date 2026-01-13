# Database Architecture Root Cause Analysis

**Date:** 2026-01-11
**Status:** Critical Architecture Issue Identified
**Severity:** P0 - Data Integrity Issue

---

## Executive Summary

Visual debugging of BUG-002 (sessions showing "No chain") uncovered a **fundamental architecture problem**: three separate SQLite databases exist with no synchronization. Python writes to one location, Rust reads from another, and the migration from underscore to hyphen naming was never completed.

This is not a bug in query logic - it's a **failed migration and undefined data layer**.

---

## Forensic Timeline: How We Got Here

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ TIMELINE OF EVENTS                                                          │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│ PRE-2026-01-08: Original Python Implementation                              │
│ ├── context_os_events/ (underscore) exists                                  │
│ ├── Python CLI indexes JSONL files                                          │
│ ├── Database: context_os_events/data/context_os_events.db                   │
│ └── Single source of truth: Python owns everything                          │
│                                                                             │
│ 2026-01-08 (Package 9): Architecture Analysis                               │
│ ├── Measured 18-second query latency                                        │
│ ├── Root cause: Python loads ENTIRE DB into memory for each query           │
│ ├── Decision: Port query layer to Rust for <100ms performance               │
│ └── Designed context-os-core Rust library                                   │
│                                                                             │
│ 2026-01-08 (Package 11): Directory Reorganization                           │
│ ├── Created apps/context-os/ (hyphen) with new structure:                   │
│ │   ├── cli/    - Python CLI (moved, kept package name)                     │
│ │   ├── core/   - Rust library (NEW)                                        │
│ │   ├── data/   - SQLite database (intended single location)                │
│ │   └── specs/  - Specifications                                            │
│ ├── OLD apps/context_os_events/ (underscore) marked for deletion            │
│ └── ⚠️  BUT: Old directory never actually deleted!                          │
│                                                                             │
│ POST-REORG: Rust Implementation                                             │
│ ├── Rust storage.rs written with MULTIPLE fallback paths:                   │
│ │   1. apps/context_os_events/data/ (OLD, underscore) ← Found FIRST         │
│ │   2. ~/.context-os/                                                       │
│ │   3. apps/context-os/data/ (NEW, hyphen)                                  │
│ ├── Python CLI kept writing to cli/data/ subdirectory                       │
│ └── Result: TWO active databases, no sync, stale data                       │
│                                                                             │
│ 2026-01-11 (TODAY): Bug Discovered                                          │
│ ├── Sessions showing "No chain" in UI                                       │
│ ├── RCA revealed: chain_graph table populated in PYTHON database            │
│ ├── But Rust reads from OLD STALE database (last updated Jan 4)             │
│ └── THREE separate databases discovered                                     │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Current State: The Three-Database Problem

### Architecture Diagram (CURRENT - BROKEN)

```
                           ┌──────────────────────────────────────┐
                           │         CLAUDE CODE                  │
                           │   ~/.claude/projects/.../*.jsonl     │
                           └──────────────────┬───────────────────┘
                                              │
                                              │ Source of truth for sessions
                                              │
                           ┌──────────────────▼───────────────────┐
                           │      PYTHON INDEXER                  │
                           │  context_os_events.cli               │
                           │                                      │
                           │  Commands:                           │
                           │  - parse-sessions                    │
                           │  - build-chains                      │
                           └──────────────────┬───────────────────┘
                                              │
                                              │ Writes to...
                                              │
           ┌──────────────────────────────────┼──────────────────────────────────┐
           │                                  │                                  │
           ▼                                  ▼                                  ▼
┌────────────────────────┐     ┌────────────────────────┐     ┌────────────────────────┐
│   DATABASE #1 (OLD)    │     │   DATABASE #2 (CLI)    │     │   DATABASE #3 (NEW)    │
│                        │     │                        │     │                        │
│ apps/context_os_events │     │ apps/context-os/cli    │     │ apps/context-os/data   │
│      /data/            │     │      /data/            │     │      /                 │
│                        │     │                        │     │                        │
│ Size: 1.8MB            │     │ Size: 1.2MB            │     │ Size: 1.2MB (copied)   │
│ Modified: Jan 4        │     │ Modified: Jan 11       │     │ Modified: Jan 11       │
│                        │     │                        │     │                        │
│ chain_graph: STALE     │     │ chain_graph: FRESH     │     │ chain_graph: FRESH     │
│ claude_sessions: OLD   │     │ claude_sessions: NEW   │     │ (manual copy)          │
└────────────────────────┘     └────────────────────────┘     └────────────────────────┘
           ▲                                                               │
           │                                                               │
           │ Reads from (WRONG!)                                           │ Intended to read
           │                                                               │
           └──────────────────┐                            ┌───────────────┘
                              │                            │
                              │                            │
                    ┌─────────┴────────────────────────────┴─────────┐
                    │                RUST QUERY ENGINE                │
                    │           context-os-core (query.rs)            │
                    │                                                 │
                    │  storage.rs find_database() search order:       │
                    │  1. apps/context_os_events/data/ ← FINDS THIS   │
                    │  2. ~/.context-os/                              │
                    │  3. apps/context-os/data/      ← NEVER REACHED  │
                    └─────────────────────────────────────────────────┘
                                              │
                                              │ Returns stale data
                                              │
                    ┌─────────────────────────▼─────────────────────────┐
                    │               TASTEMATTER UI                       │
                    │           (Shows "No chain" for sessions)          │
                    └────────────────────────────────────────────────────┘
```

### Database Locations Summary

| # | Path | Purpose | Status | Last Modified |
|---|------|---------|--------|---------------|
| 1 | `apps/context_os_events/data/context_os_events.db` | OLD Python location | **STALE** - Rust reads this first | Jan 4 |
| 2 | `apps/context-os/cli/data/context_os_events.db` | Python CLI writes here | **FRESH** - Has new chains | Jan 11 |
| 3 | `apps/context-os/data/context_os_events.db` | Intended unified location | Manually copied | Jan 11 |

### The Bug Manifestation

```
User runs Tastematter:
├── UI requests sessions via HTTP API
├── Rust query engine searches for database
├── Finds OLD database first (underscore path)
├── Reads chain_graph: last indexed Jan 4
├── Sessions from Jan 5-11 have NO chain_id
└── UI shows "No chain" badge ← USER SEES THIS

Meanwhile:
├── Python indexer (when run) writes to CLI's database
├── Fresh chain_graph data exists in cli/data/
├── But Rust never sees it
└── Data split across multiple files
```

---

## Root Causes (Jeff Dean Style)

### RC1: Incomplete Migration

**What happened:** Directory reorg from `context_os_events/` to `context-os/` was planned but old directory was never deleted.

**Why it matters:** Old directory shadows the new one in search path.

**Evidence:** Package 11 line 151: "Can be removed after closing any processes using the database" - but it was NEVER removed.

### RC2: No Canonical Database Path

**What happened:** Multiple search paths in `storage.rs` without a single authoritative location.

**Why it matters:** Any of 5+ paths could be the "database" - undefined behavior.

**Evidence:** `storage.rs:89-102` shows 5 fallback paths with no clear winner.

### RC3: Python/Rust Schema Divergence

**What happened:** Python CLI creates its own `cli/data/` directory instead of using the unified `data/` location.

**Why it matters:** Indexer writes to wrong place, queries read from wrong place.

**Evidence:** Python CLI `init` creates database in working directory, not canonical location.

### RC4: Missing Chain Graph Migration

**What happened:** The `chains` and `chain_graph` tables weren't in the old database schema.

**Why it matters:** Had to manually apply migration; should be automatic.

**Evidence:** `no such table: chains` error when running `build-chains`.

---

## Proposed Architecture (CLEAN)

### Design Principles (Jeff Dean Approved)

1. **Single Source of Truth:** ONE database file, ONE canonical path
2. **Explicit Over Implicit:** No search paths; fail if database not found
3. **Simple Data Flow:** JSONL → Indexer → DB ← Query Engine
4. **Complete Migration:** Remove ALL legacy paths and code

### Architecture Diagram (PROPOSED)

```
                           ┌──────────────────────────────────────┐
                           │         CLAUDE CODE                  │
                           │   ~/.claude/projects/.../*.jsonl     │
                           └──────────────────┬───────────────────┘
                                              │
                                              │ Watched by daemon OR
                                              │ indexed on-demand
                                              │
                           ┌──────────────────▼───────────────────┐
                           │      PYTHON INDEXER                  │
                           │  context-os index                    │
                           │                                      │
                           │  - Parses JSONL                      │
                           │  - Builds chain graph                │
                           │  - Updates indices                   │
                           └──────────────────┬───────────────────┘
                                              │
                                              │ Writes to CANONICAL PATH
                                              │
                           ┌──────────────────▼───────────────────┐
                           │      SINGLE CANONICAL DATABASE       │
                           │                                      │
                           │  ~/.context-os/context_os.db         │
                           │                                      │
                           │  OR (repo-local development):        │
                           │  apps/context-os/data/context_os.db  │
                           │                                      │
                           │  ⚠️  ONE file. ONE location.         │
                           │  ⚠️  Configured via env var or CLI   │
                           └──────────────────┬───────────────────┘
                                              │
                                              │ Read by query engine
                                              │
                    ┌─────────────────────────▼─────────────────────────┐
                    │                RUST QUERY ENGINE                  │
                    │           context-os-core (query.rs)              │
                    │                                                   │
                    │  Database path resolution (in order):             │
                    │  1. --db flag (explicit)                          │
                    │  2. CONTEXT_OS_DB env var                         │
                    │  3. ~/.context-os/context_os.db (default)         │
                    │  4. ❌ NO FALLBACK SEARCH - fail with clear error │
                    └─────────────────────────┬─────────────────────────┘
                                              │
                       ┌──────────────────────┴───────────────────────┐
                       │                                              │
                       ▼                                              ▼
            ┌────────────────────┐                      ┌────────────────────┐
            │   HTTP API         │                      │   Tauri Commands   │
            │ (Development)      │                      │   (Production)     │
            │                    │                      │                    │
            │ localhost:3001     │                      │ Direct linking     │
            └────────────────────┘                      └────────────────────┘
                       │                                              │
                       └──────────────────────┬───────────────────────┘
                                              │
                                              ▼
                           ┌──────────────────────────────────────┐
                           │           TASTEMATTER UI             │
                           │                                      │
                           │  Single consistent view of data      │
                           └──────────────────────────────────────┘
```

### Key Changes

| Current | Proposed | Rationale |
|---------|----------|-----------|
| 5 database search paths | 1 canonical path + explicit override | Eliminates ambiguity |
| Python writes to `cli/data/` | Python writes to canonical path | Single source of truth |
| Old underscore directory exists | **DELETE IT** | Complete the migration |
| Migration not applied | Auto-migrate on startup | Self-healing schema |
| No config for DB path | `CONTEXT_OS_DB` env var + `--db` flag | Explicit control |

---

## Implementation Plan

### Phase 1: Immediate Cleanup (Today)

```bash
# 1. Delete old underscore directory (the shadow)
rm -rf apps/context_os_events/

# 2. Configure Python CLI to use canonical path
# Edit cli/src/context_os_events/db/connection.py

# 3. Remove fallback paths from storage.rs
# Edit core/src/storage.rs
```

### Phase 2: Canonical Path Implementation

```rust
// storage.rs - PROPOSED

impl Database {
    pub fn find_database(explicit_path: Option<&Path>) -> Result<PathBuf, CoreError> {
        // 1. Explicit path (--db flag)
        if let Some(path) = explicit_path {
            if path.exists() {
                return Ok(path.to_path_buf());
            }
            return Err(CoreError::Config(format!(
                "Database not found at specified path: {}", path.display()
            )));
        }

        // 2. Environment variable
        if let Ok(env_path) = std::env::var("CONTEXT_OS_DB") {
            let path = PathBuf::from(&env_path);
            if path.exists() {
                return Ok(path);
            }
            return Err(CoreError::Config(format!(
                "CONTEXT_OS_DB set but file not found: {}", env_path
            )));
        }

        // 3. Default canonical location
        let default_path = dirs::home_dir()
            .map(|h| h.join(".context-os").join("context_os.db"))
            .ok_or_else(|| CoreError::Config(
                "Cannot determine home directory".to_string()
            ))?;

        if default_path.exists() {
            return Ok(default_path);
        }

        // 4. NO FALLBACK - clear error message
        Err(CoreError::Config(format!(
            "Database not found. Expected at: {}\n\
             Run 'context-os init' to create, or set CONTEXT_OS_DB env var.",
            default_path.display()
        )))
    }
}
```

### Phase 3: Python CLI Alignment

```python
# connection.py - PROPOSED

def get_database_path() -> Path:
    """Get canonical database path. Single source of truth."""

    # 1. Environment variable override
    if env_path := os.environ.get("CONTEXT_OS_DB"):
        return Path(env_path)

    # 2. Default canonical location
    return Path.home() / ".context-os" / "context_os.db"

# NO FALLBACK SEARCH. NO CLI-LOCAL DATABASE.
```

---

## Verification Commands

After implementing fixes:

```bash
# Verify single database exists
ls ~/.context-os/context_os.db

# Verify old directories are gone
ls apps/context_os_events/  # Should fail: No such file or directory

# Verify Python uses canonical path
CONTEXT_OS_DB=~/.context-os/context_os.db python -m context_os_events.cli status

# Verify Rust uses canonical path
CONTEXT_OS_DB=~/.context-os/context_os.db ./target/debug/context-os query sessions --time 7d

# Verify chains are populated
./target/debug/context-os query sessions --time 7d --format json | grep -c chain_id
# Should be > 0
```

---

## Lessons Learned

1. **Complete Migrations:** When reorganizing, DELETE old code. "Can be removed later" becomes "never removed."

2. **Single Source of Truth:** ONE database path. If you need flexibility, use explicit configuration (env var, flag), not search paths.

3. **Fail Fast:** When database isn't found, FAIL with a clear message. Don't silently find some other file.

4. **Test the Data Flow:** End-to-end test that indexer writes → query reads, not just individual components.

5. **Architecture Diagrams:** Draw the data flow BEFORE implementation. This mess would have been obvious.

---

## Related Documents

- [[06_CHAIN_LINKAGE_BUG_RCA.md]] - Original bug report that led to this discovery
- [[canonical/03_CORE_ARCHITECTURE.md]] - Core architecture spec (needs update)
- [[context_packages/11_2026-01-08_DIRECTORY_REORG_COMPLETE.md]] - The reorg that started the problem

---

**Report Generated:** 2026-01-11T22:00:00Z
**Analysis Method:** Forensic timeline + architecture tracing
