---
title: "Tastematter Context Package 34"
package_number: 34
date: 2026-02-09
status: current
previous_package: "[[33_2026-02-04_PRODUCT_VISION_SEQUENCING_AND_STATE_SYNC]]"
related:
  - "[[specs/canonical/12_CONTEXT_RESTORATION_API_SPEC.md]]"
  - "[[core/src/context_restore.rs]]"
  - "[[core/src/query.rs]]"
  - "[[core/src/types.rs]]"
  - "[[core/src/storage.rs]]"
  - "[[core/src/main.rs]]"
  - "[[core/src/http.rs]]"
  - "[[.github/workflows/staging.yml]]"
tags:
  - context-package
  - tastematter
  - context-restore-api
  - auto-init
  - distribution
---

# Tastematter - Context Package 34

## Executive Summary

Implemented the Context Restoration API (Phase 1) — the first composed query in tastematter. `tastematter context "nickel"` composes all 8 query primitives into a single structured JSON response with 9 sections. Shipped as v0.1.0-alpha.20. Also added fresh-init smoke tests to CI staging pipeline. DB auto-init for fresh machines is planned but not yet implemented.

## What Was Built

### Context Restore API (Phase 1a + 1b)

**New command:** `tastematter context "query" [--time 30d] [--limit 20] [--format json|compact|table]`

**Architecture:**
```
tastematter context "nickel" --format json
         |
    QueryEngine::query_context()            # query.rs:1324
         |
         +--[tokio::join!]-- 5 parallel DB queries (flex, heat, chains, sessions, timeline)
         +--[sequential]---- co-access x5 anchors
         +--[sequential]---- discover_project_context() (filesystem walkdir)
         |
    9 builder functions (pure transforms)   # context_restore.rs
         |
    ContextRestoreResult (JSON)
```

**Key design decisions:**
- First use of `tokio::join!` in codebase — 5 parallel DB queries [VERIFIED: [[query.rs]]:1333-1360]
- Multi-pattern filesystem discovery via walkdir instead of glob (glob hung on large repos) [VERIFIED: [[context_restore.rs]]:484-558]
- Skip dirs: node_modules, .git, target, etc. Max depth 8, cap 50 files [VERIFIED: [[context_restore.rs]]:420-431, 489, 502]
- Tier classification: high (specs, context_packages, CLAUDE.md), medium (docs, state), low (cursor rules) [VERIFIED: [[context_restore.rs]]:435-474]
- All LLM-synthesized fields are `Option<String>` = None — stable schema for Phase 2 Intel [VERIFIED: [[types.rs]] - ExecutiveSummary, WorkCluster, SuggestedRead]
- Content truncation uses `is_char_boundary()` to avoid UTF-8 panics [VERIFIED: [[context_restore.rs]]:547-551]

**Output sections (9 total):**

| Section | Source | Phase |
|---------|--------|-------|
| executive_summary | sessions + heat → status/tempo classification | 1a |
| work_clusters | flex + co-access → PMI-grouped file clusters | 1a |
| suggested_reads | flex + co-access + context files → priority-ranked | 1a |
| timeline | timeline buckets → weekly periods + Jaccard shift detection | 1a |
| insights | heat → abandoned file detection | 1a |
| verification | all counts → receipt_id, files/sessions/pairs | 1a |
| current_state | context files + flex → metrics + evidence (Option) | 1b |
| continuity | context files + chains → pending items, left_off_at (Option) | 1b |
| quick_start | context files → extracted code blocks (Option) | 1b |

**Noise risk noted:** Filesystem discovery casts a wide net. May need per-project config or smarter filtering. Shipped to iterate. [VERIFIED: [[context_restore.rs]]:10-12, 415-417]

### CI Fresh Init Smoke Tests

Added to staging.yml — tests full user journey on clean runners:

1. Remove any existing `~/.context-os/` (simulate fresh machine)
2. `tastematter daemon once` (creates DB + schema)
3. Verify DB file exists
4. `tastematter context "test" --format json` (valid JSON on empty DB)
5. Assert receipt_id exists and status enum is valid

Runs on Windows, Ubuntu, macOS. All passing. [VERIFIED: [[.github/workflows/staging.yml]]:173-221]

### CLAUDE.md Safety Warning

Added `cargo test -- --test-threads=2` requirement and documented why: daemon integration tests spin up full SQLite DBs + parse real JSONL files, causing memory spikes at default parallelism that crash VS Code and Claude Code instances. [VERIFIED: [[CLAUDE.md]]:64, 96]

## Completed This Session

- [X] Context restore types (ContextRestoreInput, ContextRestoreResult, 9 sub-structs) [VERIFIED: [[types.rs]]]
- [X] Builder module context_restore.rs (9 builders + filesystem discovery) [VERIFIED: [[context_restore.rs]]]
- [X] Orchestrator query_context() with tokio::join! [VERIFIED: [[query.rs]]:1324-1425]
- [X] CLI wiring (top-level Context command + table formatter) [VERIFIED: [[main.rs]]:169-185, 1414-1477]
- [X] HTTP endpoint POST /api/query/context [VERIFIED: [[http.rs]]]
- [X] Module declaration in lib.rs [VERIFIED: [[lib.rs]]]
- [X] Clippy clean (0 warnings) [VERIFIED: cargo clippy run 2026-02-09]
- [X] Smoke tests passing (nickel, pixee, tastematter queries) [VERIFIED: structured validation script passed]
- [X] Fresh init smoke tests in CI staging [VERIFIED: [[staging.yml]]:173-221]
- [X] Released v0.1.0-alpha.20 [VERIFIED: latest.txt = v0.1.0-alpha.20]
- [X] CI + Staging + Release all green [VERIFIED: gh run list]

