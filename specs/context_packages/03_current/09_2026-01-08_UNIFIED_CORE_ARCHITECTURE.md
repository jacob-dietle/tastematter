---
title: "Tastematter Context Package 09 - Unified Core Architecture"
package_number: 09

migrated_from: "apps/tastematter/specs/context_packages/09_2026-01-08_UNIFIED_CORE_ARCHITECTURE.md"
status: current
previous_package: "[[08_2026-01-07_SKILL_COMPLETE_PHASE0_READY]]"
related:
  - "[[apps/tastematter/specs/canonical/03_CORE_ARCHITECTURE.md]]"
  - "[[apps/tastematter/specs/canonical/02_ROADMAP.md]]"
  - "[[apps/context_os_events/specs/context_os_intelligence/01_ARCHITECTURE_GUIDE.md]]"
  - "[[apps/context_os_events/specs/context_os_intelligence/02_INDEX_STRUCTURES.md]]"
  - "[[apps/tastematter/src-tauri/src/commands.rs]]"
  - "[[apps/context_os_events/src/context_os_events/query_engine.py]]"
  - "[[.claude/skills/technical-architecture-engineering/SKILL.md]]"
  - "[[.claude/skills/observability-engineering/SKILL.md]]"
tags:
  - context-package
  - tastematter
  - architecture
  - context-os-core
  - design-decisions
---

# Tastematter - Context Package 09: Unified Core Architecture

## Executive Summary

**Strategic architecture session → Canonical spec created.** Analyzed the 18-second query latency, traced root cause to Python CLI architecture, and designed a unified `context-os-core` Rust library that serves both Tauri (human UI) and CLI (agent interface).

**Architecture now documented in [[canonical/03_CORE_ARCHITECTURE.md]]** - the authoritative specification with:
- 7 design decisions with full reasoning
- Service coordination matrix
- Type contracts
- Future-proofing for distribution/P2P
- Implementation phases

**Next agent: Read canonical spec, begin Phase 0a implementation with TDD.**

---

## Problem Analysis

### The Measured Bottleneck

We measured actual CLI query latency:

```
$ time context-os query flex --time 7d --limit 5 --format json
real    0m18.239s   ← ACTUAL (not 5s as previously claimed)
user    0m0.124s
sys     0m0.061s
```

### Root Cause Breakdown

```
CLI Query Time (18 seconds total):
├── Process spawn (.cmd → Python)     ~500ms
├── Python imports                    ~2-3s
├── ContextIndex.load() ─────────────────────┐
│   ├── SELECT ALL chains              ~1s   │
│   ├── SELECT ALL file_access         ~5s   │ ~15s (!!!)
│   ├── SELECT ALL temporal_buckets    ~1s   │
│   └── Deserialize JSON, build index  ~8s   │
└── QueryEngine.execute()              ~50ms  (actual query is fast!)
```

**Key insight:** The problem is NOT the query execution (50ms). The problem is loading the ENTIRE 1.8MB database into Python memory before every query.

[VERIFIED: `context_index.py`:610-630 shows `load()` does 3 full-table SELECTs]
[VERIFIED: `query_engine.py` operates on in-memory `ContextIndex`, not SQL]

### Why the Current Architecture is Wrong

```
Current Flow (Every Query):

Tastematter ──Command::new()──► Python CLI ──load ALL──► SQLite
                 │                    │
                 │ ~500ms spawn       │ ~15s load entire DB
                 │                    │ ~50ms actual query
                 │                    │
                 └────────────────────┴──► 18 seconds total
```

The Python CLI was designed for human usage (run, wait, see result). Using it as an API between app and database is an **unnecessary abstraction layer** that:

1. Spawns a process (expensive on Windows)
2. Loads everything into Python memory (wasteful)
3. Then finally runs the query

[INFERRED: CLI design predates Tastematter; was never intended for high-frequency programmatic access]

---

## Global Context

### The Two-Layer Architecture (Existing)

From [[01_ARCHITECTURE_GUIDE.md]], the system already has a sound conceptual architecture:

```
┌─────────────────────────────────────────────────────────────┐
│  LAYER 2: INTELLIGENT AGENT                                 │
│  • Judgment, natural language, context-aware decisions      │
│  • Tastematter (human) + Claude Code (agent)                │
└─────────────────────────────────────────────────────────────┘
                              │ Queries
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  LAYER 1: DETERMINISTIC INDEX                               │
│  • Pre-computed indexes, bloom filters, aggregations        │
│  • SQLite database with proper indexes                      │
│  • NO LLM - pure parsing and computation                    │
└─────────────────────────────────────────────────────────────┘
```

