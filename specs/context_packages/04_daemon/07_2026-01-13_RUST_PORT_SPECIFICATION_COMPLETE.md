---
title: "Rust Port Specification Complete"
package_number: 07
date: 2026-01-13
status: current
previous_package: "[[06_2026-01-13_CORE_INFRASTRUCTURE_AUDIT]]"
related:
  - "[[canonical/06_RUST_PORT_SPECIFICATION]]"
  - "[[canonical/03_CORE_ARCHITECTURE]]"
  - "[[cli/src/context_os_events/capture/git_sync.py]]"
  - "[[cli/src/context_os_events/capture/jsonl_parser.py]]"
  - "[[cli/src/context_os_events/index/chain_graph.py]]"
tags:
  - context-package
  - tastematter
  - rust-port
  - specification
---

# Rust Port Specification Complete - Context Package 07

## Executive Summary

Comprehensive specification written for porting Python indexer/daemon to Rust. Applied technical-architecture-engineering and feature-planning-and-decomposition skills to create 6-phase implementation plan with type contracts, algorithms, and success criteria.

**Deliverable:** `specs/canonical/06_RUST_PORT_SPECIFICATION.md` (~700 lines)

## Session Work

### Skills Applied

1. **feature-planning-and-decomposition:**
   - Staff Engineer Validation (passed all 4 questions)
   - 80% use case defined (parse JSONL → build chains → write to DB)
   - Success metrics with baselines and targets
   - Confirmed: Cannot solve with <200 lines, architectural port required

2. **technical-architecture-engineering:**
   - Jeff Dean latency analysis: Python spawn (4800ms) vs Rust call (<1μs) = 4,800,000x overhead
   - Jim Gray caching: Chain graph YES, session summaries NO
   - Brendan Gregg USE Method: IPC is bottleneck, not compute/I/O

### Architecture Decision

```
┌─────────────────────────────────────────────────────────────┐
│                    RUST CORE (Single Binary)                 │
├─────────────────────────────────────────────────────────────┤
│  capture/           index/            query/                │
│  ├── git_sync       └── chain_graph   └── (existing)       │
│  ├── jsonl_parse                                            │
│  └── file_watch                                             │
│                          │                                   │
│                          ▼                                   │
│              storage.rs (rusqlite + r2d2)                   │
│                          │                                   │
│                          ▼                                   │
│                    SQLite Database                          │
└─────────────────────────────────────────────────────────────┘
```

### 6-Phase Implementation Plan

| Phase | Component | Hours | Dependencies |
|-------|-----------|-------|--------------|
| 1 | Storage Foundation | 4-6 | None |
| 2 | Git Sync | 8-12 | Phase 1 |
| 3 | JSONL Parser | 12-16 | Phase 1 |
| 4 | Chain Graph | 8-12 | Phase 3 |
| 5 | File Watcher | 6-8 | Phase 3 |
| 6 | Daemon | 6-8 | Phases 2-5 |
| **Total** | | **44-62** | |

### Type Contracts Defined

```rust
// Key types from spec
pub struct GitCommit { hash, timestamp, author, is_agent_commit, files_changed }
pub struct ParsedSession { session_id, leaf_uuid, parent_session_id, tool_uses }
pub struct ChainNode { session_id, parent_id, children, chain_id }
pub struct Chain { id, root_session_id, session_count }
```

### Success Metrics

| Metric | Current (Python) | Target (Rust) |
|--------|------------------|---------------|
| Cold start | ~4,800ms | <100ms |
| Parse throughput | ~50 sessions/sec | ~500 sessions/sec |
| Memory | ~200MB | <50MB |
| Distribution | Python + pip | Single binary |

## Files Created

| File | Purpose | Lines |
|------|---------|-------|
| `specs/canonical/06_RUST_PORT_SPECIFICATION.md` | Full implementation spec | ~700 |

## Jobs To Be Done (Next Agent)

### Phase 1: Storage Foundation (4-6 hrs)

1. [ ] Add r2d2 and rusqlite to Cargo.toml
2. [ ] Implement connection pooling in storage.rs
3. [ ] Add `write()` method for read-write connections
4. [ ] Implement `insert_commits()`, `insert_sessions()`, `insert_tool_uses()`
5. [ ] Add transaction support
6. [ ] Test: Batch of 1000 inserts in <50ms

### Phase 2: Git Sync (8-12 hrs)

7. [ ] Add git2 crate to Cargo.toml
8. [ ] Create capture/git_sync.rs module
9. [ ] Implement `sync_commits()` using git2 (no subprocess)
10. [ ] Implement `is_agent_commit()` detection
11. [ ] Add `sync-git` CLI command
12. [ ] Test: 1000 commits synced in <5s

### Parallel Opportunities

- Phase 2 (Git Sync) and Phase 3 (JSONL Parser) can develop in parallel after Phase 1
- Phase 5 (File Watcher) can develop in parallel with Phase 4 (Chain Graph)

## For Next Agent

### Context Chain

| # | Date | Title | Key Content |
|---|------|-------|-------------|
| 00 | 2026-01-12 | CHAIN_LINKING_BUG_INVESTIGATION | Bug discovery |
| 01 | 2026-01-13 | CLAUDE_CODE_JSONL_DATA_MODEL | Data model reference |
| 02 | 2026-01-13 | CHAIN_LINKING_FIX_COMPLETE | Algorithm fixed |
| 03 | 2026-01-13 | INTEL_LAYER_PRIORITY_DECISION | Architectural necessity |
| 04 | 2026-01-13 | CLI_INSTALLATION_FIX | Renamed to tastematter |
| 05 | 2026-01-13 | DAEMON_CHAIN_BUILDING_COMPLETE | Auto chain building |
| 06 | 2026-01-13 | CORE_INFRASTRUCTURE_AUDIT | User decision: Full port |
| 07 | 2026-01-13 | RUST_PORT_SPECIFICATION_COMPLETE | **This package** |

### Start Here

1. Read `specs/canonical/06_RUST_PORT_SPECIFICATION.md` for full implementation details
2. Start with Phase 1: Storage Foundation
3. Follow type contracts exactly as specified
4. Run tests after each phase completes

### Critical Constraints

1. **Python CLI must remain functional during port**
2. **Each phase has its own CLI command for isolated testing**
3. **Compare Rust output with Python output for validation**

### Key Insight

The specification follows specification-driven-development methodology:
- Clear type contracts (300ms comprehension rule)
- Step-by-step implementation instructions
- Test criteria for each phase
- Success criteria checklist

This prevents "what should I do?" moments during implementation.

[VERIFIED: 06_RUST_PORT_SPECIFICATION.md created 2026-01-13]

## Test Commands

```bash
# Verify spec file exists
ls -la apps/tastematter/specs/canonical/06_RUST_PORT_SPECIFICATION.md

# Read spec file
cat apps/tastematter/specs/canonical/06_RUST_PORT_SPECIFICATION.md

# Verify Rust core still builds
cd apps/tastematter/core && cargo build --release

# Verify Python CLI still works
tastematter --help
```
