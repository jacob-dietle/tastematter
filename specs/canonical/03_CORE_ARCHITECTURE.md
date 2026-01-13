---
title: "Context-OS Core Architecture"
type: architecture-spec
created: 2026-01-08
last_updated: 2026-01-09
status: approved
foundation:
  - "[[canonical/00_VISION]]"
  - "[[canonical/01_PRINCIPLES]]"
  - "[[canonical/02_ROADMAP]]"
  - "[[apps/context_os_events/specs/context_os_intelligence/01_ARCHITECTURE_GUIDE.md]]"
  - "[[apps/context_os_events/specs/context_os_intelligence/02_INDEX_STRUCTURES.md]]"
  - "[[.claude/skills/technical-architecture-engineering/SKILL.md]]"
  - "[[.claude/skills/observability-engineering/SKILL.md]]"
related:
  - "[[context_packages/09_2026-01-08_UNIFIED_CORE_ARCHITECTURE]]"
tags:
  - tastematter
  - context-os-core
  - architecture
  - canonical
---

# Context-OS Core Architecture Specification

## Executive Summary

This specification defines the `context-os-core` Rust library architecture that serves as the unified foundation for both Tastematter (human UI) and the context-os CLI (agent interface). The design eliminates the 18-second query latency caused by the current Python CLI architecture while establishing foundations for future extensibility including distribution and peer-to-peer sync.

**Key Outcomes:**
- Query latency: 18,000ms → <100ms (180x improvement)
- Single query logic (DRY across Tauri and CLI)
- Agent-controllable UI state
- Future-proof foundations for distribution

---

## Problem Statement

### Measured Bottleneck

```bash
$ time context-os query flex --time 7d --limit 5 --format json
real    0m18.239s   # ACTUAL measured latency
user    0m0.124s
sys     0m0.061s
```

[VERIFIED: Timing measurement 2026-01-08]

### Root Cause Analysis

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

**Root cause:** Python CLI loads ENTIRE 1.8MB database into memory on every query via `ContextIndex.load()` method.

[VERIFIED: `context_index.py`:610-630 - load() performs 3 full-table SELECTs]
[VERIFIED: `query_engine.py` operates on in-memory index, not SQL]

### Why Current Architecture is Wrong

```
Current Flow (Every Query):

Tastematter ──Command::new()──► Python CLI ──load ALL──► SQLite
                 │                    │
                 │ ~500ms spawn       │ ~15s load entire DB
                 │                    │ ~50ms actual query
                 │                    │
                 └────────────────────┴──► 18 seconds total
```

The Python CLI was designed for human usage (run once, wait, see result). Using it as an API between app and database is an **unnecessary abstraction layer** that:

1. Spawns a new process for every query (expensive on Windows)
2. Loads entire database into Python memory (wasteful)
3. Duplicates query logic (Tauri → CLI → Python → SQL)

[INFERRED: CLI predates Tastematter; never intended for high-frequency programmatic access]

---

## Target Architecture

### System Overview

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
         │ (in-process, zero overhead)          │ (~100μs latency)
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
         │  │     - Valid transitions      │   │
         │  └──────────────────────────────┘   │
         │                                      │
         │  ┌──────────────────────────────┐   │
         │  │     Event Bus                │   │
         │  │     - State change events    │   │
         │  │     - Data update events     │   │
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
│       ├── ipc.rs             # Socket server for CLI
│       ├── storage.rs         # Storage trait abstraction
│       └── config.rs          # Configuration management
│
├── tastematter/               # Tauri app (human UI)
│   ├── src-tauri/
│   │   ├── Cargo.toml         # depends on context-os-core
│   │   └── src/
│   │       └── commands.rs    # Thin wrapper calling core
│   └── src/                   # Svelte frontend (unchanged)
│
└── context_os_events/         # Python (becomes thin wrapper)
    └── src/
        └── cli.py             # CLI calls context-os-core via socket
