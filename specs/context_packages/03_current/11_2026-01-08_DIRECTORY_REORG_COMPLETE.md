---
title: "Tastematter Context Package 11"
package_number: 11

migrated_from: "apps/tastematter/specs/context_packages/11_2026-01-08_DIRECTORY_REORG_COMPLETE.md"
status: current
previous_package: "[[10_2026-01-08_IMPLEMENTATION_SPECS_COMPLETE]]"
related:
  - "[[apps/context-os/core/Cargo.toml]]"
  - "[[apps/context-os/core/src/lib.rs]]"
  - "[[specs/implementation/phase_01_core_foundation/SPEC.md]]"
  - "[[specs/implementation/phase_01_core_foundation/CONTRACTS.rs]]"
tags:
  - context-package
  - tastematter
  - context-os-core
---

# Tastematter - Context Package 11

## Executive Summary

**Major directory reorganization complete.** Renamed `apps/context_os_events/` to `apps/context-os/` with Jeff Dean-style clean structure: `cli/` (Python), `core/` (Rust library skeleton), `data/` (SQLite), `specs/`. Rust crate skeleton created with Cargo.toml and module files. Ready to implement Phase 1 Core Foundation.

## Global Context

### Architecture Overview

Building `context-os-core` Rust library to replace 18-second Python CLI queries with <100ms direct SQLite access.

```
apps/context-os/           # NEW unified structure
├── cli/                   # Python CLI (was context_os_events)
│   └── src/context_os_events/
├── core/                  # Rust library (NEW)
│   ├── Cargo.toml
│   └── src/{lib,error,types,storage,query}.rs
├── data/                  # SQLite database (1.8MB)
│   └── context_os_events.db
└── specs/                 # Specifications
```

### Key Design Decisions

1. **Option C reorganization** - Library lives with its database [VERIFIED: ls apps/context-os/]
2. **Python package name preserved** - `context_os_events` inside cli/src/ for import compatibility
3. **Rust crate skeleton ready** - All module files created with placeholders

## Local Problem Set

### Completed This Session

- [X] Loaded context from package 10 via /context-foundation
- [X] Explored database schema (11 tables, 27 indexes, schema v2.2) [VERIFIED: exploration agent]
- [X] Reorganized `context_os_events/` → `context-os/` with cli/core/data/specs structure [VERIFIED: ls apps/context-os/]
- [X] Created Rust crate skeleton with Cargo.toml and src/ files [VERIFIED: ls apps/context-os/core/src/]

### In Progress

- [ ] Phase 1 Core Foundation implementation
  - Current state: Crate structure created, module files are placeholders
  - Next: Implement from CONTRACTS.rs using TDD
  - Evidence: [VERIFIED: apps/context-os/core/src/lib.rs exists]

### Jobs To Be Done (Next Session)

1. [ ] Implement error.rs (CoreError type) - Copy from CONTRACTS.rs
2. [ ] Implement types.rs (QueryFlexInput, QueryResult, FileResult, etc.) - Copy from CONTRACTS.rs
3. [ ] Implement storage.rs (Database connection pool with sqlx)
4. [ ] Write failing tests for query_flex (TDD RED phase)
5. [ ] Implement query.rs - query_flex function (TDD GREEN phase)
6. [ ] Implement remaining queries: query_chains, query_timeline, query_sessions
7. [ ] Verify all tests pass and latency <100ms

## File Locations

| File | Purpose | Status |
|------|---------|--------|
| [[apps/context-os/core/Cargo.toml]] | Rust package definition | Created |
| [[apps/context-os/core/src/lib.rs]] | Library entry point | Skeleton |
| [[apps/context-os/core/src/error.rs]] | CoreError type | Placeholder |
| [[apps/context-os/core/src/types.rs]] | Data types | Placeholder |
| [[apps/context-os/core/src/storage.rs]] | Database connection | Placeholder |
| [[apps/context-os/core/src/query.rs]] | Query functions | Placeholder |
| [[apps/context-os/data/context_os_events.db]] | SQLite database | 1.8MB, verified |
| [[specs/implementation/phase_01_core_foundation/SPEC.md]] | Implementation spec | Reference |
| [[specs/implementation/phase_01_core_foundation/CONTRACTS.rs]] | Type contracts | Reference |
| [[specs/implementation/phase_01_core_foundation/TESTS.md]] | TDD test plan | Reference |

## Database Schema Summary

Key tables for queries (from exploration):

| Table | Purpose | Key Columns |
|-------|---------|-------------|
| `file_conversation_index` | File ↔ Session index | file_path, session_id, chain_id, access_count |
| `chains` | Chain metadata | chain_id, session_count, files_json, files_bloom |
| `claude_sessions` | Session data | session_id, started_at, files_read, tools_used |
| `chain_graph` | Chain structure | session_id, parent_session_id, position_in_chain |

Database path: `apps/context-os/data/context_os_events.db`

## Test State

- No Rust tests yet (crate skeleton only)
- Python CLI tests in `apps/context-os/cli/tests/` (not run this session)

### Test Commands for Next Agent
```bash
# Verify Rust crate compiles
cd apps/context-os/core && cargo build

# Run Rust tests (after implementation)
cd apps/context-os/core && cargo test

# Verify database exists
ls -la apps/context-os/data/context_os_events.db
```

## For Next Agent

**Context Chain:**
- Previous: [[10_2026-01-08_IMPLEMENTATION_SPECS_COMPLETE]] - Spec writing complete
- This package: Directory reorganization complete, crate skeleton ready
- Next action: Implement Rust modules from CONTRACTS.rs using TDD

**Start here:**
1. Run `/context-foundation` to load this package
2. Read [[specs/implementation/phase_01_core_foundation/CONTRACTS.rs]] for type definitions
3. Read [[specs/implementation/phase_01_core_foundation/TESTS.md]] for TDD test plan
4. Run: `cd apps/context-os/core && cargo build` to verify skeleton compiles

**Implementation order (TDD):**
1. error.rs - CoreError enum with thiserror
2. types.rs - Copy types from CONTRACTS.rs exactly
3. storage.rs - Database struct with sqlx connection pool
4. Write failing tests (RED)
5. query.rs - Implement query_flex first (GREEN)
6. Remaining queries: query_chains, query_timeline, query_sessions

**Do NOT:**
- Edit old `context_os_events/` directory (deprecated, may have locked files)
- Change Python package name (must stay `context_os_events` for compatibility)
- Skip TDD - write failing tests before implementation

**Key insight:**
The database schema uses `file_conversation_index` table (not `file_accesses` as originally assumed in SPEC.md). Verify column names match actual schema when implementing queries.
[VERIFIED: exploration agent schema analysis]

## Cleanup Note

Old `apps/context_os_events/` directory still exists with locked files (.venv/, data/). Can be removed after closing any processes using the database:
```bash
rm -rf apps/context_os_events
```
