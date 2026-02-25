---
title: "Tastematter Context Package 47"
package_number: 47
date: 2026-02-25
status: current
previous_package: "[[46_2026-02-24_TRAIL_FIRST_PUSH_AND_FEATURE_FLAGS]]"
related:
  - "[[canonical/08_GLOBAL_TRAIL_SPEC]]"
  - "[[core/src/trail/pull.rs]]"
  - "[[core/src/trail/push.rs]]"
  - "[[core/src/daemon/sync.rs]]"
  - "[[core/Cargo.toml]]"
tags:
  - context-package
  - tastematter
  - global-trail
  - feature-flags
---

# Tastematter - Context Package 47: Trail Pull + Auto Sync

## Executive Summary

Completed the global trail round-trip: built `trail::pull` in Rust mirroring push pattern, wired auto-pull into daemon sync (Phase 6), added `trail pull` CLI subcommand, added UNIQUE index on `file_access_events` for data integrity. First pull: 24,043 rows. Incremental sync verified (second pull: 0 new rows). 502 tests passing with trail feature.

## What Was Done This Session

### 1. Epistemic Grounding Audit (3 parallel agents)

**5 assumptions verified STRONG before implementation:**

| Assumption | Grade | Finding |
|------------|-------|---------|
| Pull endpoint format | STRONG | `GET /trail/pull?since=&tables=` → `{tables: {...}, synced_at}` |
| D1 vs local schema | STRONG | D1 has `source_machine` + `synced_at` on all 8 tables; local doesn't |
| `_metadata` table | STRONG | Exists locally (storage.rs:272), no `last_trail_pull` yet |
| Path denormalization | STRONG | Store normalized (Unix) — no denormalization needed |
| Push TableDef columns | STRONG | push.rs:25-152 is the reference contract for both directions |

**Critical schema finding:** `file_access_events` had NO UNIQUE constraint locally. `file_edges` already had one (storage.rs:269). User caught that this was a general data integrity issue, not trail-specific. Fixed at the right level.

### 2. Data Integrity: UNIQUE Index on file_access_events

**Problem:** `file_access_events` was the only auto-increment table without a UNIQUE constraint. `file_edges` already had `idx_fe_unique`. This is a data integrity gap regardless of trail.

**Fix (two places):**
- CREATE TABLE block (storage.rs:249): `CREATE UNIQUE INDEX IF NOT EXISTS idx_fae_unique ON file_access_events(session_id, file_path, tool_name, sequence_position)` [VERIFIED: storage.rs]
- Migrations vec (storage.rs:305): Same index for existing databases [VERIFIED: storage.rs]

**Verified safe:** Existing daemon code (query.rs:1847) already uses DELETE-then-INSERT pattern, so the UNIQUE constraint doesn't break re-processing. [VERIFIED: query.rs:1847 `DELETE FROM file_access_events WHERE session_id = ?`]

### 3. Shared TABLES Contract

Made `TableDef` struct and `TABLES` const `pub` in push.rs so pull.rs can import them. This is the single source of truth for the column contract between local SQLite and D1. [VERIFIED: push.rs:16 `pub struct TableDef`, push.rs:25 `pub const TABLES`]

### 4. trail::pull Module (pull.rs, ~230 lines)

**Architecture mirrors push.rs:**
1. Check `is_configured()` — skip if not
2. Read `last_trail_pull` from `_metadata` (default: epoch)
3. HTTP GET `/trail/pull?since={last_pull}&tables=...` with CF Access headers, 60s timeout
4. Parse response JSON: `{tables: {name: [{col: val}]}, synced_at}`
5. For each table, for each row: filter to `TableDef.columns` only (strips `source_machine`, `synced_at`, `id`)
6. Upsert: INSERT OR REPLACE (natural PK tables), INSERT OR IGNORE (auto-increment)
7. Write `last_trail_pull = synced_at` to `_metadata`

**Key design: column filtering via TableDef.columns.** Pull receives all D1 columns but only INSERTs the subset defined in push.rs TABLES. This automatically strips D1-only columns without maintaining a separate exclusion list.

