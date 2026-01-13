---
title: "Tastematter Context Package 07 - Architecture Skill Creation"
package_number: 07

migrated_from: "apps/tastematter/specs/context_packages/07_2026-01-07_ARCHITECTURE_SKILL_CREATION.md"
status: current
previous_package: "[[06_2026-01-07_CANONICAL_ENRICHMENT]]"
related:
  - "[[.claude/skills/technical-architecture-engineering/SKILL.md]]"
  - "[[apps/context_os_events/specs/context_os_intelligence/01_ARCHITECTURE_GUIDE.md]]"
  - "[[apps/context_os_events/specs/context_os_intelligence/02_INDEX_STRUCTURES.md]]"
  - "[[apps/tastematter/src-tauri/src/commands.rs]]"
tags:
  - context-package
  - tastematter
  - technical-architecture
  - rust-refactor
---

# Tastematter - Context Package 07: Architecture Skill Creation

## Executive Summary

Deep architecture analysis session for Phase 0 (Performance Foundation). Created `technical-architecture-engineering` skill with 5 expert POVs (Dean, Gregg, Kleppmann, Gray, Cantrill). SKILL.md is complete. **References files need to be created from content below.**

---

## Session Accomplishments

### 1. Architecture Analysis Complete

Analyzed current CLI + Tauri architecture:

**Current State (THE PROBLEM):**
```
Tastematter (Tauri)
    │
    │ Command::new("context-os.cmd")  ← SPAWNS PYTHON PROCESS
    │ ~5000ms per query
    ▼
Python CLI (context_os_events)
    │
    ▼
SQLite Database
```

**Target State (THE SOLUTION):**
```
Tastematter (Tauri)
    │
    │ Direct Rust function call  ← <1ms
    ▼
context-os-core (Rust library)
    │
    ▼
SQLite Database (same schema)
```

[VERIFIED: [[commands.rs]]:100-154 shows Command::new spawning Python]

### 2. Feature Planning Framework Applied

Used `feature-planning-and-decomposition` skill to validate:

| Question | Answer |
|----------|--------|
| What problem? | 5000ms view switches, violates <100ms principle |
| 80% use case? | Quick navigation between views |
| Success metric? | <100ms view switch, <50ms hot query |
| Existing code fix? | NO - architectural change required |

### 3. Technical Architecture Skill Created

**Location:** `.claude/skills/technical-architecture-engineering/`

**Status:**
- [x] SKILL.md complete (comprehensive, ~450 lines)
- [ ] references/00_LATENCY_NUMBERS.md - NOT CREATED
- [ ] references/01_USE_METHOD.md - NOT CREATED
- [ ] references/02_FIVE_MINUTE_RULE.md - NOT CREATED
- [ ] references/03_CONSISTENCY_MODELS.md - NOT CREATED
- [ ] references/04_RUST_PERFORMANCE.md - NOT CREATED
- [ ] references/05_DATABASE_PATTERNS.md - NOT CREATED

**Expert POVs included:**
1. Jeff Dean - Systems design, latency awareness
2. Brendan Gregg - Performance analysis, USE method
3. Martin Kleppmann - Data systems, consistency models
4. Jim Gray - Caching economics, five-minute rule
5. Bryan Cantrill - Debuggability, Rust pragmatism

---

## FOR NEXT AGENT: Complete the Skill

### Task: Create 6 Reference Files

The SKILL.md references these files but they don't exist yet. Create them from the content below.

**Directory:** `.claude/skills/technical-architecture-engineering/references/`

---

### FILE 1: references/00_LATENCY_NUMBERS.md