**The indexes are already fast.** SQLite can answer queries in <50ms with existing indexes. We just need to access them directly.

[VERIFIED: [[02_INDEX_STRUCTURES.md]]:763-775 shows <10ms targets for indexed operations]

### What's Missing: Shared Core

The architecture has TWO consumers of Layer 1:
1. **Tastematter** (human UI) - needs <100ms queries
2. **Agent CLI** (Claude Code) - needs to control UI

Currently they're completely decoupled. There's no shared state, no way for agent to tell Tastematter "navigate here."

---

## Architecture Design: Unified Core

### Target Architecture

```
┌─────────────────┐                    ┌─────────────────┐
│   Tauri App     │                    │   Agent CLI     │
│   (Human UI)    │                    │   (context-os)  │
│                 │                    │                 │
│   Subscribes to │                    │   Mutates       │
│   state changes │                    │   state via     │
│   via events    │                    │   commands      │
└────────┬────────┘                    └────────┬────────┘
         │                                      │
         │ Rust bindings                        │ IPC socket
         │ (in-process)                         │ (local)
         │                                      │
         └──────────────────┬───────────────────┘
                            │
         ┌──────────────────▼──────────────────┐
         │         context-os-core              │
         │         (Shared Rust Library)        │
         │                                      │
         │  ┌──────────────────────────────┐   │
         │  │     Query Engine             │   │
         │  │     - Direct SQL queries     │   │
         │  │     - <50ms response         │   │
         │  └──────────────────────────────┘   │
         │                                      │
         │  ┌──────────────────────────────┐   │
         │  │     Cache Layer              │   │
         │  │     - Hot: bloom filters     │   │
         │  │     - Warm: LRU query cache  │   │
         │  │     - Cold: SQLite fetch     │   │
         │  └──────────────────────────────┘   │
         │                                      │
         │  ┌──────────────────────────────┐   │
         │  │     UI State Machine         │   │
         │  │     - Current view           │   │
         │  │     - Filters (time, chain)  │   │
         │  │     - Selections             │   │
         │  │     - Valid transitions      │   │
         │  └──────────────────────────────┘   │
         │                                      │
         │  ┌──────────────────────────────┐   │
         │  │     Event Bus                │   │
         │  │     - State change events    │   │
         │  │     - Data update events     │   │
         │  │     - Cross-component comms  │   │
         │  └──────────────────────────────┘   │
         │                                      │
         │  ┌──────────────────────────────┐   │
         │  │     Logging Service          │   │
         │  │     - Correlation IDs        │   │
         │  │     - Wide structured events │   │
         │  │     - JSONL output           │   │
         │  └──────────────────────────────┘   │
         │                                      │
         └──────────────────┬───────────────────┘
                            │
         ┌──────────────────▼──────────────────┐
         │              SQLite                  │
         │         (already indexed)            │
         └──────────────────┬──────────────────┘
                            │ writes
         ┌──────────────────▼──────────────────┐
         │     Daemon (Background Indexer)      │
         │     - Watches files, git, sessions  │
         │     - Pushes to SQLite              │
         │     - Emits "data updated" events   │
         └─────────────────────────────────────┘
```

### File Structure

```
apps/
├── context-os-core/           # Shared Rust library (NEW)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs             # Public API
│       ├── query.rs           # Query engine (direct SQL)
│       ├── cache.rs           # Hot/warm/cold cache tiers
│       ├── ui_state.rs        # UI state machine
│       ├── events.rs          # Event bus
│       ├── logging.rs         # Structured logging
│       └── ipc.rs             # Socket server for CLI
│
├── tastematter/               # Tauri app (human UI)
│   ├── src-tauri/
│   │   └── Cargo.toml         # depends on context-os-core
│   └── src/                   # Svelte frontend
│
└── context_os_events/         # Python (becomes thin wrapper)
    └── src/
        └── cli.py             # CLI calls context-os-core via socket
```

---

## Design Decisions & Reasoning

### Decision 1: No Redis / External Cache

**Decision:** Use in-memory Rust cache + SQLite. No Redis.

**Reasoning:**

| Data | Access Pattern | Size | Cache Tier |
|------|----------------|------|------------|
| Chain bloom filters | Every query | ~50KB total | HOT - always in memory |
| Active chains metadata | Most queries | ~10KB | HOT - always in memory |
| Recent query results | Variable | ~1MB (10 cached) | WARM - LRU with 5min TTL |
| Full file tree | On file view | ~500KB | COLD - SQLite fetch (<50ms) |
| Co-access matrix | On file click | ~1MB | COLD - SQLite fetch (<50ms) |

