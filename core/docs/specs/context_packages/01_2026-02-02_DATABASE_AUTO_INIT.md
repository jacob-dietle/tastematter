---
title: "Tastematter Core Context Package 01"
package_number: 01
date: 2026-02-02
status: current
previous_package: "[[00_2026-02-01_DATABASE_WRITE_PATH_FIX]]"
related:
  - "[[core/src/storage.rs]]"
  - "[[core/src/daemon/sync.rs]]"
  - "[[core/src/main.rs]]"
tags:
  - context-package
  - tastematter
  - database-init
  - fresh-install
---

# Tastematter Core - Context Package 01

## Executive Summary

**In Progress:** Implementing `Database::ensure_schema()` to auto-create tables on fresh install. Tests pass (257 total), but main.rs refactoring has type errors. The daemon commands need to be handled BEFORE the database pre-check to allow fresh installs to work.

## Global Context

### Problem Statement

Fresh CLI installs fail on every command with "Database not found" error:
- No `init` command exists
- No way to create database schema
- 100% of new users blocked

### Solution Design

1. **`Database::ensure_schema()`** - Creates all required tables using `CREATE TABLE IF NOT EXISTS`
2. **Directory creation** - `fs::create_dir_all()` for `~/.context-os/`
3. **Early daemon handling** - Daemon commands handled BEFORE database pre-check

### Architecture

```
Fresh Install Flow:
─────────────────────────────────────────────────────
User runs `tastematter daemon once`
       │
       ▼
Daemon commands handled EARLY (bypass DB pre-check)
       │
       ▼
fs::create_dir_all(~/.context-os/)
       │
       ▼
Database::open_rw()  ←── Creates empty file (?mode=rwc)
       │
       ▼
Database::ensure_schema()  ←── Creates all tables
       │
       ▼
run_sync() proceeds normally
─────────────────────────────────────────────────────
```

## Local Problem Set

### Completed This Session

- [X] Added `ensure_schema()` method to storage.rs [VERIFIED: [[storage.rs]]:114-223]
- [X] Added 3 TDD tests for ensure_schema [VERIFIED: [[storage.rs]]:553-634]
  - `test_ensure_schema_creates_tables_on_fresh_db`
  - `test_ensure_schema_is_idempotent`
  - `test_ensure_schema_preserves_existing_data`
- [X] Wired `ensure_schema()` call into sync.rs after `open_rw()` [VERIFIED: [[sync.rs]]:63-70]
- [X] Added `fs::create_dir_all()` for database directory [VERIFIED: [[sync.rs]]:61-64]
- [X] Updated CLI wrapper to point to correct binary [VERIFIED: tastematter.cmd fixed]
- [X] Pushed database write path fix to remote [VERIFIED: git push b1c6fce]

### In Progress

- [ ] **main.rs refactoring** - Move daemon commands before database pre-check
  - Current state: Type errors in daemon command handling
  - Blocker: API mismatches between new code and existing platform methods
  - Evidence: `cargo build` shows 8 errors

### Errors to Fix

```
error[E0609]: no field `interval_seconds` on type `SyncConfig`
  → Use `interval_minutes` instead

error[E0599]: no method named `check_daemon_status` found
  → Use platform.status() instead

error[E0560]: struct `InstallConfig` has no field named `interval_seconds`
  → Use `interval_minutes` and correct struct fields

error[E0599]: no method named `install_daemon` / `uninstall_daemon`
  → Use platform.install() / platform.uninstall()
```

### Jobs To Be Done (Next Session)

1. [ ] **Fix main.rs type errors** - Align daemon command code with existing APIs
   - Use `interval_minutes` not `interval_seconds`
   - Use `platform.status()` not `check_daemon_status()`
   - Use `platform.install()` / `platform.uninstall()`
   - Fix InstallConfig struct fields

2. [ ] **Test fresh install scenario** - Remove DB, run daemon once
   - Success criteria: `tastematter daemon once` creates DB and syncs

3. [ ] **Commit and push** - Once tests pass

## File Locations

| File | Purpose | Status |
|------|---------|--------|
| [[core/src/storage.rs]] | ensure_schema() implementation | Modified, 3 new tests |
| [[core/src/daemon/sync.rs]] | Wires ensure_schema() + mkdir | Modified |
| [[core/src/main.rs]] | Early daemon handling (broken) | Modified, has errors |

## Test State

- **Library tests:** 257 passing, 0 failing, 3 ignored
- **Command:** `cargo test --lib`
- **Last run:** 2026-02-02
- **New tests:** 3 (ensure_schema tests)

### Test Commands for Next Agent

```bash
# Navigate to core
cd apps/tastematter/core

# Run library tests (should pass)
cargo test --lib

# Build release (currently fails - fix main.rs first)
cargo build --release

# After fixing, test fresh install:
mv ~/.context-os/context_os_events.db ~/.context-os/context_os_events.db.bak
tastematter daemon once
# Should create DB and sync ~966 sessions
```

## Schema Created by ensure_schema()

Tables created (all with `IF NOT EXISTS`):
- `file_events` - File system events
- `claude_sessions` - Parsed session data
- `git_commits` - Git history
- `chains` - Chain metadata
- `chain_graph` - Session-to-chain mapping
- `_metadata` - Schema version tracking

## For Next Agent

**Context Chain:**
- Previous: [[00_2026-02-01_DATABASE_WRITE_PATH_FIX]] (write path working)
- This package: Auto-init in progress, main.rs needs fixing
- Next action: Fix type errors in main.rs

**Start here:**
1. Read this context package
2. Run `cargo build --release` to see current errors
3. Fix the 8 type errors in main.rs daemon handling
4. Use existing platform API: `status()`, `install()`, `uninstall()`
5. Use `interval_minutes` not `interval_seconds`

**Key API References:**
```rust
// Correct platform API (from daemon/platform/*.rs)
platform.status() -> Result<PlatformStatus, PlatformError>
platform.install(&config) -> Result<InstallResult, PlatformError>
platform.uninstall() -> Result<(), PlatformError>

// Correct config fields
config.sync.interval_minutes: u32  // NOT interval_seconds

// Correct InstallConfig (from daemon/platform/mod.rs)
InstallConfig {
    binary_path: PathBuf,      // NOT Option<PathBuf>
    interval_minutes: u32,     // NOT interval_seconds
    // ...other fields
}
```

**Do NOT:**
- Use `interval_seconds` (doesn't exist, use `interval_minutes`)
- Use `check_daemon_status()` (doesn't exist, use `platform.status()`)
- Use `install_daemon()` / `uninstall_daemon()` (use `install()` / `uninstall()`)

**Key insight:**
The daemon commands must be handled BEFORE the database pre-check at line 563-568 of main.rs.
The early-return pattern is correct, but the daemon command implementations need to use
the existing platform API, not new methods.
[VERIFIED: grep shows correct method names in daemon/platform/*.rs]
