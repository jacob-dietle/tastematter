# Tastematter Implementation Specs

## Overview

This directory contains **agent task specifications** for implementing the context-os-core unified architecture. Each phase has three files:

- **SPEC.md** - Agent task specification (500-700 lines)
- **CONTRACTS.rs** - Rust type contracts (compilable reference)
- **TESTS.md** - TDD test plan (Kent Beck Red-Green-Refactor)

## Architecture Reference

- **Canonical Architecture:** `../canonical/03_CORE_ARCHITECTURE.md`
- **Context Package:** `../context_packages/09_2026-01-08_UNIFIED_CORE_ARCHITECTURE.md`

---

## Phase Overview

| Phase | Name | Priority | Time | Dependencies | Status |
|-------|------|----------|------|--------------|--------|
| 1 | Core Foundation | CRITICAL | 3-4 hrs | None | ✅ COMPLETE |
| 2 | Tauri Integration | CRITICAL | 2-3 hrs | Phase 1 | ✅ COMPLETE |
| **3** | **HTTP API / Transport** | **HIGH** | **3-4 hrs** | **Phase 1** | **SPECIFIED** |
| 4 | Cache Layer | MEDIUM | 2 hrs | Phase 1 | DEFERRED |
| 5 | Logging Integration | MEDIUM | 1-2 hrs | Phase 1 | DEFERRED |
| 6 | IPC Socket Server | LOW | 2-3 hrs | Phase 1 | DEFERRED |
| 7 | UI State Machine | MEDIUM | 2-3 hrs | Phases 1, 6 | DEFERRED |
| 8 | Event Bus | LOW | 2 hrs | Phases 1, 6, 7 | DEFERRED |

**Total Estimated Time:** 17-22 hours

> **Note:** Phases 4-8 are DEFERRED per [[03_CORE_ARCHITECTURE.md]] implementation status.
> Phase 3 (HTTP API) added to enable browser-based development and Claude Code automation.

---

## Dependency Graph

```
Phase 1 (Core) ──────┬────► Phase 2 (Tauri) ────────────────────────┐
                     │                                               │
                     ├────► Phase 3 (Cache) ────────────────────────┤
                     │                                               │
                     ├────► Phase 4 (Logging) ──────────────────────┤
                     │                                               │
                     └────► Phase 5 (IPC) ────► Phase 6 (CLI) ──────┤
                                  │                                  │
                                  └────► Phase 7 (UI State) ────────┤
                                              │                      │
                                              └──────► Phase 8 (Bus)─┘
```

---

## Recommended Execution Order

### Sprint 1: Get Visible Value Fast (5-7 hours)

**Goal:** Tastematter queries in <100ms instead of 18 seconds

1. **Phase 1: Core Foundation** (3-4 hrs)
   - Create `apps/context-os-core/` Rust library
   - Implement direct SQLite queries
   - **Checkpoint:** `cargo test` passes, queries <100ms

2. **Phase 2: Tauri Integration** (2-3 hrs)
   - Replace Command::new() with library calls
   - **Checkpoint:** App works, visible speed improvement

### Sprint 2: Complete the System (5-7 hours)

**Goal:** CLI queries fast, full observability

3. **Phase 3: Cache Layer** + **Phase 4: Logging** (3 hrs, parallel)
   - Add query cache for repeated queries
   - Add correlation ID logging

4. **Phase 5: IPC Socket Server** (2-3 hrs)
   - Enable CLI to query via socket

5. **Phase 6: CLI Thin Wrapper** (2 hrs)
   - Python CLI becomes thin wrapper
   - **Checkpoint:** CLI queries <500ms (was 18 seconds)

### Sprint 3: Agent Foundation (4-5 hours)

**Goal:** Agent can control UI programmatically

6. **Phase 7: UI State Machine** (2-3 hrs)
   - Shared UI state between CLI and app

7. **Phase 8: Event Bus** (2 hrs)
   - Pub/sub for cross-component coordination
   - **Checkpoint:** Agent can navigate UI

---

## Phase Specifications

### Phase 1: Core Foundation
**Directory:** `phase_01_core_foundation/`

**Mission:** Create unified Rust library with <100ms query latency

**Deliverables:**
- `apps/context-os-core/Cargo.toml`
- `apps/context-os-core/src/lib.rs`
- `apps/context-os-core/src/types.rs`
- `apps/context-os-core/src/storage.rs`
- `apps/context-os-core/src/query.rs`
- `apps/context-os-core/src/error.rs`

**Success Criteria:**
- `cargo build` succeeds
- `cargo test` passes
- Query latency < 100ms

---

### Phase 2: Tauri Integration
**Directory:** `phase_02_tauri_integration/`