**Total in-memory:** ~2MB. SQLite handles cold queries in <50ms with existing indexes.

**Why not Redis:**
- Adds operational complexity (another service to run)
- Network round-trip adds latency
- SQLite already indexed for <50ms queries
- Total hot data fits easily in process memory

[INFERRED: Five-Minute Rule from [[02_FIVE_MINUTE_RULE.md]] - cache only if access frequency > 1/15s]

### Decision 2: Query SQLite Directly (Not Port Python Logic)

**Decision:** Write SQL queries in Rust. Don't port `query_engine.py`.

**Reasoning:**

The Python `query_engine.py` (960 lines) is inefficient because it:
1. Loads ENTIRE database into memory
2. Filters with Python loops
3. Sorts in Python

This is the wrong algorithm. Porting wrong algorithm to Rust gives you fast wrong algorithm.

**Better approach:** Query SQLite directly. The database already has:
- Indexed queries at <10ms
- Pre-computed aggregations
- Proper query planning

**Estimated effort:**
- Port Python: Days (960 lines of logic to understand and translate)
- Direct SQL: Hours (~200 lines of SQL + Rust type mapping)

[VERIFIED: [[02_INDEX_STRUCTURES.md]]:128-154 shows all required indexes exist]

### Decision 3: Shared UI State Machine

**Decision:** UI state lives in `context-os-core`, not just Tauri.

**Reasoning:**

From [[02_ROADMAP.md]] Phase 3 requirements:
- Agent needs to navigate Tastematter UI
- "context-os ui navigate --view timeline --chain abc123"
- Human should see smooth animation when agent navigates

This requires:
- UI state defined in shared core
- CLI can mutate state via commands
- Tauri subscribes to state changes

```rust
// context-os-core/src/ui_state.rs
pub struct UiState {
    pub current_view: View,           // Files | Timeline | Sessions
    pub time_range: TimeRange,        // 7d | 14d | 30d | custom
    pub selected_chain: Option<ChainId>,
    pub highlighted_files: Vec<PathBuf>,
}

pub enum UiCommand {
    Navigate { view: View, time: TimeRange, chain: Option<ChainId> },
    Highlight { files: Vec<PathBuf>, duration: Duration },
}
```

[VERIFIED: [[02_ROADMAP.md]]:329-420 specifies agent UI control requirements]

### Decision 4: IPC via Local Socket (CLI to Core)

**Decision:** CLI communicates with `context-os-core` via local Unix socket (Windows named pipe).

**Reasoning:**

- Tauri embeds `context-os-core` directly (Rust-to-Rust, zero overhead)
- CLI is a separate process, needs IPC
- Socket is ~100μs per call (vs 18s current)
- Simple request/response protocol (JSON-RPC style)

**Alternative considered:** Shared memory
- More complex
- Marginal benefit over socket for our message sizes
- Socket is simpler and sufficient

[INFERRED: Architecture skill IPC pattern selection - same machine = socket (~100μs)]

### Decision 5: Structured Logging with Correlation IDs

**Decision:** All components log structured JSONL events with correlation IDs.

**Reasoning:**

From [[observability-engineering]] skill:
- Wide structured events, not printf debugging
- Correlation ID follows request across components
- JSONL enables grep-based debugging

```
# Trace full request with one grep:
grep "abc12345" ~/.context-os/logs/*.jsonl | jq '.'

# Shows:
{"correlation_id":"abc12345","component":"cli","operation":"parse_args",...}
{"correlation_id":"abc12345","component":"core","operation":"query_flex",...}
{"correlation_id":"abc12345","component":"core","operation":"sql_query","duration_ms":12,...}
```

[VERIFIED: [[.claude/skills/observability-engineering/SKILL.md]] - Charity Majors patterns]

### Decision 6: Event Bus for Cross-Component Communication

**Decision:** Pub/sub event bus in core for daemon→core→Tauri flow.

**Reasoning:**

```
Daemon indexes new session
    ↓ Event::DataUpdated
Core invalidates cache
    ↓ Event::DataUpdated (forwarded)
Tauri shows "New data" indicator
```

Without this, Tauri would need to poll for changes. Event-driven is cleaner.

[INFERRED: Standard reactive architecture pattern]

---