```

---

## Design Decisions

### Decision 1: No Redis / External Cache

**Decision:** Use in-memory Rust cache + SQLite. No Redis or external cache service.

**Background:**

The question arose: should we add Redis for caching query results? This is a common pattern in web applications with high query load.

**Analysis:**

| Data | Access Pattern | Size | Recommended Tier |
|------|----------------|------|------------------|
| Chain bloom filters | Every query (membership check) | ~50KB total | HOT - always in memory |
| Active chains metadata | Most queries | ~10KB | HOT - always in memory |
| Recent query results | Variable, repeated queries | ~1MB (10 cached) | WARM - LRU with TTL |
| Full file tree | On file view load | ~500KB | COLD - SQLite fetch |
| Co-access matrix | On file click | ~1MB | COLD - SQLite fetch |

**Total in-memory requirement:** ~2MB

**Why NOT Redis:**

1. **Operational complexity:** Another service to install, configure, and keep running
2. **Network latency:** Even localhost Redis adds ~0.5ms per call vs ~0μs for in-process
3. **Overkill for data size:** 2MB fits easily in process memory
4. **SQLite is already fast:** Indexed queries complete in <50ms
5. **Single user:** No multi-instance cache invalidation concerns

**Reference:** Five-Minute Rule from [[02_FIVE_MINUTE_RULE.md]] - only cache if access frequency justifies memory cost. At 2MB total, in-process caching is trivially justified.

[VERIFIED: Database size 1.8MB measured 2026-01-08]

---

### Decision 2: Direct SQL Queries (Not Port Python Logic)

**Decision:** Write SQL queries directly in Rust. Do NOT port `query_engine.py`.

**Background:**

The Python `query_engine.py` is 960 lines of query logic. The obvious approach would be to translate this to Rust line-by-line.

**Why this is wrong:**

The Python code implements the wrong algorithm:
1. Load ENTIRE database into memory (`ContextIndex.load()`)
2. Filter with Python loops (`_slice()` method)
3. Sort in Python
4. Return results

This is O(n) on every query where n = entire database size.

**Correct algorithm:**

Query SQLite directly with indexes:
1. Execute parameterized SQL query
2. SQLite uses indexes (B-tree, O(log n))
3. Only matching rows transferred
4. Already sorted by SQLite

**Effort comparison:**

| Approach | Effort | Result |
|----------|--------|--------|
| Port Python line-by-line | Days (960 lines to understand and translate) | Fast wrong algorithm |
| Direct SQL | Hours (~200 lines SQL + Rust type mapping) | Correct algorithm |

**Key insight:** The database already has all required indexes. The problem isn't missing indexes - it's that we're not using them.

[VERIFIED: [[02_INDEX_STRUCTURES.md]]:128-154 shows indexes for file_path, session_id, timestamp, chain_id]
[VERIFIED: [[02_INDEX_STRUCTURES.md]]:763-775 shows <10ms performance targets]

---

### Decision 3: Shared UI State Machine

**Decision:** UI state (current view, filters, selections) lives in `context-os-core`, not only in Tauri frontend.

**Background:**

From [[02_ROADMAP.md]] Phase 3:
- Agent needs to navigate Tastematter UI programmatically
- "context-os ui navigate --view timeline --chain abc123"
- Human should see smooth animation when agent navigates

**Why state must be in core:**

If UI state lives only in Svelte/Tauri, the CLI has no way to:
- Know current view state
- Trigger navigation
- Highlight files
- Coordinate with human user

**State machine design:**

```rust
pub struct UiState {
    pub current_view: View,           // Files | Timeline | Sessions
    pub time_range: TimeRange,        // 7d | 14d | 30d | custom
    pub selected_chain: Option<ChainId>,
    pub highlighted_files: Vec<PathBuf>,
    pub pending_animation: Option<Animation>,
}

pub enum View {
    Files,
    Timeline,
    Sessions,
}

pub enum UiCommand {
    Navigate { view: View, time: TimeRange, chain: Option<ChainId> },
    Highlight { files: Vec<PathBuf>, duration: Duration },
    ClearHighlight,
}

