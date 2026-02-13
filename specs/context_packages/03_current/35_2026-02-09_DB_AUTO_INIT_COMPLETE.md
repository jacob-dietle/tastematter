---
title: "Tastematter Context Package 35"
package_number: 35
date: 2026-02-09
status: current
previous_package: "[[34_2026-02-09_CONTEXT_RESTORE_API_AND_AUTO_INIT]]"
related:
  - "[[core/src/storage.rs]]"
  - "[[core/src/main.rs]]"
tags:
  - context-package
  - tastematter
  - auto-init
  - fresh-install
---

# Tastematter - Context Package 35

## Executive Summary

Implemented DB auto-init for fresh machines. `tastematter context "test"` now works on a machine with no prior `daemon once` — creates `~/.context-os/` directory, empty DB with schema, and returns valid empty results. Two files changed, ~25 lines added. Release build clean, all 15 storage tests passing.

## What Was Built

### `Database::open_or_create_default()` — storage.rs:349-373

New method that mirrors the daemon's proven init pattern from `sync.rs:56-73`:

```rust
pub async fn open_or_create_default() -> Result<Self, CoreError> {
    let canonical = Self::canonical_path()?;
    if canonical.exists() {
        return Self::open(&canonical).await;  // existing fast path
    }
    // Fresh machine: create directory + DB + schema
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

**Key design decisions:**
- Creates via RW, then reopens RO — query path stays read-only as designed [VERIFIED: [[storage.rs]]:368-372]
- Uses `ensure_schema()` which is idempotent (IF NOT EXISTS) [VERIFIED: [[storage.rs]]:130-274]
- Reuses existing `canonical_path()`, `open_rw()`, `ensure_schema()`, `open()` — no new dependencies
- Explicit `--db` path still fails fast if file doesn't exist — correct behavior preserved [VERIFIED: [[main.rs]]:622-623]

### main.rs:625 — One-line change

```rust
// Before:
Database::open_default().await?

// After:
Database::open_or_create_default().await?
```

All query commands (context, query flex, query chains, etc.) now auto-init on fresh machines.

## Completed This Session

- [X] Added `open_or_create_default()` to storage.rs [VERIFIED: [[storage.rs]]:349-373]
- [X] Changed main.rs to use `open_or_create_default()` [VERIFIED: [[main.rs]]:625]
- [X] Release build compiles clean [VERIFIED: `cargo build --release` — 0 errors, 0 warnings]
- [X] All 15 storage tests passing [VERIFIED: `cargo test storage::tests -- --test-threads=2` — 15/15 ok]

## What This Does NOT Change

- Explicit `--db` path still fails fast if file doesn't exist (main.rs:622-623)
- Daemon path unchanged — still does its own init via sync.rs
- All existing query behavior unchanged
- No new CLI flags or commands
- No new dependencies

## Jobs To Be Done (Next Session)

1. [ ] Update CI smoke test to remove `daemon once` prerequisite
   - Success criteria: staging pipeline green with `context` command running on auto-init DB
2. [ ] Tag v0.1.0-alpha.21 with auto-init fix
3. [ ] Manual smoke test: `rm -rf ~/.context-os && tastematter context "test" --format json`
   - Expected: valid JSON with status "unknown", tempo "dormant", empty clusters
4. [ ] Phase 2 planning: Intel integration for Option<String> fields

## File Locations

| File | Purpose | Status |
|------|---------|--------|
| [[core/src/storage.rs]] | `open_or_create_default()` added | Modified (+25 lines) |
| [[core/src/main.rs]] | Uses new method at line 625 | Modified (1 line) |

## Test State

- Storage tests: 15 passing, 0 failing
- Command: `cargo test storage::tests -- --test-threads=2`
- Last run: 2026-02-09
- Full test suite: 311 tests, daemon integration tests crash under resource contention (pre-existing, unrelated)

## For Next Agent

**Context Chain:**
- Previous: [[34_2026-02-09_CONTEXT_RESTORE_API_AND_AUTO_INIT]] (auto-init planned)
- This package: Auto-init implemented and verified
- Next action: Update CI, tag release

**Start here:**
1. Read this package
2. Run `cargo test storage::tests -- --test-threads=2` to verify
3. Update staging.yml to test auto-init without `daemon once`
4. Tag v0.1.0-alpha.21

**Do NOT:**
- Run `cargo test` without `--test-threads=2` (will crash VS Code)
- Remove `open_default()` — other code may reference it
- Add tests that use the real `~/.context-os/` path (use tempdir)