**Mission:** Replace CLI calls with direct library calls, zero frontend changes

**Deliverables:**
- Modified `apps/tastematter/src-tauri/Cargo.toml`
- Modified `apps/tastematter/src-tauri/src/lib.rs`
- Modified `apps/tastematter/src-tauri/src/commands.rs`

**Success Criteria:**
- App starts in <2 seconds
- All queries return in <100ms
- No "CLI not found" errors
- Frontend unchanged

---

### Phase 3: HTTP API / Transport Abstraction
**Directory:** `phase_03_http_api/`

**Mission:** Enable browser-based development with HTTP API transport

**Deliverables:**
- `apps/context-os/core/src/http.rs` - HTTP server module
- `apps/context-os/core/src/main.rs` - `serve` subcommand
- `apps/tastematter/src/lib/api/transport.ts` - Transport interface
- `apps/tastematter/src/lib/api/http-transport.ts` - HTTP implementation
- `apps/tastematter/src/lib/api/tauri-transport.ts` - Tauri implementation

**Success Criteria:**
- `context-os serve --port 3001 --cors` starts HTTP server
- Browser mode: `npm run dev` loads data via HTTP
- Tauri mode: `npm run tauri dev` loads data via IPC
- Same frontend code works in both modes

**Agent Task Specs:**
- `01_HTTP_SERVER_AGENT_TASK.md` - Backend HTTP server
- `02_FRONTEND_TRANSPORT_AGENT_TASK.md` - Frontend abstraction

**Canonical Spec:** `../canonical/04_TRANSPORT_ARCHITECTURE.md`

---

### Phases 4-8: Deferred

Per [[03_CORE_ARCHITECTURE.md]] implementation status, phases 4-8 are deferred until needed:
- **Phase 4 (Cache):** Query latency is already <2ms, caching not needed
- **Phase 5 (Logging):** Basic logging exists, advanced logging deferred
- **Phase 6 (IPC Socket):** HTTP API satisfies dev tooling needs
- **Phase 7 (UI State):** Needed for Phase 3 Agent UI Control (roadmap)
- **Phase 8 (Event Bus):** Needed for Phase 4 Intelligent GitOps (roadmap)

---

## How to Use These Specs

### For Agents Implementing Phases

1. **Read SPEC.md first** - Understand mission, prerequisites, success criteria
2. **Reference CONTRACTS.rs** - Type definitions are the source of truth
3. **Follow TESTS.md** - Write tests BEFORE implementation (TDD)
4. **Write completion report** - Document what was built

### For Orchestrating Multi-Agent Work

1. **Check dependencies** - Don't start Phase 2 until Phase 1 complete
2. **Parallelize where possible** - Phases 2, 3, 4, 5 can run after Phase 1
3. **Use checkpoints** - Verify each sprint before proceeding

### For Context Handoffs

Each phase spec is self-contained. If context compacts:
1. Read the relevant phase SPEC.md
2. Check CONTRACTS.rs for type definitions
3. Run TESTS.md tests to verify current state
4. Continue from where previous agent left off

---

## Legacy Specs

Previous specifications have been moved to `../legacy/`:

- `07_CHAIN_INTEGRATION_SPEC.md` - Chain-related features
- `08_UNIFIED_DATA_ARCHITECTURE.md` - Data layer design
- `09_LOGGING_SERVICE_SPEC.md` - Logging implementation
- `10_PERF_OPTIMIZATION_SPEC.md` - Performance optimization

These contain valuable context but are superseded by the unified architecture in `03_CORE_ARCHITECTURE.md`.

---

## Key Principles

### From specification-driven-development skill:

1. **Specs before code** - Eliminates 30% rework
2. **Type contracts first** - Zero integration surprises
3. **Real tests over synthetic** - E2E finds 3x more bugs
4. **File-based handoffs** - Zero context loss

### From test-driven-execution skill:

1. **Red-Green-Refactor** - Tests first, always
2. **One test, one assertion** - Clear failure reasons
3. **Test pyramid** - Unit (many) → Integration (some) → E2E (few)
4. **Never trust a test you haven't seen fail**

---

## Evidence Base

This specification structure is based on the LinkedIn Intelligence Pipeline refactor:

- **17 hours** actual implementation (vs 22-26 hours code-first)
- **Zero** integration surprises
- **Zero** context loss across 10 sessions
- **17/17** bugs fixed first try

---

**Created:** 2026-01-08
**Last Updated:** 2026-01-09
**Status:** Active
**Phases Complete:** 2 of 8 (Phase 1 Core, Phase 2 Tauri)
**Phases Specified:** 3 of 8 (Phases 1-3 fully specified)
**Phases Deferred:** 5 (Phases 4-8, per implementation status)
