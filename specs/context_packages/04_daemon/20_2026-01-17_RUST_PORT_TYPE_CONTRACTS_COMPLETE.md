---
title: "Tastematter Context Package 20"
package_number: 20
date: 2026-01-17
status: current
previous_package: "[[19_2026-01-17_PARSER_GAP_FIX_COMPLETE_BOTH_FILES]]"
related:
  - "[[specs/canonical/08_PYTHON_PORT_INVENTORY.md]]"
  - "[[specs/canonical/09_RUST_PORT_TYPE_CONTRACTS.md]]"
  - "[[~/.claude/plans/synchronous-coalescing-harbor.md]]"
tags:
  - context-package
  - tastematter
  - rust-port
---

# Tastematter - Context Package 20

## Executive Summary

Completed comprehensive documentation for Rust daemon port: full Python inventory (11,133 lines across 34 files) and detailed type contracts for all 6 remaining phases (3-8). Ready to begin Phase 3 implementation.

## Global Context

### Architecture Overview

Tastematter is being ported from Python CLI to Rust for single-binary distribution:

```
CURRENT STATE (Hybrid):
┌─────────────────────────────────────────────────────────┐
│ Python CLI (5,520 lines)         Rust Core (complete)  │
│ ├── capture/                     ├── storage.rs        │
│ │   ├── git_sync.py      ──────► │   (read + write)   │
│ │   ├── jsonl_parser.py          ├── query.rs         │
│ │   └── file_watcher.py          ├── types.rs         │
│ ├── index/                       └── http.rs          │
│ │   ├── chain_graph.py                                 │
│ │   └── inverted_index.py                              │
│ └── daemon/                                            │
│     └── runner.py                                      │
└─────────────────────────────────────────────────────────┘

TARGET STATE (Single Binary):
┌─────────────────────────────────────────────────────────┐
│ Rust Core (all operations)                              │
│ ├── capture/git_sync.rs          Phase 3               │
│ ├── capture/jsonl_parser.rs      Phase 4               │
│ ├── capture/file_watcher.rs      Phase 7               │
│ ├── index/chain_graph.rs         Phase 5               │
│ ├── index/inverted_index.rs      Phase 6               │
│ └── daemon/runner.rs             Phase 8               │
└─────────────────────────────────────────────────────────┘
```

### Phase Status

| Phase | Name | Status | Evidence |
|-------|------|--------|----------|
| 0 | Glob Bug Fix | ✅ COMPLETE | Package 14, 988 sessions |
| 1 | Storage Foundation | ✅ COMPLETE | 26 Rust tests |
| 2 | Tauri Integration | ✅ COMPLETE | Package 10 |
| 2.5 | Parser Gap Fix | ✅ COMPLETE | Package 19, 468 tests |
| 3 | Git Sync | ⬜ SPEC COMPLETE | Type contracts ready |
| 4 | JSONL Parser | ⬜ SPEC COMPLETE | Type contracts ready |
| 5 | Chain Graph | ⬜ SPEC COMPLETE | Type contracts ready |
| 6 | Inverted Index | ⬜ SPEC COMPLETE | Type contracts ready |
| 7 | File Watcher | ⬜ SPEC COMPLETE | Type contracts ready |
| 8 | Daemon Runner | ⬜ SPEC COMPLETE | Type contracts ready |

## Local Problem Set

### Completed This Session

- [X] Fixed test assertion for relative paths from file-history-snapshot [VERIFIED: [[cli/tests/test_cli_query.py]]]
- [X] Enumerated all Python modules for Rust port (34 files, 11,133 lines) [VERIFIED: [[specs/canonical/08_PYTHON_PORT_INVENTORY.md]]]
- [X] Created comprehensive type contracts for phases 3-8 [VERIFIED: [[specs/canonical/09_RUST_PORT_TYPE_CONTRACTS.md]]]
- [X] Documented 3-source extraction algorithm in type contracts
- [X] Documented 5-pass chain algorithm in type contracts
- [X] Documented path encoding (Windows C:\ → C--) in type contracts

### In Progress

None - documentation phase complete, ready for implementation.

### Jobs To Be Done (Next Session)

1. [ ] **Phase 3: Git Sync Implementation**
   - Add `git2` crate dependency to Cargo.toml
   - Create `core/src/capture/mod.rs` and `core/src/capture/git_sync.rs`
   - Port `sync_commits`, `parse_commit_block`, `detect_agent_commit`
   - Add CLI command `context-os sync-git`
   - Write TDD tests
   - Success criteria: Rust output matches Python (commit counts)

2. [ ] **Phase 4: JSONL Parser Implementation**
   - Create `core/src/capture/jsonl_parser.rs`
   - Port 3-source extraction (tool_use, toolUseResult, file-history-snapshot)
   - Port path encoding/decoding
   - Add CLI command `context-os parse-sessions`
   - Success criteria: 196K tool uses extracted (matching Python)

3. [ ] **Phase 5-8** - Follow type contracts in sequence

## File Locations

