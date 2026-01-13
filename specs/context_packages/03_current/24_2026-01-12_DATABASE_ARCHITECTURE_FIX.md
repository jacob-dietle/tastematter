# Context Package 24: Database Architecture Fix

---
package: 24

migrated_from: "apps/tastematter/specs/context_packages/24_2026-01-12_DATABASE_ARCHITECTURE_FIX.md"
previous: [[23_2026-01-11_BUG_FIXES_COMPLETE]]
status: complete
---

## Summary

Resolved critical P0 architecture issue: "Three-Database Problem". Python indexer wrote to one location, Rust query engine read from another (stale), migration from `context_os_events/` to `context-os/` was never completed. Now: single canonical database at `~/.context-os/context_os_events.db`.

## What Was Accomplished

### Three-Database Problem - RESOLVED

**Problem Discovered:** Visual debugging of "No chain" sessions revealed THREE separate SQLite databases with no synchronization:

| # | Path | Status | Modified |
|---|------|--------|----------|
| 1 | `apps/context_os_events/data/` | OLD (underscore) - Rust found this FIRST | Jan 4 |
| 2 | `apps/context-os/cli/data/` | Python wrote here | Jan 11 |
| 3 | `apps/context-os/data/` | Intended unified location | Jan 11 |

**Root Cause:**
1. Directory reorg from `context_os_events/` → `context-os/` was planned but old directory never deleted
2. `storage.rs` had 5 fallback paths - found old stale database first
3. Python CLI created its own `cli/data/` directory instead of unified location

### storage.rs Rewrite - Jeff Dean Approved

**BEFORE (broken):** 5 fallback paths with hardcoded Windows path
```rust
// Old: Would find stale database first
let search_paths = [
    "C:/Users/dietl/.../context_os_events/data/", // Found first!
    "~/.context-os/",
    // ... 3 more fallbacks
];
```

**AFTER (fixed):** Explicit `--db` flag OR canonical path only
```rust
// New: Single canonical location, fail fast
const DB_FILENAME: &str = "context_os_events.db";
const DB_DIR: &str = ".context-os";

pub fn find_database(explicit_path: Option<&Path>) -> Result<PathBuf, CoreError> {
    // 1. Explicit path (--db flag)
    if let Some(path) = explicit_path {
        if path.exists() { return Ok(path.to_path_buf()); }
        return Err(CoreError::Config("not found at specified path"));
    }

    // 2. Canonical location ONLY (~/.context-os/context_os_events.db)
    let canonical = Self::canonical_path()?;
    if canonical.exists() { return Ok(canonical); }

    // 3. NO FALLBACK - fail fast with clear error
    Err(CoreError::Config("Database not found at canonical location"))
}
```

**Key design decisions:**
- NO environment variable (user said CLI's --db flag is sufficient)
- NO fallback search paths (eliminates ambiguity)
- Fail fast with clear error message showing expected location

### Cleanup Completed

| Action | Path | Result |
|--------|------|--------|
| Centralized | `~/.context-os/context_os_events.db` | Canonical location |
| Backup created | `~/.context-os/backup_2026-01-12/` | Safety copy |
| Deleted | `apps/context_os_events/` | Old underscore directory |
| Deleted | `apps/context-os/cli/data/` | Python CLI duplicate |
| Deleted | `apps/context-os/data/` | Unified location duplicate |

### Claude Code JSONL Structure Documented

Created canonical reference: [[08_CHAIN_LINKING_CANONICAL_REFERENCE.md]]

Key insight: Chain linking is EXPLICIT via `leafUuid`, not heuristic-based.

```
Session B (child) summary record:
{
  "type": "summary",
  "leafUuid": "msg-003"  ← Points to message UUID in parent session
}

Session A (parent) contains:
{
  "type": "user",
  "uuid": "msg-003"  ← This message is referenced
}
```

Four-pass algorithm:
1. Extract leafUuid from summary records
2. Extract message.uuid from user/assistant/tool_result records
3. Build parent→child relationships
4. Group into chains (connected components)

## Architecture Clarified

**Three Interfaces to Shared Rust Core:**

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│  Browser Dev    │     │  Production     │     │  Agent/CLI      │
│  (HTTP API)     │     │  (Tauri IPC)    │     │  Interface      │
│  localhost:3001 │     │  Direct binding │     │  context-os     │
└────────┬────────┘     └────────┬────────┘     └────────┬────────┘
         │                       │                       │
         └───────────────────────┼───────────────────────┘
                                 │
                    ┌────────────▼────────────┐
                    │     RUST CORE           │
                    │  context-os-core        │
                    │                         │
                    │  query.rs, storage.rs   │
                    │  types.rs, error.rs     │
                    └────────────┬────────────┘
                                 │
                    ┌────────────▼────────────┐
                    │  CANONICAL DATABASE     │
                    │  ~/.context-os/         │
                    │  context_os_events.db   │
                    └─────────────────────────┘
```

## Files Modified

| File | Change |
|------|--------|
| `apps/context-os/core/src/storage.rs` | Complete rewrite - canonical path only |
| `apps/context-os/specs/event_capture/08_CHAIN_LINKING_CANONICAL_REFERENCE.md` | New - JSONL structure docs |
| `apps/tastematter/specs/07_DATABASE_ARCHITECTURE_RCA.md` | Exists - full RCA document |

## Files Deleted

- `apps/context_os_events/` (entire directory - old underscore naming)
- `apps/context-os/cli/data/context_os_events.db`
- `apps/context-os/data/context_os_events.db`

## Test Status

```
All 14 Rust tests passing:
cargo test (in apps/context-os/core)

CLI verification:
./target/release/context-os query chains --limit 3
→ Returns chains with session_count and file_count (717 files in main chain)
```

## Jobs To Be Done (Next Session)

### Immediate
1. **Verify CLI works without --db flag** - Should use canonical path automatically
2. **Test Tauri app** - Ensure it finds database at canonical location
3. **Test HTTP server** - Browser dev should work

### Roadmap
4. **Port Python indexer to Rust** - User requested this for language standardization
   - `apps/context-os/cli/src/context_os_events/` → Rust
   - Key files: `index/chain_graph.py`, `capture/jsonl_parser.py`
   - Benefit: Single language, faster indexing, no Python dependency

### Remaining UI Issues (from package 23)
- ISSUE-003: Timeline shows individual files instead of sessions
- ISSUE-004: Session names are meaningless hashes
- ISSUE-005: Timeline buckets empty

## Verification Commands

```bash
# Verify canonical database exists
ls -la ~/.context-os/context_os_events.db

# Verify old directories gone
ls apps/context_os_events/  # Should fail: No such file

# Test CLI with canonical path (no --db flag)
cd apps/context-os/core
./target/release/context-os query chains --limit 3

# Test with explicit --db flag
./target/release/context-os --db ~/.context-os/context_os_events.db query sessions --time 7d

# Run Rust tests
cargo test
```

## Related Specs

- [[07_DATABASE_ARCHITECTURE_RCA]] - Full forensic analysis of the three-database problem
- [[08_CHAIN_LINKING_CANONICAL_REFERENCE]] - Claude Code JSONL structure documentation
- [[23_2026-01-11_BUG_FIXES_COMPLETE]] - Previous session fixing chain-file linkage

---

**Session Duration:** ~90 minutes
**Key Achievement:** Eliminated three-database problem, single canonical path, Jeff Dean simplicity principles applied
**Architecture Decision:** CLI's `--db` flag is sufficient - no env var or config file needed