## Bugs Found and Fixed During Smoke Testing

1. **UTF-8 boundary panic** — `content[..4096]` hit middle of multi-byte `✓` character. Fixed with `is_char_boundary()` scan-back. [VERIFIED: [[context_restore.rs]]:547-551]
2. **Glob hang** — `glob::glob("**/specs/**/*.md")` recursed into node_modules/.git/target on large repo. Replaced with walkdir + filter_entry to skip heavy dirs, max_depth(8), 50-file cap. [VERIFIED: [[context_restore.rs]]:488-499]
3. **Clippy clamp** — `min(5).max(1)` flagged as manual_clamp. Changed to `.clamp(1, 5)`. [VERIFIED: [[context_restore.rs]]:166]

## In Progress: DB Auto-Init for Fresh Machines

### Problem

Fresh install → `tastematter context "something"` → error:
```
Database not found at canonical location: ~/.context-os/context_os_events.db
To fix: Run the indexer to create the database...
```

"Run the indexer" is not a command. Example references old binary name `context-os`. User must discover `daemon once` on their own.

### Root Cause

Query path (`main.rs:625` → `storage.rs:344` → `storage.rs:324`) calls `open_default()` → `find_database()` → checks `path.exists()` → fails.

Daemon path (`sync.rs:62-71`) does `create_dir_all` → `open_rw` (mode=rwc) → `ensure_schema`. Daemon self-bootstraps; queries don't.

### Planned Fix (~20 lines)

Add `Database::open_or_create_default()` to `storage.rs`:

```rust
pub async fn open_or_create_default() -> Result<Self, CoreError> {
    let canonical = Self::canonical_path()?;
    if canonical.exists() {
        return Self::open(&canonical).await;
    }
    // Fresh machine: create dir + DB + schema
    if let Some(parent) = canonical.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            CoreError::Config(format!("Could not create database directory: {}", e))
        })?;
    }
    let rw = Self::open_rw(&canonical).await?;
    rw.ensure_schema().await?;
    rw.close().await;
    Self::open(&canonical).await
}
```

Replace `Database::open_default()` with `Database::open_or_create_default()` at `main.rs:625`.

### Verification Plan
```bash
rm -rf ~/.context-os
tastematter context "test" --format json
# Expected: valid JSON with status "unknown", empty clusters
# NOT expected: "Database not found" error
```

## Jobs To Be Done (Next Session)

1. [ ] Implement DB auto-init fix (storage.rs + main.rs, ~20 lines)
   - Success criteria: `tastematter context "test"` works on fresh machine without prior `daemon once`
   - Gate: existing CI fresh init smoke test passes without the `daemon once` step

2. [ ] Update CI smoke test to remove `daemon once` step (proves auto-init works)
   - Success criteria: staging pipeline green with context command running on clean DB

3. [ ] Tag v0.1.0-alpha.21 with auto-init fix

4. [ ] Phase 2 planning: Intel integration for Option<String> fields (one_liner, cluster names, reasons, narrative)
   - Depends on: Intel service architecture decisions

## File Locations

| File | Purpose | Status |
|------|---------|--------|
| [[core/src/context_restore.rs]] | 9 builder functions + filesystem discovery | NEW |
| [[core/src/types.rs]] | ContextRestoreInput/Result + 9 sub-structs | Modified |
| [[core/src/query.rs]] | query_context() orchestrator with tokio::join! | Modified |
| [[core/src/main.rs]] | Context CLI command + table formatter | Modified |
| [[core/src/http.rs]] | POST /api/query/context endpoint | Modified |
| [[core/src/lib.rs]] | pub mod context_restore | Modified |
| [[core/src/storage.rs]] | Auto-init fix target (not yet modified) | Planned |
| [[.github/workflows/staging.yml]] | Fresh init smoke tests | Modified |
| [[CLAUDE.md]] | cargo test --test-threads=2 warning | Modified |

## Test State

- Tests: 306 passing, 1 pre-existing flaky (batch_insert_commits_performance), 4 ignored
- Command: `cargo test -- --test-threads=2` (ALWAYS limit threads)
- CI: green (all platforms)
- Staging: green (including fresh init smoke tests)
- Smoke tests: nickel, pixee, tastematter queries all verified

## For Next Agent

**Context Chain:**
- Previous: [[33_2026-02-04_PRODUCT_VISION_SEQUENCING_AND_STATE_SYNC]]
- This package: Context Restore API shipped (v0.1.0-alpha.20) + auto-init planned
- Next action: Implement DB auto-init fix

**Start here:**
1. Read this package (you're doing it now)
2. Read [[core/src/storage.rs]] lines 50-84 (open), 96-111 (open_rw), 294-347 (canonical_path, find_database, open_default)
3. Implement `open_or_create_default()` per the plan above
4. Change `main.rs:625` from `open_default()` to `open_or_create_default()`
5. Build, clippy, test, smoke test with `rm -rf ~/.context-os && tastematter context "test"`

**Do NOT:**
- Run `cargo test` without `--test-threads=2` (will crash VS Code)
- Use `glob::glob` for filesystem discovery (hangs on large repos — use walkdir)
- Slice strings by byte index without checking `is_char_boundary()` (UTF-8 panic)
- Assume test users have run `daemon once` before querying

**Key insight:**
The deterministic context restore is surprisingly useful without Intel/LLM synthesis. PMI clustering, Jaccard attention shift detection, and filesystem context discovery provide enough structure for agents to restore context. Phase 2 Intel adds labeling (cluster names, one-liners, reasons) — readability, not correctness.
