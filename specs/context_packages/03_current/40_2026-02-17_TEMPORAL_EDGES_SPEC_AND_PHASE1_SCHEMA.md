---
title: "Tastematter Context Package 40"
package_number: 40
date: 2026-02-17
status: current
previous_package: "[[39_2026-02-17_TEMPORAL_SIGNAL_VALIDATION_PASS]]"
related:
  - "[[specs/canonical/19_TEMPORAL_EDGES_SPEC.md]]"
  - "[[specs/canonical/07_CLAUDE_CODE_DATA_MODEL_V2.md]]"
  - "[[core/src/storage.rs]]"
  - "[[core/src/daemon/sync.rs]]"
  - "[[core/src/capture/jsonl_parser.rs]]"
tags:
  - context-package
  - tastematter
  - temporal-edges
  - specification
  - schema-migration
---

# Tastematter - Context Package 40

## Executive Summary

Wrote canonical spec `19_TEMPORAL_EDGES_SPEC.md` (4 phases, ~24 tests) using epistemic grounding, feature planning, data engineering, and devops perspectives. Implemented Phase 1 schema migration (2 new tables + 5 indexes + 5 tests) in `storage.rs`. Tests compile but were NOT yet run to confirm passing — user interrupted for context package before test execution completed.

## What Was Done This Session

### 1. Epistemic Context Grounding

Read and confirmed understanding of the Claude Code data model from canonical spec V2:

- **7 top-level JSONL record types:** assistant, user, progress, system, summary, file-history-snapshot, queue-operation [VERIFIED: [[specs/canonical/07_CLAUDE_CODE_DATA_MODEL_V2.md]]:425-463]
- **Each API response = multiple JSONL records** — one per content block. `[thinking, text, tool_use, tool_use]` = 4 records. Each `tool_use` gets its own record with unique timestamp. [VERIFIED: V2 spec Section 2.2, line 219]
- **3-source extraction in parser:** Source 1 assistant `tool_use` (~190K), Source 2 user `toolUseResult` (~4K), Source 3 `file-history-snapshot` (~2K) [VERIFIED: [[core/src/capture/jsonl_parser.rs]]:1-8]
- **Validation confirmed (pkg #39):** 0% multi-tool records, ms precision, monotonic timestamps, 62 avg R→E patterns/session

### 2. Domain Knowledge Deep Read

Read every file that will be modified, in full:

| File | Lines Read | Key Finding |
|------|-----------|-------------|
| `storage.rs` (full, 1222 lines) | ensure_schema() at line 130, SCHEMA_SQL at 132-242, 7 existing tables, ALTER migrations at 253-272 | `IF NOT EXISTS` pattern, schema version in `_metadata` |
| `sync.rs` (full, 1282 lines) | 5-phase sync pipeline: git→sessions→chains→intel→index. sync_sessions_phase at 140-201 upserts sessions. ToolUse data counted (line 922) but discarded after aggregation. | Graceful degradation pattern (intel enrichment). Event persistence slots between session upsert and chain building. |
| `jsonl_parser.rs` (key sections) | ToolUse struct at 34-50, aggregate_session at 559-695 (where ordering destroyed), sync_sessions at 856-945 | Data already parsed — just needs to be persisted instead of discarded |
| `query.rs:1350-1454` | 5-phase query_context pipeline. Phase 2 = co-access. Phase 4 = builders. | Edge query slots into Phase 2, pattern builder into Phase 4 |
| `types.rs:658-784` | WorkCluster, Continuity, ExecutiveSummary, etc. All use Option + skip_serializing_if. | New fields via Option<T> = zero breaking changes |
| `context_restore.rs:110-187, 718-760` | build_work_clusters, build_continuity — pure transforms | Adding edges parameter is clean |

### 3. Canonical Spec Written

Created `specs/canonical/19_TEMPORAL_EDGES_SPEC.md` — comprehensive 4-phase specification:

| Phase | What | Files | Complexity | Tests |
|-------|------|-------|------------|-------|
| 1 | Schema migration (2 tables, 5 indexes) | storage.rs | Low | 5 |
| 2 | Event persistence (stop discarding ToolUse) | jsonl_parser.rs, sync.rs, query.rs | Medium | 4 |
| 3 | Edge extraction (5 types + 3 noise filters) | NEW file_edges.rs, mod.rs, sync.rs | Medium-High | 10+ |
| 4 | Context restore integration | types.rs, query.rs, context_restore.rs | Medium | 6+ |

Key spec design decisions:
- `access_type TEXT` not `is_read BOOLEAN` — 3 values (read/write/search), single column, extensible
- `sequence_position INTEGER` — explicit ordering cheaper than timestamp ORDER BY, defensive against edge cases
- `confidence REAL` pre-computed — avoids division at query time
- `UNIQUE INDEX (source_file, target_file, edge_type)` — enables INSERT OR REPLACE for idempotent rebuild
- Backward compatible via `Option<WorkPattern>` + `skip_serializing_if`

### 4. Observability Design

Applied Charity Majors' wide events + request-scoped context:

- One wide event per sync phase, not printf-per-operation
- `session_id` as natural correlation key (flows through entire pipeline)
- `debug!` at phase boundaries, `info!` for unexpected conditions, `error!` for actual errors
- NOT logging: individual tool use inserts (190K = noise), per-edge extraction details
- `SyncResult` to gain `edges_extracted: i32` field
- `_metadata` stores `last_edge_extraction` for incremental tracking

### 5. Phase 1 Implementation (Schema Migration)

**storage.rs changes:**
- Added Layer 8: `file_access_events` table (7 columns + 3 indexes) at SCHEMA_SQL
- Added Layer 9: `file_edges` table (10 columns + 4 indexes including UNIQUE) at SCHEMA_SQL
- Updated schema version from `2.2` to `2.3`
- Added 5 new tests:
  - `test_ensure_schema_creates_temporal_tables` — verifies both tables exist with correct columns
  - `test_file_access_events_insert_and_query` — insert 5 events, query back ordered by sequence_position
  - `test_file_edges_unique_constraint` — INSERT OR REPLACE deduplicates on (source, target, type)
  - `test_temporal_tables_preserved_across_ensure_schema` — data survives re-run of ensure_schema
  - `test_schema_version_updated_to_2_3` — verifies _metadata has "2.3"

**sync.rs change:**
- Updated existing `test_fresh_install_creates_db_and_schema` assertion from "2.2" to "2.3"

**Compilation:** `cargo check` passes [VERIFIED: output "Finished `dev` profile" this session]

**Tests:** NOT YET RUN. `cargo test` was interrupted by user before completion. Tests need to be run next session.

## What Changed Since Package 39

| Item | Status |
|------|--------|
| Empirical validation (pkg 39) | DONE — 7/7 PASS |
| Canonical spec | **DONE** — 19_TEMPORAL_EDGES_SPEC.md written |
| Epistemic grounding (V2 data model) | **DONE** — confirmed understanding |
| Observability design | **DONE** — wide events, correlation IDs, NOT per-record logging |
| Phase 1 schema migration | **IN PROGRESS** — code written, compiles, tests not yet verified passing |
| Phase 2 event persistence | NOT STARTED |
| Phase 3 edge extraction | NOT STARTED |
| Phase 4 context restore integration | NOT STARTED |

## Local Problem Set

### Completed This Session

- [X] Re-read V2 canonical data model spec (confirmed 7 record types, one-record-per-content-block) [VERIFIED: [[specs/canonical/07_CLAUDE_CODE_DATA_MODEL_V2.md]]:219]
- [X] Read all 6 files that will be modified (storage.rs, sync.rs, jsonl_parser.rs, query.rs, types.rs, context_restore.rs) [VERIFIED: this package]
- [X] Wrote canonical spec 19_TEMPORAL_EDGES_SPEC.md [VERIFIED: file exists]
- [X] Designed observability approach (wide events, not printf) [VERIFIED: this package]
- [X] Implemented Phase 1 schema: 2 tables, 5 indexes in storage.rs [VERIFIED: `cargo check` passes]
- [X] Updated schema version to 2.3 [VERIFIED: storage.rs SCHEMA_SQL]
- [X] Wrote 5 new tests for Phase 1 [VERIFIED: storage.rs test module]
- [X] Updated sync.rs test assertion from "2.2" to "2.3" [VERIFIED: sync.rs:1214]

### Jobs To Be Done (Next Session)

1. [ ] **RUN PHASE 1 TESTS** — `cargo test "temporal\|file_access\|file_edges\|schema_version_updated_to_2_3" -- --test-threads=2`
   - The tests compile but were NOT executed this session
   - Success criteria: all 5 new tests pass + zero regressions in existing 330+ tests
   - If any fail, fix before proceeding

2. [ ] **Full test suite regression check** — `cargo test -- --test-threads=2`
   - Must verify the schema version change from 2.2→2.3 doesn't break anything
   - Key concern: the sync.rs test `test_fresh_install_creates_db_and_schema` was updated to expect "2.3"

3. [ ] **Phase 2: Event persistence** — follow spec Section "Phase 2: Parser Integration"
   - Add `ParsedSessionData` struct pairing summary with tool uses
   - Modify `sync_sessions()` to return tool uses alongside summaries
   - Add `insert_file_access_events()` to QueryEngine
   - Modify `sync_sessions_phase()` to persist events
   - 4 new tests

4. [ ] **Phase 3: Edge extraction module** — follow spec Section "Phase 3"
   - New file: `core/src/index/file_edges.rs`
   - 5 edge types, 3 noise filters
   - Batch extraction during daemon sync
   - 10+ new tests

5. [ ] **Phase 4: Context restore integration** — follow spec Section "Phase 4"
   - New types: `FileEdge`, `WorkPattern`, `IncompleteSequence`
   - Enhance `WorkCluster` and `Continuity`
   - New builder: `build_work_patterns()`
   - 6+ new tests

## File Locations

| File | Purpose | Status |
|------|---------|--------|
| [[specs/canonical/19_TEMPORAL_EDGES_SPEC.md]] | Canonical 4-phase spec for temporal edges | CREATED this session |
| [[core/src/storage.rs]] | Database schema — 2 new tables added | MODIFIED this session |
| [[core/src/daemon/sync.rs]] | Schema version test assertion updated | MODIFIED this session (1 line) |
| [[specs/canonical/07_CLAUDE_CODE_DATA_MODEL_V2.md]] | Canonical JSONL data model | Reference (read for grounding) |
| [[core/src/capture/jsonl_parser.rs]] | Rust JSONL parser — ToolUse struct | Reference (will be modified Phase 2) |
| [[core/src/query.rs]] | Query engine — query_context pipeline | Reference (will be modified Phase 4) |
| [[core/src/context_restore.rs]] | Context restore builders | Reference (will be modified Phase 4) |
| [[core/src/types.rs]] | API types — WorkCluster, Continuity | Reference (will be modified Phase 4) |

## Test State

- **Rust core:** `cargo check` passes [VERIFIED: this session]
- **New tests:** 5 written, NOT YET EXECUTED (interrupted before test run)
- **Existing tests:** 330+ — expected passing but not verified this session
- **Risk:** Schema version change 2.2→2.3 could affect `test_fresh_install_creates_db_and_schema` — already updated assertion

### Test Commands for Next Agent

```bash
# FIRST: Run the new Phase 1 tests specifically
cd apps/tastematter/core && cargo test storage::tests::test_ensure_schema_creates_temporal -- --test-threads=1
cd apps/tastematter/core && cargo test storage::tests::test_file_access_events -- --test-threads=1
cd apps/tastematter/core && cargo test storage::tests::test_file_edges_unique -- --test-threads=1
cd apps/tastematter/core && cargo test storage::tests::test_temporal_tables_preserved -- --test-threads=1
cd apps/tastematter/core && cargo test storage::tests::test_schema_version_updated_to_2_3 -- --test-threads=1

# THEN: Full regression suite
cd apps/tastematter/core && cargo test -- --test-threads=2
```

## For Next Agent

**Context Chain:**
- Package 38: Temporal edges design thesis from CodeGraph teardown
- Package 39: Empirical validation PASS — 7/7 sessions, gate cleared
- **Package 40 (this): Spec written + Phase 1 schema implemented (NOT YET TESTED)**
- Next: Run tests, then Phase 2 event persistence

**Start here:**
1. Read this context package (you're doing it now)
2. Read [[specs/canonical/19_TEMPORAL_EDGES_SPEC.md]] for full 4-phase spec with code examples
3. Run `cargo test -- --test-threads=2` to verify Phase 1 tests pass and no regressions
4. If tests pass → proceed to Phase 2 (event persistence per spec)
5. If tests fail → fix first, then proceed

**Do NOT:**
- Re-read the V2 data model spec (already confirmed this session — see section 1 above)
- Re-run temporal signal validation (already PASS — pkg 39)
- Run `cargo test` without `--test-threads=2` (crashes VS Code)
- Edit existing context packages (append-only)
- Skip running tests before Phase 2 — the Phase 1 tests were NOT verified

**Key insight:**
The spec uses three principles from the observability skill: (1) wide structured events at phase boundaries, not per-record logging; (2) `session_id` as natural correlation key flowing through the entire pipeline; (3) SyncResult as the wide event carrier. Every design decision in the spec cites a specific source file and line number. Phase boundaries are clean — each phase is independently shippable and testable. [VERIFIED: [[specs/canonical/19_TEMPORAL_EDGES_SPEC.md]]]