```markdown
# Latency Numbers Every Programmer Should Know

Source: Jeff Dean (Google), updated for 2024 hardware.

## The Numbers

| Operation | Latency | Notes |
|-----------|---------|-------|
| L1 cache reference | 0.5 ns | |
| Branch mispredict | 5 ns | |
| L2 cache reference | 7 ns | 14x L1 |
| Mutex lock/unlock | 25 ns | |
| L3 cache reference | 20 ns | 40x L1 |
| Main memory reference | 100 ns | 200x L1 |
| Compress 1KB with Snappy | 3,000 ns | 3 μs |
| Send 1KB over 1 Gbps network | 10,000 ns | 10 μs |
| Read 4KB randomly from SSD | 150,000 ns | 150 μs |
| Read 1MB sequentially from memory | 250,000 ns | 250 μs |
| Round trip within datacenter | 500,000 ns | 500 μs |
| Read 1MB sequentially from SSD | 1,000,000 ns | 1 ms |
| HDD seek | 10,000,000 ns | 10 ms |
| Read 1MB sequentially from HDD | 20,000,000 ns | 20 ms |
| Send packet CA→Netherlands→CA | 150,000,000 ns | 150 ms |

## Visual Scale

```
L1 cache      ■
L2 cache      ■■■■■■■■■■■■■■
RAM           ■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■ (200x)
SSD           ■■■■■■■■■■■■... (300,000x - off the chart)
Network       ■■■■■■■■■■■■... (300,000,000x - way off the chart)
```

## Design Implications

### 1. Memory Beats Disk by 1000x+

**Implication:** Cache aggressively in RAM for hot data.

```
Bad:  Read config from disk on every request (150μs)
Good: Read config once, cache in memory (0.1μs)
Savings: 1500x faster
```

### 2. Same-Machine Beats Cross-Machine by 1000x

**Implication:** Co-locate services that communicate frequently.

```
Bad:  Microservice call over network (500μs + serialization)
Good: In-process function call (<1μs)
Savings: 500x+ faster
```

### 3. Sequential Beats Random by 10x+

**Implication:** Batch operations, use sequential access patterns.

```
Bad:  1000 random 4KB reads = 150ms
Good: 1 sequential 4MB read = 4ms
Savings: 37x faster
```

### 4. Process Spawn is Expensive

**Implication:** Avoid spawning processes in the hot path.

```
Bad:  Spawn Python process per query (~500ms startup)
Good: Keep process alive, reuse connection
Better: Direct library call, no process boundary
```

## Application to Tastematter

Current state spawns Python CLI:
- Python startup: ~500ms
- Module imports: ~300ms
- DB connection: ~50ms
- Query: ~50ms
- Total: ~900ms minimum, often 5000ms

Target with Rust library:
- Function call: <1μs
- DB query (cached connection): ~5ms
- Total: <10ms

**Improvement: 100x-500x faster**

## References

- Original: Jeff Dean, "Numbers Everyone Should Know" (2009)
- Updated: Colin Scott, "Latency Numbers Every Programmer Should Know" (2020)
- Modern: Various benchmarks on NVMe, DDR5, etc.
```

---

### FILE 2: references/01_USE_METHOD.md

```markdown
# The USE Method - Performance Analysis

Source: Brendan Gregg, Netflix Performance Engineering

## What is USE?

For every resource, check:
- **U**tilization: % time the resource was busy
- **S**aturation: Degree to which resource has extra work it can't service (queue length)
- **E**rrors: Count of error events

## The Method

### Step 1: Identify Resources

Physical:
- CPUs
- Memory
- Storage devices
- Network interfaces

Software:
- Thread pools
- Connection pools
- File descriptors
- Locks/mutexes

### Step 2: For Each Resource, Measure USE

| Resource | Utilization | Saturation | Errors |
|----------|-------------|------------|--------|
| CPU | % busy | Run queue length | - |
| Memory | % used | Swap activity, OOM | Allocation failures |
| Disk | % busy | Wait queue length | Device errors |
| Network | % bandwidth | Dropped packets | Interface errors |
| Thread pool | Active/max | Queue depth | Rejected tasks |
| Connection pool | Used/max | Wait time | Timeouts |
| Locks | % contended | Wait queue | Deadlocks |

### Step 3: Interpret Results

```
High Utilization + Low Saturation = Working well, near capacity
High Utilization + High Saturation = BOTTLENECK - needs attention
Low Utilization + High Saturation = Something wrong (check for errors)
Low Utilization + Low Saturation = Not the problem, look elsewhere
```

## Checklist for Common Resources

### CPU
```bash
# Utilization
top -bn1 | grep "Cpu(s)"
mpstat 1

# Saturation
vmstat 1 | awk '{print $1}'  # r column = run queue
cat /proc/loadavg

# Errors
dmesg | grep -i "cpu"
```

### Memory
```bash
# Utilization
free -m
cat /proc/meminfo | grep -E "MemTotal|MemAvailable"

# Saturation
vmstat 1 | awk '{print $7, $8}'  # si, so = swap in/out
cat /proc/vmstat | grep -E "pgpgin|pgpgout"

# Errors
dmesg | grep -i "oom"
```

### Disk I/O
```bash
# Utilization
iostat -xz 1

# Saturation
iostat -xz 1 | awk '{print $10}'  # await = average wait time