## Summary: Architecture Decisions Table

| Question | Decision | Rationale |
|----------|----------|-----------|
| **Redis/external cache?** | No | <2MB hot data, SQLite indexed queries <50ms |
| **Port Python query logic?** | No | Wrong algorithm; direct SQL is simpler and faster |
| **Shared query logic?** | Yes - context-os-core | DRY, single source of truth |
| **UI state location?** | Core library | Agent CLI needs to control Tauri UI |
| **IPC CLI ↔ Core?** | Local socket | CLI is separate process, ~100μs latency |
| **IPC Tauri ↔ Core?** | Rust bindings | Same process, zero overhead |
| **Logging approach?** | Structured JSONL + correlation IDs | Grep-able, traceable |
| **Event system?** | Pub/sub in core | Daemon → Core → Tauri coordination |

---

## What This Architecture Enables

### Phase 0 (Performance): Trivial

Just query SQLite from Rust instead of spawning Python.

### Phase 3 (Agent UI Control): Trivial

UI state is in core. CLI sends `UiCommand::Navigate`, Tauri receives event.

```bash
# This becomes possible:
context-os ui navigate --view timeline --chain abc123
context-os ui highlight --files "*.py" --duration 3s
```

### Observability: Built-in

Every request traceable via correlation ID across all components.

### No Duplicate Logic (DRY)

- Query logic: Once, in context-os-core
- Cache logic: Once, in context-os-core
- UI state: Once, in context-os-core
- Both CLI and Tauri consume the same core

---

## Open Questions for Next Agent

### 1. IPC Protocol Details

Should CLI↔Core use:
- JSON-RPC? (standard, tooling exists)
- Cap'n Proto? (faster, more complex)
- Simple JSON lines? (simplest)

**Recommendation:** JSON-RPC for standardization, but review.

### 2. Cache Invalidation Strategy

When daemon writes new data:
- Invalidate all query cache? (simple but wasteful)
- Invalidate only affected queries? (complex but efficient)
- TTL-based expiration only? (simple, eventual consistency)

**Recommendation:** TTL-based (5 min) for simplicity. Revisit if needed.

### 3. Python CLI Migration Path

Options:
- Rewrite CLI in Rust (most consistent)
- Keep Python CLI, call Rust core via FFI (pragmatic)
- Keep Python CLI, call Rust core via socket (simplest transition)

**Recommendation:** Socket first (simplest), evaluate Rust rewrite later.

### 4. Error Handling Strategy

How should errors propagate:
- Core → Tauri (via Tauri command return types)
- Core → CLI (via socket response)
- Across event bus?

**Recommendation:** Review and document error contract.

---

## For Next Agent

### Context Chain

- Previous: [[08_2026-01-07_SKILL_COMPLETE_PHASE0_READY]] - Architecture skill completed
- This package: Strategic architecture design session
- Next: Review architecture, finalize decisions, begin implementation

### Start Here

1. Read this package thoroughly (you're doing it now)
2. Review the open questions above
3. If architecture is approved, proceed to implementation:
   - Create `apps/context-os-core/` Rust crate
   - Implement query engine with direct SQL
   - Integrate into Tastematter

### Verification Commands

```bash
# Current CLI latency (should be ~18s)
time context-os query flex --time 7d --limit 5 --format json

# Database location and size
ls -lh apps/context_os_events/data/context_os_events.db

# Existing Tauri commands to replace
grep -n "Command::new" apps/tastematter/src-tauri/src/commands.rs
```

### Key Files to Read

| File | Why |
|------|-----|
| [[02_INDEX_STRUCTURES.md]] | SQLite schema, indexes, performance targets |
| [[01_ARCHITECTURE_GUIDE.md]] | Two-layer architecture philosophy |
| [[commands.rs]]:88-155 | Current bottleneck code (Command::new) |
| [[query_engine.py]] | Python logic to NOT port (understand, don't copy) |

### Do NOT

- Port `query_engine.py` line-by-line (it's the wrong algorithm)
- Add Redis or external cache (SQLite is sufficient)
- Put UI state only in Tauri (agent needs to control it)
- Skip correlation IDs in logging (we need traceability)

### Key Insight

**The database is already fast. The architecture around it is slow.**

SQLite with existing indexes: <50ms
Python CLI loading everything: ~15 seconds

Fix the architecture, not the database.

---

**Package written:** 2026-01-08
**Session focus:** Strategic architecture analysis and design
**Key deliverable:** Unified `context-os-core` architecture with design rationale
