# Phase 7: File Watcher - Architecture Guide

**Purpose:** Real-time file system monitoring for Context OS
**Python Source:** `cli/src/context_os_events/capture/file_watcher.py` (568 lines)
**Rust Target:** `core/src/capture/file_watcher.rs` (~450 lines estimated)
**Created:** 2026-01-18
**Status:** PLANNING

---

## Executive Summary

Port Python file watcher to Rust for single-binary distribution. The file watcher monitors project directories for changes, debounces rapid events, filters ignored patterns, and persists events to SQLite.

**Key difference from Python:** Use `notify` crate instead of `watchdog`.

---

## Problem Validation (Staff Engineer Framework)

### What problem are we solving?

**User need:** Track file changes in real-time to understand work patterns.

**Current state:** Python daemon requires Python + pip + watchdog.

**Target state:** Single Rust binary with integrated file watching.

### What's the 80% use case?

1. User runs `context-os daemon`
2. Daemon watches project directory
3. File changes are captured and persisted
4. Changes are queryable via CLI/UI

### Success metric

- File events captured with <100ms latency
- Zero missed events during normal development
- <5% CPU usage during idle watching

### Can existing code handle this?

**NO** - Python implementation uses `watchdog` library. Rust needs `notify` crate.
Must port, cannot reuse.

---

## Architecture (60-Second Rule)

```
┌─────────────────────────────────────────────────────────────────┐
│                    FILE WATCHER ARCHITECTURE                     │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  File System           notify crate          Rust Code          │
│  ┌────────────┐       ┌────────────┐       ┌──────────────┐    │
│  │ File       │──────►│ Raw Event  │──────►│ EventFilter  │    │
│  │ Changes    │       │ (Create,   │       │ (40+ ignore  │    │
│  │            │       │ Modify,    │       │ patterns)    │    │
│  │            │       │ Delete,    │       └──────┬───────┘    │
│  │            │       │ Rename)    │              │             │
│  └────────────┘       └────────────┘              ▼             │
│                                          ┌──────────────┐       │
│                                          │ Debouncer    │       │
│                                          │ (100ms       │       │
│                                          │ window)      │       │
│                                          └──────┬───────┘       │
│                                                 │               │
│                                                 ▼               │
│                                          ┌──────────────┐       │
│                                          │ Database     │       │
│                                          │ (file_events │       │
│                                          │ table)       │       │
│                                          └──────────────┘       │
└─────────────────────────────────────────────────────────────────┘
```

---

## Component Breakdown

### Component 1: EventFilter (~100 lines)

**Purpose:** Filter out noise (.git, node_modules, __pycache__, etc.)

**Input:** Raw file path
**Output:** Boolean (should_ignore)

**Key behaviors:**
- 40+ default ignore patterns
- Pattern matching via `fnmatch` equivalent
- Path component matching (catches `__pycache__` anywhere)

### Component 2: EventDebouncer (~80 lines)

**Purpose:** Consolidate rapid saves (IDE auto-save, formatter, etc.)

**Input:** FileEvent
**Output:** Deduplicated FileEvents after debounce window

**Key behaviors:**
- 100ms default debounce window
- Thread-safe (Mutex-protected)
- Replaces events for same path (keeps latest)
- Periodic flush (50ms polling)

### Component 3: FileWatcher (~150 lines)

**Purpose:** Orchestrate watching, filtering, debouncing, persistence

**Input:** Watch path, database connection
**Output:** Running watcher with stats

**Key behaviors:**
- Recursive directory watching
- Background flush thread
- Graceful shutdown with final flush
- Statistics tracking

### Component 4: Database Operations (~50 lines)

**Purpose:** Persist file events to SQLite

**Input:** FileEvent
**Output:** Inserted row

**Key behaviors:**
- Single event insert
- Batch event insert
- Transaction management

---

## Data Flow

```
1. notify::Watcher detects file change
        ↓
2. EventHandler receives raw event
        ↓
3. EventFilter.should_ignore(path) → Skip if true
        ↓
4. FileEvent created from path + event type
        ↓
5. EventDebouncer.add(event) → Buffer event
        ↓
6. [50ms later] Debouncer.flush() → Events ready
        ↓
7. insert_events(db, events) → Persist to SQLite
```

---

## Dependencies

```toml
# Cargo.toml additions
notify = "6.1"                    # File system notifications
notify-debouncer-mini = "0.4"     # Built-in debouncing (optional)
```

**Note:** We may implement custom debouncing for more control, matching Python behavior exactly.

---

## Database Schema

**Table:** `file_events` (already exists in storage.rs schema)

```sql
CREATE TABLE IF NOT EXISTS file_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL,          -- ISO8601
    path TEXT NOT NULL,               -- Relative to repo root
    event_type TEXT NOT NULL,         -- create, write, delete, rename
    size_bytes INTEGER,               -- File size (NULL for delete)
    old_path TEXT,                    -- Previous path for renames
    is_directory INTEGER NOT NULL,    -- Boolean as 0/1
    extension TEXT                    -- File extension
);

CREATE INDEX IF NOT EXISTS idx_file_events_timestamp ON file_events(timestamp);
CREATE INDEX IF NOT EXISTS idx_file_events_path ON file_events(path);
```

---

## Loose Coupling Verification

| Component | Can run independently? | Dependencies |
|-----------|----------------------|--------------|
| EventFilter | ✅ Yes | None (pure function) |
| EventDebouncer | ✅ Yes | None (pure data structure) |
| FileWatcher | ❌ No | notify, EventFilter, EventDebouncer, Storage |

**Result:** Components are loosely coupled. Filter and Debouncer can be unit tested in isolation.

---

## Success Criteria

- [ ] 18+ unit tests passing (4 cycles)
- [ ] 40+ ignore patterns working (parity with Python)
- [ ] Debouncing consolidates rapid events (100ms window)
- [ ] Events persist to database correctly
- [ ] CLI `watch` command functional
- [ ] Performance: <5% CPU during idle, <100ms event latency

---

## Estimated Effort

| Component | Lines | Time | Tests |
|-----------|-------|------|-------|
| Types + Constants | ~50 | 30 min | - |
| EventFilter | ~100 | 1 hour | 6 |
| EventDebouncer | ~80 | 45 min | 4 |
| FileWatcher | ~150 | 1.5 hours | 4 |
| Database Ops | ~50 | 30 min | 4 |
| CLI Command | ~30 | 30 min | - |
| **Total** | **~460** | **~5 hours** | **18** |

---

## References

- Python source: [[cli/src/context_os_events/capture/file_watcher.py]]
- Type contracts: [[specs/phase7_file_watcher/01_TYPE_CONTRACTS.rs]]
- TDD plan: [[specs/phase7_file_watcher/03_TEST_DRIVEN_PLAN.md]]
- notify crate docs: https://docs.rs/notify/6.1.1/notify/