# Errors
smartctl -a /dev/sda
dmesg | grep -i "error"
```

### Network
```bash
# Utilization
sar -n DEV 1
ip -s link

# Saturation
netstat -s | grep -i "drop"
ss -s

# Errors
ip -s link | grep -i error
netstat -s | grep -i error
```

## Software Resources

### SQLite Connection Pool (Rust/r2d2)
```rust
// Utilization: active connections / max connections
let state = pool.state();
let utilization = state.connections as f32 / state.max_size as f32;

// Saturation: threads waiting for connection
// (r2d2 doesn't expose this directly, use timeout as proxy)

// Errors: connection failures, timeouts
```

### Thread Pool
```rust
// Utilization: active threads / max threads
// Saturation: queued tasks
// Errors: panicked threads, rejected tasks
```

## When USE Shows No Bottleneck

If USE analysis shows all resources healthy but performance is still bad:

1. **Lock contention** - Not a resource in traditional sense
2. **Bad algorithm** - O(n²) hidden somewhere
3. **External dependency** - Waiting on API, database
4. **Configuration** - Wrong settings limiting throughput
5. **Code path issue** - Hot path has unnecessary work

## Application to Tastematter

Current bottleneck analysis:
```
Resource      Utilization  Saturation  Errors  Notes
───────────────────────────────────────────────────────
CPU           Low          None        None    Not CPU-bound
Memory        Low          None        None    Not memory-bound
Disk          Low          None        None    SQLite is fast
Network       N/A          N/A         N/A     Local only
Process spawn HIGH         N/A         None    ← THE BOTTLENECK
```

The bottleneck is **process spawn overhead**, not a traditional resource.
Solution: Eliminate process boundary with Rust library.

## References

- Brendan Gregg, "The USE Method" (2012)
- Brendan Gregg, "Systems Performance" (2020)
- Netflix Tech Blog, Performance Engineering articles
```

---

### FILE 3: references/02_FIVE_MINUTE_RULE.md

```markdown
# The Five-Minute Rule - Caching Economics

Source: Jim Gray, Microsoft Research (Turing Award Winner)

## The Original Rule (1987)

> "Pages referenced every five minutes should be memory resident."

**The math:**
- Cost of RAM per MB per month
- Cost of disk I/O per access
- Break-even: When RAM cost equals I/O cost savings

## Updated Rule (2024)

Hardware ratios have changed dramatically:

| Era | RAM $/GB | SSD $/GB | Break-even |
|-----|----------|----------|------------|
| 1987 | $5,000 | N/A (HDD) | 5 minutes |
| 2008 | $50 | N/A (HDD) | 1.5 hours |
| 2024 | $3 | $0.10 | ~15 seconds |

**Modern rule:** Cache if accessed more than once every 15-30 seconds.

## The Decision Framework

### Step 1: Calculate Access Frequency

```
Access frequency = Total accesses / Time period

Example:
- Chain graph queried 100 times per minute
- Frequency = 100/60 = 1.67 per second
- Much higher than 1/15s threshold → CACHE
```

### Step 2: Calculate Memory Cost

```
Memory cost = Data size × Duration × RAM $/GB

Example:
- Chain graph: 10MB
- Cache for 1 hour
- RAM at $3/GB/month = $0.0000004/MB/hour
- Cost = 10 × 1 × $0.0000004 = $0.000004
```

### Step 3: Calculate I/O Savings

```
I/O savings = Accesses avoided × I/O cost per access

Example:
- 100 accesses/min × 60 min = 6000 accesses
- SSD read: 150μs = 0.00015s of CPU time
- Value: 6000 × 0.00015 × (hourly rate) = significant
```

### Step 4: Compare and Decide

```
If I/O savings > Memory cost → CACHE
If I/O savings < Memory cost → DON'T CACHE
```

## Cache Tier Strategy

Based on access frequency and data characteristics:

### Hot Tier (Always in Memory)
```
Criteria:
- Access > 10x per second
- Size < 100MB
- Rarely changes

Examples:
- Bloom filters for chain membership
- Active chain list
- Configuration

Implementation:
- HashMap/BTreeMap in process memory
- No eviction (always resident)
```

### Warm Tier (LRU Cache)
```
Criteria:
- Access 1-10x per second
- Size 100MB - 1GB
- Changes occasionally

Examples:
- Query results
- Co-access lookups
- File tree subtrees