pub enum UiEvent {
    ViewChanged { from: View, to: View },
    FilterChanged { time_range: TimeRange, chain: Option<ChainId> },
    HighlightStarted { files: Vec<PathBuf> },
    HighlightEnded,
}
```

**Flow:**

```
Agent CLI ──UiCommand::Navigate──► context-os-core
                                        │
                                        ▼
                                   UiState updated
                                        │
                                        ▼
                              UiEvent::ViewChanged emitted
                                        │
Tauri subscribes ◄──────────────────────┘
        │
        ▼
   Svelte animates transition
```

[VERIFIED: [[02_ROADMAP.md]]:329-420 specifies agent UI control requirements]

---

### Decision 4: IPC via Local Socket (CLI to Core)

**Decision:** CLI communicates with `context-os-core` via local socket (Unix domain socket on Linux/Mac, named pipe on Windows).

**Background:**

Two consumers access the core:
1. **Tauri:** Same process, Rust-to-Rust calls (zero overhead)
2. **CLI:** Separate process, needs IPC

**Options considered:**

| Mechanism | Latency | Complexity | Cross-platform |
|-----------|---------|------------|----------------|
| Shared memory | ~1μs | High | Hard (platform-specific APIs) |
| Local socket | ~100μs | Low | Good (Rust std::os) |
| HTTP localhost | ~1ms | Medium | Good |
| Named pipe (Windows) | ~100μs | Medium | Windows only |

**Decision rationale:**

- **Socket latency (~100μs)** is invisible compared to 18 seconds saved
- **Simple protocol:** JSON-RPC over socket
- **Cross-platform:** Unix sockets + Windows named pipes via Rust abstractions
- **Debuggable:** Can inspect with netcat/socat

**Protocol: JSON-RPC 2.0**

```json
// Request
{
  "jsonrpc": "2.0",
  "method": "query_flex",
  "params": { "time_range": "7d", "limit": 100 },
  "id": 1
}

// Success response
{
  "jsonrpc": "2.0",
  "result": { "files": [...], "total": 42 },
  "id": 1
}

// Error response
{
  "jsonrpc": "2.0",
  "error": { "code": -32600, "message": "Invalid request" },
  "id": 1
}
```

**Why JSON-RPC:**
- Standard protocol (tooling exists)
- Clear error handling semantics
- Self-documenting (method names)
- Grep-able for debugging

---

### Decision 5: TTL-Based Cache Invalidation

**Decision:** Use time-based expiration (TTL = 5 minutes) for query cache. No event-based invalidation initially.

**Background:**

When daemon writes new data to SQLite, cached query results become stale. Options:

| Strategy | Pros | Cons |
|----------|------|------|
| Invalidate all on write | Simple | Wasteful - cold cache after every daemon write |
| Invalidate affected queries | Efficient | Complex - must track query→data dependencies |
| TTL-based (5 min) | Simple, predictable | May serve stale data briefly |

**Decision rationale:**

1. **Daemon writes infrequently:** File watches and git syncs happen sporadically, not continuously
2. **5-minute staleness is acceptable:** This is a visualization tool, not a trading system
3. **Simplicity wins:** Tracking query→data dependencies is complex and error-prone
4. **Can always query fresh:** Force-refresh option bypasses cache

**TTL explanation:**

```
Query: "Files from last 7 days"