**bind_json_value() helper:** Maps serde_json::Value to sqlx bind types (null, bool, i64, f64, string, JSON string for arrays/objects).

### 5. Daemon Integration (Phase 6)

Auto-pull added as Phase 6 in `run_sync()` (after Phase 5 push). Feature-gated with `#[cfg(feature = "trail")]`. Same pattern as push: check `is_configured()`, call `pull_trail()`, aggregate into `SyncResult`.

- `trail_rows_pulled: i32` added to `SyncResult` (ungated — stays 0 without feature)
- Display format updated in both `daemon once` and `daemon start` to show pulled rows

### 6. CLI Subcommand

Added `Pull` variant to `TrailCommands` enum. Handler opens RW database (`Database::open_rw()`) since pull writes to local SQLite. Read-only engine from main.rs wouldn't work. [VERIFIED: first attempt failed with "attempt to write a readonly database", fixed by opening RW connection]

### 7. Live Verification

| Test | Result |
|------|--------|
| `trail pull` (first, epoch since) | 24,043 rows pulled |
| `trail pull` (second, incremental) | No new rows (since tracked) |
| `daemon once` | Auto-push + auto-pull, 0 errors |
| `trail --help` | Shows status, push, pull |
| `trail status` | D1 counts match |

## Files Modified

| File | Change |
|------|--------|
| `core/src/trail/pull.rs` | **NEW** — pull_trail(), TrailPullResult, bind_json_value(), 9 tests |
| `core/src/trail/mod.rs` | Added `pub mod pull;` |
| `core/src/trail/push.rs` | Made `TableDef` + `TABLES` pub (shared contract) |
| `core/src/daemon/sync.rs` | Added `trail_rows_pulled` to SyncResult, Phase 6 auto-pull, import |
| `core/src/main.rs` | Added `Pull` to TrailCommands, RW handler, display in daemon output |
| `core/src/storage.rs` | UNIQUE index on file_access_events (CREATE TABLE + migration) |

## Test State

- **Rust (without trail):** 473 passing, 1 env-specific failure (`from_env_returns_none_without_api_key` — ANTHROPIC_API_KEY set in env)
- **Rust (with trail):** 502 passing (+9 pull tests), same 1 env failure
- **D1 data:** 24K+ rows across 7 tables, incremental sync working

## Current State

**Trail is fully bidirectional for personal use:**
- Auto-push in daemon sync (Phase 5) — laptop → D1
- Auto-pull in daemon sync (Phase 6) — D1 → local SQLite
- Manual push/pull/status via `tastematter trail {status,push,pull}`
- Feature-gated out of public release binary
- Incremental sync via `_metadata.last_trail_pull`
- 24K+ rows synced, idempotent both directions

## Jobs To Be Done (Next Session)

### Must-do (VPS round-trip verification)
1. [ ] **VPS setup** — clone repo, `cargo build --release --features trail`, write config.yaml with same CF Access credentials
2. [ ] **Verify round-trip** — laptop pushes → VPS pulls → VPS queries show laptop's data
3. [ ] **Verify VPS push** — VPS generates its own sessions → pushes to D1 → laptop pulls → laptop sees VPS data

### Future (not this sprint)
4. [ ] Incremental push (track `last_trail_push` timestamp — currently pushes all rows every time)
5. [ ] Custom domain `trail.tastematter.dev` + CF Access policy
6. [ ] Team D1 (multi-user push to same database)

## For Next Agent

**Context Chain:**
- Previous: [[46_2026-02-24_TRAIL_FIRST_PUSH_AND_FEATURE_FLAGS]] (push, CLI, feature flags)
- This package: Pull module, auto-pull, data integrity fix, full bidirectional sync
- Next action: VPS setup and round-trip verification

**Start here:**
1. Read this package
2. `cd core && cargo build --release --features trail`
3. `./target/release/tastematter trail status` (verify D1 is live)
4. `./target/release/tastematter trail pull` (verify incremental — should show 0 new rows)
5. SSH to VPS, clone, build, test

**Key insight:** Pull reuses push.rs TABLES as the column contract. D1-only columns (source_machine, synced_at) are stripped automatically by only inserting TableDef.columns. No separate exclusion list to maintain.