Implementation:
- LRU cache with size limit
- TTL for freshness
- Evict least-recently-used
```

### Cold Tier (Fetch on Demand)
```
Criteria:
- Access < 1x per minute
- Size > 1GB or unbounded
- May change frequently

Examples:
- Historical session data
- Full file content
- Archive queries

Implementation:
- Always fetch from SQLite
- Maybe short TTL cache (seconds)
```

## Cache Invalidation

> "There are only two hard things in Computer Science: cache invalidation and naming things." - Phil Karlton

### Strategies

**Time-based (TTL):**
```rust
struct CacheEntry<T> {
    value: T,
    expires_at: Instant,
}

// Good for: Data that changes on known schedule
// Bad for: Data that changes unpredictably
```

**Event-based:**
```rust
// On write to chain_graph table:
cache.invalidate("chain_graph");

// Good for: Data you control writes to
// Bad for: External data sources
```

**Version-based:**
```rust
struct VersionedCache<T> {
    data: HashMap<String, (T, u64)>, // (value, version)
    current_version: u64,
}

// Good for: Avoiding race conditions
// Bad for: Simple use cases (over-engineering)
```

## Application to Tastematter

| Data Structure | Access Pattern | Decision | Size |
|----------------|----------------|----------|------|
| Chain blooms | Every query | HOT | ~1KB/chain |
| Active chains | Most queries | HOT | ~10KB |
| Co-access matrix | Per file click | WARM LRU | ~1MB |
| File tree stats | Per nav | WARM LRU | ~5MB |
| Query results | Variable | WARM TTL | ~100KB/query |
| Session history | Rare | COLD | Unbounded |

**Memory budget:** ~50MB for hot+warm = instant queries

## References

- Jim Gray, "The Five-Minute Rule for Trading Memory for Disc Accesses" (1987)
- Jim Gray, "The Five-Minute Rule Ten Years Later" (1997)
- Goetz Graefe, "The Five-Minute Rule 20 Years Later" (2007)
```

---

### FILE 4: references/03_CONSISTENCY_MODELS.md

```markdown
# Consistency Models - Choosing the Right Tradeoffs

Source: Martin Kleppmann, "Designing Data-Intensive Applications"

## The Consistency Spectrum

```
Strong ◄────────────────────────────────────► Eventual

Linearizable → Sequential → Causal → Eventual
     │              │           │         │
     │              │           │         └─ Reads may be stale
     │              │           └─ Respects causality
     │              └─ Same order for all
     └─ Real-time ordering
```

## Consistency Models Explained

### Linearizability (Strongest)
```
Every operation appears to happen atomically at some point
between its invocation and response.

Guarantees:
- If write completes, all subsequent reads see it
- Operations have real-time ordering

Cost:
- Requires coordination (slow)
- Can't survive network partitions

Use when:
- Financial transactions
- Distributed locks
- Leader election
```

### Sequential Consistency
```
All operations appear in some sequential order,
and each process's operations appear in program order.

Guarantees:
- Global ordering exists
- Respects per-process order

Cost:
- Still requires coordination
- Better than linearizable for some workloads

Use when:
- Need ordering but not real-time
```

### Causal Consistency
```
Operations that are causally related are seen in same order
by all processes. Concurrent operations may be seen in different orders.

Guarantees:
- If A causes B, everyone sees A before B
- Concurrent operations can be reordered

Cost:
- Requires tracking causality
- Can be partition-tolerant

Use when:
- Social feeds (see reply after original)
- Collaborative editing
```

### Eventual Consistency (Weakest)
```
If no new updates, all replicas eventually converge.
No ordering guarantees during updates.

Guarantees:
- Eventually converges
- High availability

Cost:
- Temporary inconsistency
- Conflict resolution needed

Use when:
- High availability critical
- Conflicts are rare or resolvable
- DNS, caching, session stores
```

## Decision Framework

```
Do you need real-time ordering?
├─ YES → Linearizability (but expect latency)
└─ NO  → Continue

Do concurrent operations need same order everywhere?
├─ YES → Sequential consistency
└─ NO  → Continue

Do causally-related operations need ordering?
├─ YES → Causal consistency
└─ NO  → Eventual consistency (simplest, fastest)
```

## Application to Tastematter

### Index Reads (Most Queries)
```
Consistency needed: Eventual (or none)

