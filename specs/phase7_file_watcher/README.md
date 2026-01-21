# Phase 7: File Watcher Specification

**Status:** READY FOR IMPLEMENTATION
**Created:** 2026-01-18
**Python Source:** 568 lines
**Rust Target:** ~450 lines

---

## Overview

Port Python `file_watcher.py` to Rust for single-binary distribution.

**Purpose:** Real-time file system monitoring for Context OS
- Watch project directories for changes
- Debounce rapid events (IDE auto-save, formatters)
- Filter ignored patterns (.git, node_modules, etc.)
- Persist events to SQLite

---

## Specification Documents

| File | Purpose | Read Order |
|------|---------|------------|
| [[00_ARCHITECTURE_GUIDE.md]] | Architecture overview, data flow | 1st |
| [[01_TYPE_CONTRACTS.rs]] | Rust type definitions | 2nd |
| [[02_IMPLEMENTATION_GUIDE.md]] | Step-by-step implementation | 3rd |
| [[03_TEST_DRIVEN_PLAN.md]] | TDD cycles with test code | 4th |

---

## Quick Reference

### Dependencies

```toml
notify = "6.1"   # File system notifications
glob = "0.3"     # Already exists - pattern matching
```

### Key Types

```rust
pub struct FileEvent {
    pub timestamp: DateTime<Utc>,
    pub path: String,           // Relative to repo root
    pub event_type: String,     // create, write, delete, rename
    pub size_bytes: Option<i64>,
    pub old_path: Option<String>,
    pub is_directory: bool,
    pub extension: Option<String>,
}

pub struct EventFilter { /* 40+ ignore patterns */ }
pub struct EventDebouncer { /* 100ms consolidation */ }
pub struct FileWatcher { /* Orchestrator */ }
```

### TDD Plan

| Cycle | Component | Tests |
|-------|-----------|-------|
| 1 | EventFilter | 6 |
| 2 | EventDebouncer | 4 |
| 3 | FileEvent + DB | 4 |
| 4 | FileWatcher | 4 |
| **Total** | | **18** |

---

## Success Criteria

- [ ] 18 unit tests passing
- [ ] 40+ ignore patterns (parity with Python)
- [ ] Debouncing works (100ms window)
- [ ] Events persist to database
- [ ] CLI `watch` command functional
- [ ] <5% CPU during idle
- [ ] <100ms event latency

---

## Implementation Time

**Estimated:** 5-6 hours
- Cycle 1 (Filter): 1 hour
- Cycle 2 (Debouncer): 45 min
- Cycle 3 (FileEvent + DB): 45 min
- Cycle 4 (FileWatcher): 1.5 hours
- CLI + Testing: 1 hour

---

## References

- Python source: [[cli/src/context_os_events/capture/file_watcher.py]]
- Previous phase: [[context_packages/04_daemon/25_2026-01-18_PHASE6_INVERTED_INDEX_COMPLETE.md]]
- Plan file: [[~/.claude/plans/synchronous-coalescing-harbor.md]]