| File | Purpose | Status |
|------|---------|--------|
| [[specs/canonical/08_PYTHON_PORT_INVENTORY.md]] | Full Python enumeration | Created |
| [[specs/canonical/09_RUST_PORT_TYPE_CONTRACTS.md]] | Rust type contracts | Created |
| [[~/.claude/plans/synchronous-coalescing-harbor.md]] | Master plan file | Updated |
| [[cli/src/context_os_events/capture/git_sync.py]] | Python git sync (reference) | Unchanged |
| [[cli/src/context_os_events/capture/jsonl_parser.py]] | Python parser (reference) | Unchanged |
| [[cli/src/context_os_events/index/chain_graph.py]] | Python chains (reference) | Unchanged |
| [[core/src/types.rs]] | Existing Rust types | Has GitCommitInput |

## Test State

- Python CLI: 468 tests passing
- Rust Core: 26 tests passing
- Command: `cd cli && pytest tests/ -v`
- Last run: 2026-01-17

### Test Commands for Next Agent
```bash
# Verify Python test state
cd apps/tastematter/cli && pytest tests/ -v --tb=short

# Verify Rust test state
cd apps/tastematter/core && cargo test

# After implementing Phase 3
cd apps/tastematter/core && cargo test git_sync
```

## Key Specifications Created

### 08_PYTHON_PORT_INVENTORY.md

Complete enumeration of Python code:

| Layer | Files | Lines | Key Modules |
|-------|-------|-------|-------------|
| Capture | 3 | 1,678 | git_sync, jsonl_parser, file_watcher |
| Index | 5 | 3,221 | chain_graph, inverted_index, context_index |
| Daemon | 4 | 988 | runner, config, state |
| **Total** | **34** | **11,133** | - |

**Must Port:** 3,405 lines (Capture + Core Index + Daemon)
**Already Ported:** 1,035 lines (Database, Query Engine)
**Defer:** 4,211 lines (Intelligence, Observability)

### 09_RUST_PORT_TYPE_CONTRACTS.md

Detailed type mappings for each phase:

**Phase 3 (Git Sync):**
- `GitCommit` struct (15 fields)
- `SyncOptions`, `SyncResult` types
- `AGENT_SIGNATURES` constant
- 5 functions to port

**Phase 4 (JSONL Parser):**
- `ToolUse`, `ParsedMessage`, `SessionSummary` structs
- 3-source extraction algorithm (critical)
- Path encoding functions
- 7 functions to port

**Phase 5 (Chain Graph):**
- `ChainNode`, `Chain` structs
- 5-pass algorithm (critical: LAST leafUuid)
- BFS connected components

**Phases 6-8:** File access, file watcher, daemon types

## Critical Algorithms Documented

### 3-Source Extraction (Phase 4)

```rust
// MUST extract from THREE sources:
1. assistant.tool_use blocks (existing)
2. user.toolUseResult (Gap 1 - file paths in results)
3. file-history-snapshot (Gap 2 - tracked file backups)
```

### 5-Pass Chain Algorithm (Phase 5)

```rust
// CRITICAL: Use LAST summary's leafUuid, not first!
1. Extract leafUuid from LAST summary record
2. Extract sessionId from agent sessions (agent-* files)
3. Extract message.uuid ownership
4. Build parent-child relationships
5. Group into chains via BFS
```

### Path Encoding (Phase 4)

```rust
// Windows: C:\Users\foo → C--Users-foo
// Unix: /home/user → -home-user
```

## For Next Agent

**Context Chain:**
- Previous: [[19_2026-01-17_PARSER_GAP_FIX_COMPLETE_BOTH_FILES]] (Phase 2.5 done)
- This package: Type contracts complete, ready for Phase 3
- Next action: Implement Phase 3 (Git Sync)

**Start here:**
1. Read this context package (you're doing it now)
2. Read [[specs/canonical/09_RUST_PORT_TYPE_CONTRACTS.md]] for exact types
3. Read [[cli/src/context_os_events/capture/git_sync.py]] for Python reference
4. Run: `cd apps/tastematter/core && cargo test` to verify baseline

**Key insight:**
The type contracts document is the implementation blueprint. If Rust types serialize to identical JSON as Python, the port is correct. [VERIFIED: [[specs/canonical/09_RUST_PORT_TYPE_CONTRACTS.md]]]

**Do NOT:**
- Skip reading the type contracts (they contain critical algorithms)
- Use first leafUuid (use LAST for immediate parent)
- Forget 3-source extraction (tool_use, toolUseResult, file-history-snapshot)
- Use `*.jsonl` glob (must be `**/*.jsonl` for recursive)

## Dependencies for Implementation

```toml
# Add to core/Cargo.toml for Phases 3-8
git2 = { version = "0.19", features = ["bundled"] }  # Phase 3
notify = "6.1"                                        # Phase 7
notify-debouncer-mini = "0.4"                         # Phase 7
glob = "0.3"                                          # Phase 4
```

## Estimated Effort

| Phase | Hours | Cumulative |
|-------|-------|------------|
| 3: Git Sync | 8-12 | 8-12 |
| 4: JSONL Parser | 12-16 | 20-28 |
| 5: Chain Graph | 8-12 | 28-40 |
| 6: Inverted Index | 4-6 | 32-46 |
| 7: File Watcher | 6-8 | 38-54 |
| 8: Daemon | 6-8 | 44-62 |

**Total:** 44-62 hours to complete Rust port