Why:
- Index is append-mostly
- Stale reads are acceptable (user won't notice)
- Query performance is priority

Implementation:
- Read from SQLite without locks
- Background refresh acceptable
```

### Index Writes (Daemon Updates)
```
Consistency needed: Causal

Why:
- Chain graph must see parent before child
- Co-access depends on file_access
- Order matters within ingestion

Implementation:
- Single writer (daemon)
- Ordered inserts within transaction
```

### UI State (Svelte)
```
Consistency needed: Sequential (within session)

Why:
- User actions must appear in order
- Cross-tab consistency not critical

Implementation:
- Svelte stores handle this
- No special coordination needed
```

## CAP Theorem Reminder

```
You can only have 2 of 3:
- Consistency (all nodes see same data)
- Availability (every request gets response)
- Partition tolerance (system works despite network splits)

Since network partitions happen, real choice is:
- CP: Consistent but may be unavailable
- AP: Available but may be inconsistent
```

For Tastematter (single-machine):
- No network partitions
- Can have both C and A
- Choose based on simplicity

## References

- Martin Kleppmann, "Designing Data-Intensive Applications" (2017)
- Leslie Lamport, "How to Make a Multiprocessor Computer That Correctly Executes Multiprocess Programs" (1979)
- Werner Vogels, "Eventually Consistent" (2009)
```

---

### FILE 5: references/04_RUST_PERFORMANCE.md

```markdown
# Rust Performance Patterns

Patterns for high-performance Rust, especially for the Tastematter refactor.

## rusqlite Optimization

### Connection Pooling with r2d2

```rust
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;

pub fn create_pool(db_path: &str) -> Pool<SqliteConnectionManager> {
    let manager = SqliteConnectionManager::file(db_path);

    Pool::builder()
        .max_size(10)           // Tune based on workload
        .min_idle(Some(2))      // Keep 2 connections warm
        .connection_timeout(Duration::from_secs(5))
        .build(manager)
        .expect("Failed to create pool")
}
```

### WAL Mode for Concurrent Access

```rust
fn setup_connection(conn: &Connection) -> Result<()> {
    // Enable WAL mode (concurrent readers, one writer)
    conn.pragma_update(None, "journal_mode", "WAL")?;

    // Sync less often (faster, slight durability risk)
    conn.pragma_update(None, "synchronous", "NORMAL")?;

    // Larger cache (default 2MB, increase for read-heavy)
    conn.pragma_update(None, "cache_size", "-64000")?; // 64MB

    // Memory-map for faster reads
    conn.pragma_update(None, "mmap_size", "268435456")?; // 256MB

    Ok(())
}
```

### Prepared Statements

```rust
// BAD: Compiles query every time
fn bad_query(conn: &Connection, pattern: &str) -> Vec<String> {
    let sql = format!("SELECT path FROM files WHERE path LIKE '%{}%'", pattern);
    // SQL injection risk + recompilation overhead
}

// GOOD: Prepare once, execute many
fn good_query(conn: &Connection, pattern: &str) -> Result<Vec<String>> {
    let mut stmt = conn.prepare_cached(
        "SELECT path FROM files WHERE path LIKE ?1"
    )?;

    let pattern = format!("%{}%", pattern);
    let rows = stmt.query_map([&pattern], |row| row.get(0))?;

    rows.collect()
}
```

## Memory Management

### Avoid Allocations in Hot Paths

```rust
// BAD: Allocates on every call
fn bad_process(items: &[Item]) -> Vec<String> {
    items.iter()
        .map(|i| format!("{}: {}", i.name, i.value))  // Allocates String each time
        .collect()  // Allocates Vec
}

// GOOD: Reuse allocations
fn good_process(items: &[Item], buffer: &mut Vec<String>) {
    buffer.clear();
    buffer.reserve(items.len());

    for item in items {
        // Use write! to reuse String if possible
        buffer.push(format!("{}: {}", item.name, item.value));
    }
}

// BETTER: Return iterator, let caller decide
fn better_process(items: &[Item]) -> impl Iterator<Item = String> + '_ {
    items.iter().map(|i| format!("{}: {}", i.name, i.value))
}
```

### Use Cow for Flexible Ownership

```rust
use std::borrow::Cow;

fn process_path(path: &str) -> Cow<str> {
    if path.starts_with('/') {
        Cow::Borrowed(path)  // No allocation
    } else {
        Cow::Owned(format!("/{}", path))  // Allocate only when needed
    }
}
```

## Data Structures

### HashMap vs BTreeMap

```rust
// HashMap: O(1) average lookup, unordered
// Use for: Caches, lookups by exact key
use std::collections::HashMap;
let cache: HashMap<String, QueryResult> = HashMap::new();