Check cache:
├── Cached 2 minutes ago (TTL=5min) → Return cached (instant)
├── Cached 8 minutes ago (TTL=5min) → Cache expired, query SQLite (~50ms)
└── Not in cache → Query SQLite, cache result with 5-min TTL
```

**Future consideration:** If fresher data is needed, add targeted invalidation by listening to daemon's `sync_complete` events.

---

### Decision 6: Structured Logging with Correlation IDs

**Decision:** All components emit structured JSONL events with correlation IDs that trace requests across boundaries.

**Background:**

From [[observability-engineering]] skill (Charity Majors / Honeycomb patterns):
- Wide structured events, not printf debugging
- Correlation ID follows request across components
- JSONL enables grep-based debugging

**Log format:**

```json
{"timestamp":"2026-01-08T12:00:00.000Z","correlation_id":"abc123","component":"cli","operation":"parse_args","duration_ms":1}
{"timestamp":"2026-01-08T12:00:00.001Z","correlation_id":"abc123","component":"core","operation":"query_flex","duration_ms":45,"result_count":100}
{"timestamp":"2026-01-08T12:00:00.046Z","correlation_id":"abc123","component":"core","operation":"cache_miss","query_hash":"xyz789"}
```

**Debugging workflow:**

```bash
# Trace full request with one grep:
grep "abc123" ~/.context-os/logs/*.jsonl | jq '.'

# Find all errors:
cat ~/.context-os/logs/*.jsonl | jq 'select(.success == false)'

# Find slow operations:
cat ~/.context-os/logs/*.jsonl | jq 'select(.duration_ms > 100)'
```

**Log file location:** `~/.context-os/logs/YYYY-MM-DD.jsonl`

[VERIFIED: Pattern from [[.claude/skills/observability-engineering/SKILL.md]]]

---

### Decision 7: Event Bus for Cross-Component Communication

**Decision:** Pub/sub event bus in core for daemon→core→Tauri coordination.

**Background:**

Data flow requires coordination:
```
Daemon indexes new session
    ↓
Core needs to invalidate cache
    ↓
Tauri needs to show "New data" indicator
```

**Without event bus:** Tauri would need to poll for changes (wasteful, latency)

**With event bus:**

```rust
pub enum CoreEvent {
    DataUpdated { source: DataSource, timestamp: DateTime<Utc> },
    QueryCompleted { correlation_id: String, duration_ms: u64 },
    CacheInvalidated { reason: InvalidationReason },
    UiStateChanged { event: UiEvent },
}

// Daemon emits
event_bus.publish(CoreEvent::DataUpdated { source: GitSync, ... });

// Core subscribes
event_bus.subscribe(|e| match e {
    DataUpdated { .. } => invalidate_cache(),
    _ => {}
});

// Tauri subscribes (forwarded from core)
event_bus.subscribe(|e| match e {
    DataUpdated { .. } => show_refresh_indicator(),
    UiStateChanged { event } => animate_transition(event),
    _ => {}
});
```

---

## Service Coordination

### Startup Sequence

```
┌─────────────────────────────────────────────────────────────────────┐
│                    SERVICE DEPENDENCY GRAPH                          │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│   SQLite DB        ← Must exist (or be created)                     │
│       ↑                                                              │
│       │ writes                                                       │
│   Daemon           ← Should be running (indexing)                   │
│       ↑                                                              │
│       │ notifies via socket                                         │
│   context-os-core  ← Must be running for queries                    │
│       ↑            ↑                                                │
│   Tauri         CLI  ← Either can start core                        │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

### Startup Matrix

| Scenario | Behavior | Implementation |
|----------|----------|----------------|
| Tastematter starts, no DB | Create empty DB, show "No data" | Core creates DB on init |
| Tastematter starts, DB exists, no daemon | Works - queries existing data | Core works standalone |
| Tastematter starts, daemon not running | Start daemon as subprocess | Tauri spawns daemon |
| Daemon starts, core not running | Fine - daemon writes to SQLite | Independent operation |
| CLI query, core not running | Start core, then query | CLI starts core via socket |
| Two Tastematter instances | Reject second | Lock file check |

### Tastematter Startup Flow

```
1. Check lock file (~/.context-os/tastematter.lock)
   ├── Locked → Show "Already running", exit
   └── Not locked → Create lock, continue

2. Initialize context-os-core (in-process)
   ├── Open SQLite (create if missing)
   ├── Run schema migrations if needed
   ├── Load hot cache (bloom filters, active chains)
   └── Start socket server for CLI

3. Check daemon status
   ├── Read ~/.context-os/daemon.pid
   ├── Verify process exists (kill -0 or Windows equivalent)
   ├── Running → Subscribe to daemon events
   └── Not running → Spawn daemon as detached subprocess

4. Ready to serve UI
   └── Emit CoreEvent::Ready
```

### Error Handling Contract

```
Core → Tauri:
  - Rust Result<T, CoreError> → Tauri command returns Result
  - Frontend displays error in UI toast/banner

Core → CLI (via socket):
  - JSON-RPC error: { "error": { "code": N, "message": "..." } }
  - CLI exits with non-zero code
  - Error written to stderr

Event Bus:
  - Events are notifications, not requests
  - No error propagation via events
  - Errors logged with correlation ID

Error Types:
  - DatabaseError: SQLite errors, migrations failed
  - QueryError: Invalid query parameters
  - IpcError: Socket communication failed
  - ConfigError: Invalid configuration
```

---

## Future-Proofing

### Evolution Path

```
Stage 1: Personal Tool (Current Target)
├── SQLite local only
├── No network
├── No auth
└── No analytics

Stage 2: Personal Tool with Telemetry
├── Local-first (still works offline)
├── Opt-in telemetry to central server
├── Usage patterns, errors, performance
└── Privacy-preserving (no file contents)

Stage 3: Distributed Free Software
├── Other people install it
├── Still local-first
├── Optional cloud sync
├── Crash reporting, update mechanism

Stage 4: Peer-to-Peer Sharing
├── E2E encrypted sync between instances
├── Conflict resolution (CRDTs)
├── Peer discovery
└── Zero-knowledge sync
```

### Foundation for Future Stages

#### 1. Storage Trait Abstraction

```rust
/// Abstract storage layer for future sync capabilities
pub trait ContextStore: Send + Sync {
    fn query(&self, spec: QuerySpec) -> Result<QueryResult, StoreError>;
    fn insert(&self, events: Vec<Event>) -> Result<(), StoreError>;
    fn subscribe(&self, callback: Box<dyn Fn(StoreEvent)>) -> Subscription;
}

// Current implementation
pub struct SqliteStore {
    conn: Connection,
    cache: Cache,
}

// Future implementation for sync
pub struct SyncedStore {
    local: SqliteStore,
    remote: SyncClient,
    conflict_resolver: ConflictResolver,
}
```

#### 2. UUID-Based Event IDs

```rust
/// Events use UUIDs, not auto-increment IDs
/// UUIDs work across machines for future sync
pub struct Event {
    pub id: Uuid,                    // NOT i64 auto-increment
    pub timestamp: DateTime<Utc>,    // With timezone for sync
    pub source_device: Option<Uuid>, // For future multi-device
    pub data: EventData,
}
```

**Why UUIDs:**
- Auto-increment IDs collide when syncing between machines
- UUIDs are globally unique
- Can be generated offline

#### 3. Schema Versioning from Day 1

```sql
-- Schema version table
CREATE TABLE IF NOT EXISTS schema_version (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL,
    description TEXT
);

-- Migration pattern
-- migrations/001_initial.sql
-- migrations/002_add_sync_metadata.sql
-- migrations/003_add_conflict_markers.sql
```

```rust
pub fn migrate(conn: &Connection) -> Result<(), MigrationError> {
    let current = get_current_version(conn)?;
    let migrations = get_pending_migrations(current)?;

    for migration in migrations {
        conn.execute_batch(&migration.sql)?;
        record_migration(conn, migration.version)?;
    }

    Ok(())
}
```

#### 4. Telemetry Interface

```rust
/// Telemetry trait - implemented even if unused
pub trait Telemetry: Send + Sync {
    fn record(&self, event: TelemetryEvent);
    fn flush(&self);
}

#[derive(Debug)]
pub struct TelemetryEvent {
    pub category: TelemetryCategory,
    pub action: String,
    pub value: Option<i64>,
    pub metadata: HashMap<String, String>,
}

// Default: no-op implementation
pub struct NoTelemetry;
impl Telemetry for NoTelemetry {
    fn record(&self, _: TelemetryEvent) {}
    fn flush(&self) {}
}

// Future: HTTP implementation
pub struct HttpTelemetry {
    endpoint: Url,
    buffer: Mutex<Vec<TelemetryEvent>>,
}
```

#### 5. Config-Driven Behavior

```yaml
# ~/.context-os/config.yaml
version: 2

storage:
  type: local          # Future: "synced"
  path: ~/.context-os/data

cache:
  hot_size_mb: 10
  warm_ttl_seconds: 300

telemetry:
  enabled: false       # User controls
  endpoint: null       # Future: your server

sync:                  # Future feature
  enabled: false
  peers: []
  encryption_key: null

logging:
  level: info
  path: ~/.context-os/logs
  max_size_mb: 100
  retention_days: 30
```

### Risk Mitigation

| Risk | Mitigation | Implementation |
|------|------------|----------------|
| Schema changes break old data | Migration system | `schema_version` table + numbered migrations |
| Auto-increment IDs break sync | UUIDs everywhere | `Uuid` type for all record IDs |
| Hardcoded paths break distribution | Config-driven | All paths from `config.yaml` |
| No telemetry = flying blind | Interface ready | `Telemetry` trait with no-op default |
| Tight coupling = hard to extend | Storage abstraction | `ContextStore` trait |
| SQLite locks = P2P conflict | Plan for CRDTs | Document conflict resolution strategy |

---

## Type Contracts

### Query Types

```rust
/// Query specification matching hypercube model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuerySpec {
    pub time_range: TimeRange,
    pub chain_filter: Option<ChainId>,
    pub path_pattern: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub aggregations: Vec<Aggregation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TimeRange {
    Days(u32),           // Last N days
    Range { start: DateTime<Utc>, end: DateTime<Utc> },
    All,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Aggregation {
    Count,
    SumAccessCount,
    GroupByDay,
    GroupByChain,
}
```

### Query Results

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub files: Vec<FileRecord>,
    pub total_count: usize,
    pub aggregations: HashMap<String, AggregationResult>,
    pub query_time_ms: u64,
    pub cache_hit: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRecord {
    pub id: Uuid,
    pub path: PathBuf,
    pub access_count: u32,
    pub last_accessed: DateTime<Utc>,
    pub chains: Vec<ChainId>,
}
```

### UI State Types

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiState {
    pub current_view: View,
    pub time_range: TimeRange,
    pub selected_chain: Option<ChainId>,
    pub highlighted_files: Vec<PathBuf>,
    pub highlight_expires: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum View {
    Files,
    Timeline,
    Sessions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UiCommand {
    Navigate { view: View, time: TimeRange, chain: Option<ChainId> },
    Highlight { files: Vec<PathBuf>, duration_secs: u32 },
    ClearHighlight,
    Refresh,
}
```

### Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Query error: {message}")]
    Query { message: String },

    #[error("IPC error: {0}")]
    Ipc(#[from] std::io::Error),

    #[error("Config error: {message}")]
    Config { message: String },

    #[error("Migration error: version {version}, {message}")]
    Migration { version: u32, message: String },
}
```

---

## Implementation Phases

### Phase 0a: Core Foundation (This Sprint)

**Deliverables:**
1. `apps/context-os-core/` Rust crate structure
2. Direct SQL query engine (replace Python `query_engine.py` algorithm)
3. Basic cache layer (hot + warm tiers)
4. Integration with Tastematter (replace `Command::new`)

**Success criteria:**
- Query latency < 100ms (measured)
- All existing Tauri commands working
- Tests passing

**Estimated effort:** 2-3 days

### Phase 0b: CLI Integration

**Deliverables:**
1. Socket server in core
2. JSON-RPC protocol implementation
3. Python CLI wrapper calling socket
4. Correlation ID logging

**Success criteria:**
- CLI queries via socket < 200ms
- Same results as current CLI
- Logs traceable via correlation ID

**Estimated effort:** 1-2 days

### Phase 0c: Service Coordination

**Deliverables:**
1. Lock file for single instance
2. Daemon startup from Tauri
3. Event bus for daemon→core→Tauri
4. Schema migration system

**Success criteria:**
- Tastematter starts daemon if not running
- New data triggers UI update
- Migrations run automatically

**Estimated effort:** 1 day

---

## Success Metrics

**Performance:**
- Query latency: <100ms (was 18,000ms)
- Cache hit rate: >80% for repeated queries
- Socket IPC: <200μs per call

**Reliability:**
- Zero query logic duplication (single source in core)
- All errors traceable via correlation ID
- Schema migrations automatic and reversible

**Future-readiness:**
- Storage trait abstraction in place
- UUID-based record IDs
- Config-driven behavior
- Telemetry interface defined

---

## References

**Architecture foundations:**
- [[01_ARCHITECTURE_GUIDE.md]] - Two-layer architecture philosophy
- [[02_INDEX_STRUCTURES.md]] - SQLite schema and indexes
- [[02_ROADMAP.md]] - Phase definitions and requirements

**Skills applied:**
- [[technical-architecture-engineering]] - Jeff Dean, Brendan Gregg principles
- [[observability-engineering]] - Charity Majors structured logging
- [[specification-driven-development]] - This spec methodology

**Context packages:**
- [[09_2026-01-08_UNIFIED_CORE_ARCHITECTURE]] - Design session that produced this spec

---

**Specification Status:** APPROVED
**Created:** 2026-01-08
**Author:** Architecture planning session
**Next Action:** Begin Phase 0a implementation with TDD

---

## Implementation Status (2026-01-09)

Phase 0 Performance Foundation is COMPLETE. This section documents what was implemented vs deferred.

### Implemented ✅

| Component | Location | Notes |
|-----------|----------|-------|
| Query Engine | `apps/context-os/core/src/query.rs` | 4 functions (flex, timeline, sessions, chains), parameterized SQL, chain filtering |
| Type Contracts | `apps/context-os/core/src/types.rs` | All input/output types with serde |
| Storage Layer | `apps/context-os/core/src/storage.rs` | SQLite with sqlx pool, auto-discovery of DB path |
| Error Handling | `apps/context-os/core/src/error.rs` | CoreError + CommandError types |
| Tauri Integration | `apps/tastematter/src-tauri/src/commands.rs` | Thin wrappers calling QueryEngine |
| CLI Binary | `apps/context-os/core/src/main.rs` | clap-based, 4 query commands |
| CLI Wrapper | `tastematter.ps1` / `tastematter.cmd` | Repo root, works from anywhere |

**Performance achieved:** 1.5ms average query latency (target was <100ms)

### Deferred (Not Required for Phase 0) ⏸️

| Component | Spec Section | Reason Deferred | When Needed |
|-----------|--------------|-----------------|-------------|
| Cache Layer | Decision 1 | Query latency already <2ms, caching unnecessary | If latency becomes issue |
| IPC Socket | Decision 4 | Built Rust CLI instead of Python wrapper | Phase 3 (agent UI control) |
| UI State Machine | Decision 3 | Frontend manages state adequately | Phase 3 (agent UI control) |
| Event Bus | Decision 7 | No daemon coordination yet | Phase 4 (Intelligent GitOps) |
| Structured Logging | Decision 6 | Basic Tauri logging sufficient | When debugging complex issues |

### Architecture Deviation from Spec

**Spec proposed (Decision 4):**
```
Python CLI ──IPC socket──► context-os-core ──► SQLite
```

**Actual implementation:**
```
Rust CLI (clap) ──direct link──► QueryEngine ──► SQLite
```

**Why this deviation is acceptable:**
1. Direct linking has 0ms IPC overhead (vs ~100μs socket)
2. Python CLI wrapper not needed since we have pure Rust CLI
3. IPC socket can be added later for Phase 3 agent UI control
4. Simpler architecture with fewer moving parts

### Test State

- **Unit tests:** 7 passing
- **Integration tests:** 8 passing
- **Total:** 15 tests
- **Last verified:** 2026-01-09

### Next Phase Requirements

**Phase 1 (Stigmergic Display)** will need:
- Git integration (`git2` crate) - NOT in original spec
- Commit timeline view
- Agent/human attribution logic

**Phase 3 (Agent UI Control)** will need:
- UI State Machine (Decision 3)
- IPC Socket Server (Decision 4)

**Phase 4 (Intelligent GitOps)** will need:
- Event Bus (Decision 7)
- Daemon coordination

[VERIFIED: Implementation status from context package [[15_2026-01-09_PHASE0_COMPLETE]]]