// BTreeMap: O(log n) lookup, ordered
// Use for: Range queries, ordered iteration
use std::collections::BTreeMap;
let timeline: BTreeMap<DateTime, Event> = BTreeMap::new();
```

### SmallVec for Small Collections

```rust
use smallvec::SmallVec;

// Stack-allocated for small sizes, heap for large
type FileList = SmallVec<[String; 8]>;

fn get_files() -> FileList {
    let mut files = SmallVec::new();
    files.push("file1.rs".into());
    // If <= 8 items, no heap allocation
    files
}
```

## Async Patterns

### Tauri Commands

```rust
// Tauri commands are async by default
#[command]
pub async fn query_flex(
    time: Option<String>,
    state: State<'_, AppState>,
) -> Result<QueryResult, CommandError> {
    // Access shared state
    let index = state.index.lock().await;

    // Perform query (blocking DB work should use spawn_blocking)
    let result = tokio::task::spawn_blocking(move || {
        index.query(time)
    }).await??;

    Ok(result)
}
```

### spawn_blocking for CPU-bound Work

```rust
// DON'T block the async runtime
async fn bad_compute(data: &[u8]) -> Hash {
    compute_hash(data)  // Blocks async thread
}

// DO use spawn_blocking
async fn good_compute(data: Vec<u8>) -> Result<Hash> {
    tokio::task::spawn_blocking(move || {
        compute_hash(&data)
    }).await?
}
```

## Bloom Filter Implementation

```rust
use bitvec::prelude::*;

pub struct BloomFilter {
    bits: BitVec,
    hash_count: usize,
}

impl BloomFilter {
    pub fn new(expected_items: usize, false_positive_rate: f64) -> Self {
        let size = optimal_size(expected_items, false_positive_rate);
        let hash_count = optimal_hash_count(size, expected_items);

        Self {
            bits: bitvec![0; size],
            hash_count,
        }
    }

    pub fn insert(&mut self, item: &str) {
        for i in 0..self.hash_count {
            let idx = self.hash(item, i);
            self.bits.set(idx, true);
        }
    }

    pub fn contains(&self, item: &str) -> bool {
        (0..self.hash_count).all(|i| {
            let idx = self.hash(item, i);
            self.bits[idx]
        })
    }

    fn hash(&self, item: &str, seed: usize) -> usize {
        // Use xxhash or similar for speed
        let h = xxhash_rust::xxh3::xxh3_64_with_seed(item.as_bytes(), seed as u64);
        (h as usize) % self.bits.len()
    }
}
```

## LRU Cache

```rust
use lru::LruCache;
use std::num::NonZeroUsize;

pub struct QueryCache {
    cache: LruCache<QuerySpec, QueryResult>,
}

impl QueryCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: LruCache::new(NonZeroUsize::new(capacity).unwrap()),
        }
    }

    pub fn get(&mut self, key: &QuerySpec) -> Option<&QueryResult> {
        self.cache.get(key)
    }

    pub fn put(&mut self, key: QuerySpec, value: QueryResult) {
        self.cache.put(key, value);
    }
}
```

## Profiling

### Using flamegraph

```bash
# Install
cargo install flamegraph

# Run with profiling
cargo flamegraph --bin tastematter

# On Windows, may need:
cargo flamegraph --bin tastematter -- --profile
```

### Using criterion for benchmarks

```rust
// benches/query_bench.rs
use criterion::{criterion_group, criterion_main, Criterion};

fn benchmark_query(c: &mut Criterion) {
    let index = setup_test_index();

    c.bench_function("query_flex_7d", |b| {
        b.iter(|| index.query_flex(Some("7d".into()), None, None))
    });
}

criterion_group!(benches, benchmark_query);
criterion_main!(benches);
```

## References

- Rust Performance Book: https://nnethercote.github.io/perf-book/
- rusqlite docs: https://docs.rs/rusqlite/
- Tauri docs: https://tauri.app/
```

---

### FILE 6: references/05_DATABASE_PATTERNS.md

```markdown
# Database Patterns for SQLite

Patterns for high-performance SQLite usage in the Context OS stack.

## Schema Design

### Existing Schema (from 02_INDEX_STRUCTURES.md)

```sql
-- Chain Graph
CREATE TABLE chains (
    chain_id TEXT PRIMARY KEY,
    root_session_id TEXT NOT NULL,
    session_count INTEGER,
    started_at TEXT,
    ended_at TEXT,
    files_bloom BLOB,
    files_json TEXT
);

CREATE TABLE chain_graph (
    session_id TEXT PRIMARY KEY,
    parent_session_id TEXT,
    chain_id TEXT NOT NULL,
    position_in_chain INTEGER
);

-- File Access
CREATE TABLE file_tree (
    path TEXT PRIMARY KEY,
    is_directory BOOLEAN,
    parent_path TEXT,
    chains_json TEXT,
    sessions_json TEXT,
    session_count INTEGER,
    last_accessed TEXT,
    depth INTEGER
);

CREATE TABLE file_access (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_path TEXT NOT NULL,
    session_id TEXT NOT NULL,
    chain_id TEXT,
    access_type TEXT NOT NULL,
    tool_name TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    UNIQUE(file_path, session_id, timestamp)
);

-- Co-access Matrix
CREATE TABLE co_access (
    file_a TEXT NOT NULL,
    file_b TEXT NOT NULL,
    jaccard_score REAL NOT NULL,
    co_occurrence_count INTEGER,
    PRIMARY KEY (file_a, file_b)
);

-- Temporal Buckets
CREATE TABLE temporal_buckets (
    period TEXT PRIMARY KEY,
    period_type TEXT NOT NULL,
    sessions_json TEXT,
    chains_json TEXT,
    files_bloom BLOB,
    session_count INTEGER,
    started_at TEXT,
    ended_at TEXT
);
```

## Index Design

### Principles

1. **Index columns used in WHERE clauses**
2. **Index columns used in JOIN conditions**
3. **Index columns used in ORDER BY**
4. **Avoid over-indexing (write overhead)**

### Key Indexes

```sql
-- For chain lookups
CREATE INDEX idx_chain_graph_chain ON chain_graph(chain_id);
CREATE INDEX idx_chain_graph_parent ON chain_graph(parent_session_id);

-- For file lookups
CREATE INDEX idx_file_tree_parent ON file_tree(parent_path);
CREATE INDEX idx_file_tree_last_accessed ON file_tree(last_accessed);

-- For file access queries
CREATE INDEX idx_file_access_path ON file_access(file_path);
CREATE INDEX idx_file_access_session ON file_access(session_id);
CREATE INDEX idx_file_access_timestamp ON file_access(timestamp);

-- For co-access lookups
CREATE INDEX idx_co_access_file_a ON co_access(file_a);
CREATE INDEX idx_co_access_score ON co_access(jaccard_score);
```

## Query Optimization

### EXPLAIN QUERY PLAN

```sql
-- Always check query plans for slow queries
EXPLAIN QUERY PLAN
SELECT * FROM file_access WHERE file_path LIKE '%parser%';

-- Look for:
-- SCAN (bad - full table scan)
-- SEARCH (good - using index)
```

### Common Query Patterns

**Get files for chain (fast):**
```sql
SELECT f.*
FROM file_access f
JOIN chain_graph cg ON f.session_id = cg.session_id
WHERE cg.chain_id = ?
ORDER BY f.timestamp DESC;
```

**Get co-accessed files (fast):**
```sql
SELECT file_b, jaccard_score
FROM co_access
WHERE file_a = ?
ORDER BY jaccard_score DESC
LIMIT 10;
```

**Search files by pattern (slower, needs optimization):**
```sql
-- Instead of LIKE '%pattern%' (can't use index):
SELECT path FROM file_tree
WHERE path LIKE ? || '%'  -- Prefix match CAN use index

-- Or use FTS5 for full-text search:
CREATE VIRTUAL TABLE file_paths_fts USING fts5(path);
SELECT path FROM file_paths_fts WHERE path MATCH ?;
```

## Transaction Patterns

### Batch Inserts

```rust
// BAD: One transaction per insert
for item in items {
    conn.execute("INSERT INTO table VALUES (?)", [item])?;
}

// GOOD: Batch in single transaction
let tx = conn.transaction()?;
{
    let mut stmt = tx.prepare("INSERT INTO table VALUES (?)")?;
    for item in items {
        stmt.execute([item])?;
    }
}
tx.commit()?;
```

### Read-Heavy Optimization

```rust
// For read-heavy workloads, use WAL mode
conn.pragma_update(None, "journal_mode", "WAL")?;

// Allows concurrent reads while writing
// Writers don't block readers
// Readers don't block writers
```

## JSON Columns

SQLite has native JSON support:

```sql
-- Store JSON
INSERT INTO file_tree (path, chains_json)
VALUES ('src/', '["chain_001", "chain_002"]');

-- Query JSON arrays
SELECT path FROM file_tree
WHERE json_array_length(chains_json) > 0;

-- Extract from JSON
SELECT path, json_extract(chains_json, '$[0]') as first_chain
FROM file_tree;

-- Check if value in JSON array
SELECT path FROM file_tree
WHERE chains_json LIKE '%"chain_001"%';
-- Or with json_each:
SELECT DISTINCT ft.path
FROM file_tree ft, json_each(ft.chains_json) je
WHERE je.value = 'chain_001';
```

## BLOB Storage (Bloom Filters)

```rust
// Serialize bloom filter to blob
impl BloomFilter {
    pub fn to_blob(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }

    pub fn from_blob(data: &[u8]) -> Self {
        bincode::deserialize(data).unwrap()
    }
}

// Store in SQLite
conn.execute(
    "UPDATE chains SET files_bloom = ? WHERE chain_id = ?",
    params![bloom.to_blob(), chain_id],
)?;

// Retrieve from SQLite
let bloom: BloomFilter = conn.query_row(
    "SELECT files_bloom FROM chains WHERE chain_id = ?",
    [chain_id],
    |row| {
        let blob: Vec<u8> = row.get(0)?;
        Ok(BloomFilter::from_blob(&blob))
    },
)?;
```

## Performance Tuning

### Pragmas for Speed

```sql
-- WAL mode (required for concurrent access)
PRAGMA journal_mode = WAL;

-- Synchronous mode (trade durability for speed)
PRAGMA synchronous = NORMAL;  -- or OFF for max speed

-- Cache size (negative = KB, positive = pages)
PRAGMA cache_size = -64000;  -- 64MB cache

-- Memory-mapped I/O
PRAGMA mmap_size = 268435456;  -- 256MB mmap

-- Temp store in memory
PRAGMA temp_store = MEMORY;
```

### Vacuum and Analyze

```sql
-- Rebuild database file (reclaim space, defragment)
VACUUM;

-- Update query planner statistics
ANALYZE;
```

## Backup Strategy

```rust
// Online backup while database is in use
fn backup_database(src: &Connection, dst_path: &str) -> Result<()> {
    let mut dst = Connection::open(dst_path)?;
    let backup = rusqlite::backup::Backup::new(src, &mut dst)?;
    backup.run_to_completion(5, Duration::from_millis(250), None)?;
    Ok(())
}
```

## References

- SQLite Documentation: https://sqlite.org/docs.html
- rusqlite crate: https://docs.rs/rusqlite/
- SQLite Performance: https://sqlite.org/fasterthanfs.html
```

---

## Current State Summary

### Completed
- [x] Full architecture analysis (Python CLI vs Rust)
- [x] Bottleneck identified (process spawn = 5000ms)
- [x] Solution designed (Rust core library)
- [x] `technical-architecture-engineering` SKILL.md written
- [x] Reference content drafted (above)

### Remaining (For Next Agent)
- [ ] Create 6 reference files from content above
- [ ] Verify skill loads correctly
- [ ] Begin Phase 0 implementation planning

---

## For Next Agent

### Immediate Task

**Chop up the reference content above into 6 separate files:**

1. Copy content from "FILE 1" section → `references/00_LATENCY_NUMBERS.md`
2. Copy content from "FILE 2" section → `references/01_USE_METHOD.md`
3. Copy content from "FILE 3" section → `references/02_FIVE_MINUTE_RULE.md`
4. Copy content from "FILE 4" section → `references/03_CONSISTENCY_MODELS.md`
5. Copy content from "FILE 5" section → `references/04_RUST_PERFORMANCE.md`
6. Copy content from "FILE 6" section → `references/05_DATABASE_PATTERNS.md`

**Location:** `.claude/skills/technical-architecture-engineering/references/`

### After Skill Complete

Proceed with Phase 0 planning:
1. Read SKILL.md and apply patterns
2. Design `context-os-core` Rust library crate
3. Port Python query engine to Rust
4. Integrate into Tastematter

### Context Chain

- Previous: [[06_2026-01-07_CANONICAL_ENRICHMENT]]
- This: Architecture skill creation + reference content
- Next: Finish skill → Phase 0 implementation

---

**Package written:** 2026-01-07
**Session focus:** Architecture analysis + skill creation
**Key deliverable:** technical-architecture-engineering skill (SKILL.md complete, references pending)
