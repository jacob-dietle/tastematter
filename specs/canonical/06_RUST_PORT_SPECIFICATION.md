---
title: "Rust Port Specification - Python Indexer/Daemon"
type: implementation-spec
created: 2026-01-13
status: approved
foundation:
  - "[[canonical/03_CORE_ARCHITECTURE]]"
  - "[[canonical/05_INTELLIGENCE_LAYER_ARCHITECTURE]]"
  - "[[cli/src/context_os_events/capture/git_sync.py]]"
  - "[[cli/src/context_os_events/capture/jsonl_parser.py]]"
  - "[[cli/src/context_os_events/index/chain_graph.py]]"
related:
  - "[[context_packages/04_daemon/06_CORE_INFRASTRUCTURE_AUDIT]]"
tags:
  - tastematter
  - rust-port
  - daemon
  - implementation
---

# Rust Port Specification: Python Indexer/Daemon

## Executive Summary

This specification defines the complete port of the Python indexer/daemon to Rust, creating a single-binary distribution. The port transforms the current hybrid architecture (Python writes, Rust reads) into a unified Rust system.

**Current State:**
- Rust core: ~1,700 LOC (25%) - READ path (query, serve)
- Python CLI: ~6,500 LOC (75%) - WRITE path (daemon, sync, parse, chains)

**Target State:**
- Rust core: ~4,500 LOC (100%) - All operations
- Python CLI: Reference only (kept functional during transition)

**Estimated Effort:** 44-62 hours across 6 phases

---

## Problem Statement

### Measured Bottleneck

```
Python subprocess spawn: ~4,800ms
Rust direct function call: <1μs
Overhead: 4,800,000x
```

[VERIFIED: Timing measurement from 03_CORE_ARCHITECTURE.md]

### Why Port is Required

1. **Distribution complexity:** Users must install Python + pip + dependencies
2. **Startup latency:** 4.8s cold start vs <100ms target
3. **Dual maintenance:** Two languages, two test suites, two build systems
4. **Context switches:** Developer productivity loss from language switching

### Why Existing Code Cannot Handle This

The problem is architectural, not fixable with <200 lines:
- Cannot eliminate Python subprocess spawn without removing Python
- Cannot reduce Python import time (fundamental to language)
- Cannot share memory between Python process and Rust core

[VERIFIED: Staff Engineer Decision Framework - Phase 1 validation passed]

---

## Success Metrics

| Metric | Current (Python) | Target (Rust) | Validation Method |
|--------|------------------|---------------|-------------------|
| Cold start | ~4,800ms | <100ms | Time `tastematter daemon run --once` |
| Parse throughput | ~50 sessions/sec | ~500 sessions/sec | Benchmark 1000 JSONL files |
| Memory (large corpus) | ~200MB | <50MB | `tastematter daemon` memory profile |
| Distribution | Python + pip | Single binary | `cargo build --release` |
| Maintenance | Two languages | One language | Code inspection |

---

## Architecture

### Component Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                      RUST CORE (Single Binary)                          │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌───────────────────────────────────────────────────────────────────┐  │
│  │                        CLI Layer (clap)                            │  │
│  │  query  serve  sync-git  parse-sessions  build-chains  daemon     │  │
│  └───────────────────────────────────────────────────────────────────┘  │
│                                   │                                      │
│          ┌────────────────────────┼────────────────────────┐            │
│          ▼                        ▼                        ▼            │
│  ┌──────────────┐        ┌──────────────┐        ┌──────────────┐       │
│  │   capture/   │        │    index/    │        │    query/    │       │
│  │              │        │              │        │  (existing)  │       │
│  │  git_sync    │        │ chain_graph  │        │              │       │
│  │  jsonl_parse │        │              │        │  query_flex  │       │
│  │  file_watch  │        │              │        │  query_chain │       │
│  └──────┬───────┘        └──────┬───────┘        └──────┬───────┘       │
│         │                       │                       │               │
│         └───────────────────────┼───────────────────────┘               │
│                                 ▼                                        │
│  ┌───────────────────────────────────────────────────────────────────┐  │
│  │                     storage.rs (rusqlite)                         │  │
│  │                   Connection Pool (r2d2)                          │  │
│  │                                                                    │  │
│  │  READ (existing)              WRITE (new)                         │  │
│  │  - query_flex()               - insert_commits()                  │  │
│  │  - query_chains()             - insert_sessions()                 │  │
│  │  - query_timeline()           - insert_tool_uses()                │  │
│  │                               - persist_chains()                  │  │
│  └───────────────────────────────────────────────────────────────────┘  │
│                                 │                                        │
│                                 ▼                                        │
│         ┌──────────────────────────────────────────────────┐            │
│         │              SQLite Database                      │            │
│         │  ~/.context-os/context_os_events.db              │            │
│         └──────────────────────────────────────────────────┘            │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### Module Structure

```
core/src/
├── main.rs              # CLI entry point (extend with new commands)
├── lib.rs               # Library exports
├── storage.rs           # Database layer (extend with writes)
├── types.rs             # Data types (extend with new types)
├── query.rs             # Query engine (existing, no changes)
├── http.rs              # HTTP server (existing, no changes)
├── capture/             # NEW: Capture layer
│   ├── mod.rs
│   ├── git_sync.rs      # Port from git_sync.py
│   ├── jsonl_parser.rs  # Port from jsonl_parser.py
│   └── file_watcher.rs  # Port from file_watcher.py
├── index/               # NEW: Index layer
│   ├── mod.rs
│   └── chain_graph.rs   # Port from chain_graph.py
└── daemon/              # NEW: Daemon layer
    ├── mod.rs
    ├── runner.rs        # Port from runner.py
    └── scheduler.rs     # Tokio-based scheduler
```

---

## Phase 1: Storage Foundation

**Estimated Time:** 4-6 hours
**Dependencies:** None
**Python Reference:** `cli/src/context_os_events/db/connection.py`

### Objective

Add write capabilities to existing `storage.rs` using rusqlite for direct SQLite access.

### Type Contracts

```rust
// storage.rs additions

/// Result of a database write operation
pub struct WriteResult {
    pub rows_affected: u64,
    pub last_insert_id: Option<i64>,
}

/// Database connection with read AND write capabilities
pub struct Database {
    pool: r2d2::Pool<SqliteConnectionManager>,
}

impl Database {
    /// Get a read-only connection (existing behavior)
    pub fn read(&self) -> Result<PooledConnection> { ... }

    /// Get a read-write connection (new)
    pub fn write(&self) -> Result<PooledConnection> { ... }

    /// Execute INSERT with prepared statement (new)
    pub fn insert_commits(&self, commits: &[GitCommit]) -> Result<WriteResult> { ... }

    /// Execute INSERT with prepared statement (new)
    pub fn insert_sessions(&self, sessions: &[ParsedSession]) -> Result<WriteResult> { ... }

    /// Execute INSERT with prepared statement (new)
    pub fn insert_tool_uses(&self, tool_uses: &[ToolUse]) -> Result<WriteResult> { ... }

    /// Execute batch operations in transaction (new)
    pub fn transaction<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&Transaction) -> Result<T> { ... }
}
```

### Implementation Steps

1. **Add dependencies to Cargo.toml:**
   ```toml
   [dependencies]
   r2d2 = "0.8"
   r2d2_sqlite = "0.24"
   ```

2. **Extend Database struct:**
   - Add connection pooling via r2d2
   - Add `write()` method for read-write connections
   - Pool size: 4 connections (based on formula: 100 req/sec × 0.05s × 2 = 10, reduced for local use)

3. **Implement batch insert methods:**
   - Use prepared statements for performance
   - Wrap in transactions for atomicity
   - Return row counts for validation

4. **Add schema creation:**
   - Port from Python `schema.py`
   - CREATE TABLE IF NOT EXISTS for all tables
   - CREATE INDEX IF NOT EXISTS for all indexes

### Test Criteria

```bash
# Unit test: Can insert a commit
cargo test test_insert_commit

# Integration test: Batch insert 1000 commits
cargo test test_batch_insert_commits --release

# Performance: <50ms for batch of 1000
time cargo test test_batch_insert_commits --release
```

### Success Criteria

- [ ] Connection pool initialized with r2d2
- [ ] `write()` returns read-write connection
- [ ] `insert_commits()` inserts to git_commits table
- [ ] `insert_sessions()` inserts to claude_sessions table
- [ ] `insert_tool_uses()` inserts to tool_uses table
- [ ] Transaction support works
- [ ] Batch of 1000 inserts completes in <50ms

---

## Phase 2: Git Sync

**Estimated Time:** 8-12 hours
**Dependencies:** Phase 1 (Storage Foundation)
**Python Reference:** `cli/src/context_os_events/capture/git_sync.py` (484 lines)

### Objective

Port git commit syncing from Python subprocess calls to native Rust using git2 crate.

### Type Contracts

```rust
// capture/git_sync.rs

/// Options for sync operation
pub struct SyncOptions {
    /// How far back to sync (e.g., "30 days")
    pub since: String,
    /// Only sync new commits since last run
    pub incremental: bool,
    /// Repository path (defaults to cwd)
    pub repo_path: Option<PathBuf>,
}

/// Result of sync operation
pub struct SyncResult {
    pub commits_synced: u32,
    pub agent_commits: u32,
    pub skipped_existing: u32,
    pub errors: Vec<String>,
}

/// A single git commit with file changes
#[derive(Debug, Clone)]
pub struct GitCommit {
    pub hash: String,
    pub short_hash: String,
    pub timestamp: DateTime<Utc>,
    pub author_name: String,
    pub author_email: String,
    pub subject: String,
    pub parent_hashes: Vec<String>,
    pub is_agent_commit: bool,
    pub files_changed: Vec<FileChange>,
}

/// File change within a commit
#[derive(Debug, Clone)]
pub struct FileChange {
    pub path: String,
    pub status: FileStatus,
    pub additions: u32,
    pub deletions: u32,
}

#[derive(Debug, Clone)]
pub enum FileStatus {
    Added,
    Modified,
    Deleted,
    Renamed { from: String },
    Copied { from: String },
}

/// Sync git commits to database
pub fn sync_commits(db: &Database, options: SyncOptions) -> Result<SyncResult> { ... }

/// Check if commit is likely from Claude agent
fn is_agent_commit(commit: &git2::Commit) -> bool { ... }
```

### Algorithm Port

**Current Python approach:**
```python
# Subprocess with custom format
output = subprocess.run(
    ["git", "log", f"--since={since}",
     "--format=%H§%h§%aI§%an§%ae§%s§%P",
     "--numstat", "--name-status"],
    capture_output=True
)
# Parse delimited output
```

**Rust approach (git2 native):**
```rust
// Native git operations
let repo = Repository::open(path)?;
let mut revwalk = repo.revwalk()?;
revwalk.push_head()?;

for oid in revwalk {
    let commit = repo.find_commit(oid?)?;
    // Direct access to commit data
    let author = commit.author();
    let message = commit.message();
    // ...
}
```

### Agent Commit Detection

Port the signature patterns from Python:
```rust
fn is_agent_commit(commit: &git2::Commit) -> bool {
    let message = commit.message().unwrap_or("").to_lowercase();
    let author = commit.author();
    let email = author.email().unwrap_or("");

    // Pattern 1: Known agent signatures in message
    let agent_patterns = [
        "generated with claude code",
        "🤖 generated with",
        "co-authored-by: claude",
    ];

    // Pattern 2: Known agent email patterns
    let agent_emails = [
        "noreply@anthropic.com",
        "claude@anthropic.com",
    ];

    agent_patterns.iter().any(|p| message.contains(p)) ||
    agent_emails.iter().any(|e| email.contains(e))
}
```

### Implementation Steps

1. **Add git2 dependency:**
   ```toml
   [dependencies]
   git2 = "0.18"
   ```

2. **Create capture/git_sync.rs module:**
   - Define types (GitCommit, FileChange, etc.)
   - Implement `sync_commits()` using git2
   - Implement `is_agent_commit()` detection
   - Implement incremental sync via last hash tracking

3. **Add CLI command:**
   ```rust
   // main.rs
   #[derive(Subcommand)]
   enum Commands {
       // ... existing commands ...

       /// Sync git commits to database
       SyncGit {
           /// How far back to sync (e.g., "30d", "7d")
           #[arg(long, default_value = "30d")]
           since: String,

           /// Repository path (defaults to current directory)
           #[arg(long)]
           repo: Option<PathBuf>,
       },
   }
   ```

4. **Wire to storage layer:**
   - Call `db.insert_commits()` with parsed commits
   - Track last synced hash for incremental sync

### Test Criteria

```bash
# Unit test: Parse commit data
cargo test test_parse_commit

# Unit test: Agent commit detection
cargo test test_agent_commit_detection

# Integration test: Sync real repo
cargo test test_sync_real_repo --ignored

# CLI test
./target/release/context-os sync-git --since 7d
```

### Success Criteria

- [ ] git2 successfully opens repository
- [ ] Commits parsed with all fields (hash, author, timestamp, etc.)
- [ ] File changes extracted (additions, deletions, status)
- [ ] Agent commits detected correctly (>95% accuracy)
- [ ] Incremental sync only processes new commits
- [ ] `sync-git` CLI command works
- [ ] Performance: 1000 commits synced in <5s

---

## Phase 3: JSONL Parser

**Estimated Time:** 12-16 hours
**Dependencies:** Phase 1 (Storage Foundation)
**Python Reference:** `cli/src/context_os_events/capture/jsonl_parser.py` (580 lines)

### Objective

Port Claude session JSONL parsing from Python to Rust using serde_json with streaming support.

### Type Contracts

```rust
// capture/jsonl_parser.rs

/// Options for parsing sessions
pub struct ParseOptions {
    /// Project path to find Claude sessions
    pub project_path: PathBuf,
    /// Only parse new/changed files
    pub incremental: bool,
}

/// Result of parse operation
pub struct ParseResult {
    pub sessions_parsed: u32,
    pub tool_uses_extracted: u32,
    pub skipped_unchanged: u32,
    pub errors: Vec<String>,
}

/// A parsed Claude session from JSONL
#[derive(Debug, Clone)]
pub struct ParsedSession {
    pub session_id: String,
    pub project_path: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub message_count: u32,
    pub user_messages: u32,
    pub assistant_messages: u32,
    pub tool_uses: Vec<ToolUse>,
    pub leaf_uuid: Option<String>,           // For chain linking
    pub parent_session_id: Option<String>,   // For agent sessions
    pub file_size: u64,                      // For incremental tracking
}

/// Tool usage extracted from session
#[derive(Debug, Clone)]
pub struct ToolUse {
    pub tool_name: String,
    pub file_path: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub session_id: String,
}

/// Parse all sessions for a project
pub fn parse_sessions(db: &Database, options: ParseOptions) -> Result<ParseResult> { ... }

/// Parse a single JSONL file
fn parse_jsonl_file(path: &Path) -> Result<ParsedSession> { ... }

/// Extract tool uses from message content
fn extract_tool_uses(content: &Value, session_id: &str, timestamp: DateTime<Utc>) -> Vec<ToolUse> { ... }
```

### Algorithm Port

**Key parsing logic from Python:**

1. **Encode/decode project path:**
   ```rust
   /// Encode project path to Claude's format (Windows: C:\path → C--Users-...)
   fn encode_project_path(path: &Path) -> String {
       let path_str = path.to_string_lossy();
       path_str
           .replace(":", "-")
           .replace("/", "-")
           .replace("\\", "-")
   }
   ```

2. **Find Claude session directory:**
   ```rust
   fn find_claude_sessions_dir(project_path: &Path) -> Option<PathBuf> {
       let encoded = encode_project_path(project_path);
       let claude_dir = dirs::home_dir()?.join(".claude").join("projects").join(encoded);
       if claude_dir.exists() { Some(claude_dir) } else { None }
   }
   ```

3. **Extract leafUuid for chain linking:**
   ```rust
   fn extract_leaf_uuid(first_record: &Value) -> Option<String> {
       // Only first record's leafUuid matters for chain linking
       // Compaction summaries also have leafUuid but point to same session
       first_record
           .get("parentMessageUuid")  // Wait, this might be different
           .or_else(|| first_record.get("leafUuid"))
           .and_then(|v| v.as_str())
           .map(|s| s.to_string())
   }
   ```

4. **Extract sessionId for agent sessions:**
   ```rust
   fn extract_parent_session_id(first_record: &Value) -> Option<String> {
       // Agent sessions have sessionId pointing to parent's filename
       first_record
           .get("sessionId")
           .and_then(|v| v.as_str())
           .map(|s| s.to_string())
   }
   ```

### Streaming Parser Design

For large JSONL files, use streaming to minimize memory:

```rust
use std::io::{BufRead, BufReader};
use serde_json::Value;

fn parse_jsonl_streaming(path: &Path) -> Result<ParsedSession> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let mut first_record: Option<Value> = None;
    let mut last_record: Option<Value> = None;
    let mut message_count = 0;
    let mut tool_uses = Vec::new();

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() { continue; }

        let record: Value = serde_json::from_str(&line)?;

        if first_record.is_none() {
            first_record = Some(record.clone());
        }

        // Extract tool uses from this record
        if let Some(content) = record.get("content") {
            let uses = extract_tool_uses(content, &session_id, timestamp);
            tool_uses.extend(uses);
        }

        message_count += 1;
        last_record = Some(record);
    }

    // Build session from first/last records
    Ok(ParsedSession {
        session_id: extract_session_id(&first_record),
        leaf_uuid: extract_leaf_uuid(&first_record),
        parent_session_id: extract_parent_session_id(&first_record),
        started_at: extract_timestamp(&first_record),
        ended_at: extract_timestamp(&last_record),
        message_count,
        tool_uses,
        // ...
    })
}
```

### Implementation Steps

1. **Create capture/jsonl_parser.rs module:**
   - Define types (ParsedSession, ToolUse, etc.)
   - Implement streaming JSONL parser
   - Implement tool use extraction

2. **Add CLI command:**
   ```rust
   /// Parse Claude sessions
   ParseSessions {
       /// Project path (defaults to current directory)
       #[arg(long)]
       project: Option<PathBuf>,

       /// Only parse new/changed files
       #[arg(long)]
       incremental: bool,
   },
   ```

3. **Wire to storage layer:**
   - Call `db.insert_sessions()` with parsed sessions
   - Call `db.insert_tool_uses()` with extracted tool uses
   - Track file sizes for incremental parsing

### Test Criteria

```bash
# Unit test: Parse single JSONL
cargo test test_parse_single_jsonl

# Unit test: Extract tool uses
cargo test test_extract_tool_uses

# Unit test: Project path encoding
cargo test test_encode_project_path

# Integration test: Parse real sessions
cargo test test_parse_real_sessions --ignored

# CLI test
./target/release/context-os parse-sessions --project .
```

### Success Criteria

- [ ] Streaming parser processes files without loading all into memory
- [ ] leafUuid extracted from first record only
- [ ] sessionId extracted for agent session linking
- [ ] Tool uses extracted with file paths
- [ ] Incremental parsing skips unchanged files
- [ ] `parse-sessions` CLI command works
- [ ] Performance: 1000 sessions parsed in <10s

---

## Phase 4: Chain Graph

**Estimated Time:** 8-12 hours
**Dependencies:** Phase 3 (JSONL Parser)
**Python Reference:** `cli/src/context_os_events/index/chain_graph.py` (609 lines)

### Objective

Port the 5-pass chain building algorithm from Python to Rust.

### Type Contracts

```rust
// index/chain_graph.rs

/// A node in the chain graph (represents one session)
#[derive(Debug, Clone)]
pub struct ChainNode {
    pub session_id: String,
    pub parent_id: Option<String>,
    pub children: Vec<String>,
    pub chain_id: Option<String>,
    pub is_agent_session: bool,
    pub depth: u32,
}

/// A chain of related sessions
#[derive(Debug, Clone)]
pub struct Chain {
    pub id: String,
    pub root_session_id: String,
    pub session_ids: Vec<String>,
    pub session_count: u32,
    pub agent_count: u32,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
}

/// Result of chain building
pub struct ChainBuildResult {
    pub chains_built: u32,
    pub sessions_linked: u32,
    pub orphan_sessions: u32,
}

/// Build chains from session data
pub fn build_chains(db: &Database) -> Result<ChainBuildResult> { ... }

/// Persist chains to database
pub fn persist_chains(db: &Database, chains: &[Chain]) -> Result<u32> { ... }
```

### 5-Pass Algorithm Port

**Critical insight from Package 02:** Only the FIRST record's leafUuid indicates session resumption. Compaction summaries also have leafUuid but point to the same session.

```rust
pub fn build_chains(db: &Database) -> Result<ChainBuildResult> {
    // Load all sessions
    let sessions = db.load_sessions()?;

    // Pass 1: Extract leafUuid from regular sessions
    // Key: session with leafUuid, Value: target session (parent)
    let mut leaf_uuid_links: HashMap<String, String> = HashMap::new();
    for session in &sessions {
        if let Some(leaf_uuid) = &session.leaf_uuid {
            if !session.is_agent_session {
                // Find session that owns this UUID
                if let Some(parent) = find_session_by_uuid(&sessions, leaf_uuid) {
                    leaf_uuid_links.insert(session.session_id.clone(), parent.session_id.clone());
                }
            }
        }
    }

    // Pass 2: Extract sessionId from agent sessions
    // Agent sessions point to parent via sessionId (filename of parent)
    let mut session_id_links: HashMap<String, String> = HashMap::new();
    for session in &sessions {
        if let Some(parent_id) = &session.parent_session_id {
            if session.is_agent_session {
                session_id_links.insert(session.session_id.clone(), parent_id.clone());
            }
        }
    }

    // Pass 3: Build unified parent-child graph
    let mut graph: HashMap<String, ChainNode> = HashMap::new();
    for session in &sessions {
        graph.insert(session.session_id.clone(), ChainNode {
            session_id: session.session_id.clone(),
            parent_id: leaf_uuid_links.get(&session.session_id)
                .or_else(|| session_id_links.get(&session.session_id))
                .cloned(),
            children: Vec::new(),
            chain_id: None,
            is_agent_session: session.is_agent_session,
            depth: 0,
        });
    }

    // Pass 4: Populate children from parent links
    let parent_map: HashMap<String, String> = graph.iter()
        .filter_map(|(id, node)| node.parent_id.as_ref().map(|p| (id.clone(), p.clone())))
        .collect();

    for (child_id, parent_id) in &parent_map {
        if let Some(parent_node) = graph.get_mut(parent_id) {
            parent_node.children.push(child_id.clone());
        }
    }

    // Pass 5: Group into chains via connected components
    let chains = find_connected_components(&graph);

    // Persist to database
    persist_chains(db, &chains)?;

    Ok(ChainBuildResult {
        chains_built: chains.len() as u32,
        sessions_linked: parent_map.len() as u32,
        orphan_sessions: sessions.len() as u32 - parent_map.len() as u32,
    })
}

fn find_connected_components(graph: &HashMap<String, ChainNode>) -> Vec<Chain> {
    let mut visited: HashSet<String> = HashSet::new();
    let mut chains: Vec<Chain> = Vec::new();

    for session_id in graph.keys() {
        if visited.contains(session_id) { continue; }

        // BFS to find all connected sessions
        let mut component: Vec<String> = Vec::new();
        let mut queue: VecDeque<String> = VecDeque::new();
        queue.push_back(session_id.clone());

        while let Some(current) = queue.pop_front() {
            if visited.contains(&current) { continue; }
            visited.insert(current.clone());
            component.push(current.clone());

            if let Some(node) = graph.get(&current) {
                // Add parent if exists
                if let Some(parent) = &node.parent_id {
                    if !visited.contains(parent) {
                        queue.push_back(parent.clone());
                    }
                }
                // Add children
                for child in &node.children {
                    if !visited.contains(child) {
                        queue.push_back(child.clone());
                    }
                }
            }
        }

        // Create chain from component
        let chain = create_chain_from_component(&component, graph);
        chains.push(chain);
    }

    chains
}
```

### Implementation Steps

1. **Create index/chain_graph.rs module:**
   - Define types (ChainNode, Chain, etc.)
   - Implement 5-pass algorithm
   - Implement connected component finder

2. **Add CLI command:**
   ```rust
   /// Build chain graph from sessions
   BuildChains,
   ```

3. **Wire to storage layer:**
   - Load sessions from database
   - Build chain graph
   - Persist chains to database

### Test Criteria

```bash
# Unit test: leafUuid linking
cargo test test_leaf_uuid_linking

# Unit test: sessionId linking (agent sessions)
cargo test test_session_id_linking

# Unit test: Connected components
cargo test test_connected_components

# Integration test: Build chains from real data
cargo test test_build_chains_integration --ignored

# CLI test
./target/release/context-os build-chains
```

### Success Criteria

- [ ] leafUuid links extracted correctly (first record only)
- [ ] sessionId links extracted for agent sessions
- [ ] Connected components correctly grouped
- [ ] Chain statistics accurate (session count, agent count)
- [ ] Orphan sessions identified
- [ ] `build-chains` CLI command works
- [ ] Chains match Python output (313+ sessions in largest chain)

---

## Phase 5: File Watcher

**Estimated Time:** 6-8 hours
**Dependencies:** Phase 3 (JSONL Parser)
**Python Reference:** `cli/src/context_os_events/capture/file_watcher.py` (300 lines)

### Objective

Implement file system watching using the notify crate to detect new Claude session files.

### Type Contracts

```rust
// capture/file_watcher.rs

/// Configuration for file watcher
pub struct WatchConfig {
    /// Paths to watch
    pub paths: Vec<PathBuf>,
    /// Debounce interval in milliseconds
    pub debounce_ms: u64,
    /// File patterns to watch (e.g., "*.jsonl")
    pub patterns: Vec<String>,
}

/// Event from file watcher
#[derive(Debug)]
pub enum WatchEvent {
    Created(PathBuf),
    Modified(PathBuf),
    Deleted(PathBuf),
}

/// File watcher handle
pub struct FileWatcher {
    watcher: RecommendedWatcher,
    rx: Receiver<WatchEvent>,
}

impl FileWatcher {
    /// Create new file watcher
    pub fn new(config: WatchConfig) -> Result<Self> { ... }

    /// Start watching (non-blocking)
    pub fn start(&mut self) -> Result<()> { ... }

    /// Stop watching
    pub fn stop(&mut self) -> Result<()> { ... }

    /// Get next event (blocking with timeout)
    pub fn next_event(&self, timeout: Duration) -> Option<WatchEvent> { ... }
}
```

### Implementation Steps

1. **Add notify dependency:**
   ```toml
   [dependencies]
   notify = "6.1"
   notify-debouncer-mini = "0.4"
   ```

2. **Create capture/file_watcher.rs module:**
   - Define WatchConfig, WatchEvent types
   - Implement FileWatcher with notify
   - Add debounce logic

3. **Integrate with daemon:**
   - Watcher triggers parse on new JSONL files
   - Debounce prevents multiple parses on rapid changes

### Test Criteria

```bash
# Unit test: Watcher creation
cargo test test_watcher_creation

# Integration test: Detect file changes
cargo test test_detect_file_changes --ignored
```

### Success Criteria

- [ ] Watcher detects new JSONL files
- [ ] Debounce prevents duplicate events
- [ ] Cross-platform (Windows, macOS, Linux)
- [ ] Clean shutdown without resource leaks

---

## Phase 6: Daemon

**Estimated Time:** 6-8 hours
**Dependencies:** Phase 2, 3, 4, 5 (All capture/index modules)
**Python Reference:** `cli/src/context_os_events/daemon/runner.py` (290 lines)

### Objective

Port the daemon runner using tokio for async scheduling.

### Type Contracts

```rust
// daemon/runner.rs

/// Daemon configuration
pub struct DaemonConfig {
    /// Sync interval in minutes
    pub sync_interval_minutes: u32,
    /// Enable file watching
    pub watch_enabled: bool,
    /// Watch paths
    pub watch_paths: Vec<PathBuf>,
    /// Project path
    pub project_path: PathBuf,
}

/// Daemon state
pub struct DaemonState {
    pub started_at: DateTime<Utc>,
    pub last_git_sync: Option<DateTime<Utc>>,
    pub last_session_parse: Option<DateTime<Utc>>,
    pub last_chain_build: Option<DateTime<Utc>>,
    pub git_commits_synced: u32,
    pub sessions_parsed: u32,
    pub chains_built: u32,
}

/// Main daemon runner
pub struct Daemon {
    config: DaemonConfig,
    state: DaemonState,
    db: Database,
    watcher: Option<FileWatcher>,
}

impl Daemon {
    /// Create new daemon
    pub fn new(config: DaemonConfig) -> Result<Self> { ... }

    /// Start daemon (blocks until stopped)
    pub async fn run(&mut self) -> Result<()> { ... }

    /// Run single sync cycle
    pub async fn run_once(&mut self) -> Result<SyncResult> { ... }

    /// Graceful shutdown
    pub async fn stop(&mut self) -> Result<()> { ... }
}
```

### Implementation Steps

1. **Create daemon/runner.rs module:**
   - Define DaemonConfig, DaemonState types
   - Implement Daemon with tokio runtime
   - Implement scheduled sync using tokio::time::interval

2. **Add CLI command:**
   ```rust
   /// Run daemon
   Daemon {
       /// Run once and exit
       #[arg(long)]
       once: bool,

       /// Sync interval in minutes
       #[arg(long, default_value = "30")]
       interval: u32,
   },
   ```

3. **Implement sync loop:**
   ```rust
   impl Daemon {
       pub async fn run(&mut self) -> Result<()> {
           let mut interval = tokio::time::interval(
               Duration::from_secs(self.config.sync_interval_minutes as u64 * 60)
           );

           loop {
               tokio::select! {
                   _ = interval.tick() => {
                       self.run_sync().await?;
                   }
                   event = self.watcher.next_event() => {
                       if let Some(event) = event {
                           self.handle_watch_event(event).await?;
                       }
                   }
                   _ = tokio::signal::ctrl_c() => {
                       println!("Shutting down...");
                       break;
                   }
               }
           }

           self.stop().await
       }

       async fn run_sync(&mut self) -> Result<SyncResult> {
           // 1. Sync git commits
           let git_result = sync_commits(&self.db, SyncOptions::default())?;

           // 2. Parse sessions
           let parse_result = parse_sessions(&self.db, ParseOptions::default())?;

           // 3. Build chains
           let chain_result = build_chains(&self.db)?;

           // Update state
           self.state.last_git_sync = Some(Utc::now());
           self.state.last_session_parse = Some(Utc::now());
           self.state.last_chain_build = Some(Utc::now());
           self.state.git_commits_synced += git_result.commits_synced;
           self.state.sessions_parsed += parse_result.sessions_parsed;
           self.state.chains_built += chain_result.chains_built;

           Ok(SyncResult { ... })
       }
   }
   ```

### Test Criteria

```bash
# Unit test: Daemon creation
cargo test test_daemon_creation

# Integration test: Run once
cargo test test_daemon_run_once --ignored

# CLI test
./target/release/context-os daemon --once
./target/release/context-os daemon --interval 5
```

### Success Criteria

- [ ] Daemon starts and runs sync loop
- [ ] `--once` flag runs single sync and exits
- [ ] File watcher triggers parse on new files
- [ ] Graceful shutdown on Ctrl+C
- [ ] State persisted between runs
- [ ] `daemon` CLI command works
- [ ] Performance: Cold start <100ms

---

## Implementation Timeline

| Phase | Component | Hours | Dependencies | Milestone |
|-------|-----------|-------|--------------|-----------|
| 1 | Storage Foundation | 4-6 | None | Can INSERT to database |
| 2 | Git Sync | 8-12 | Phase 1 | `sync-git` command works |
| 3 | JSONL Parser | 12-16 | Phase 1 | `parse-sessions` command works |
| 4 | Chain Graph | 8-12 | Phase 3 | `build-chains` command works |
| 5 | File Watcher | 6-8 | Phase 3 | Background watching works |
| 6 | Daemon | 6-8 | Phases 2-5 | `daemon` command works |
| **Total** | | **44-62** | | Single Rust binary |

### Parallel Work Opportunities

- Phase 2 (Git Sync) and Phase 3 (JSONL Parser) can be developed in parallel after Phase 1
- Phase 5 (File Watcher) can be developed in parallel with Phase 4 (Chain Graph)

### Risk Mitigation

1. **Python CLI remains functional:** Don't remove Python code until Rust equivalent is tested
2. **Incremental testing:** Each phase has its own CLI command for isolated testing
3. **Feature parity verification:** Compare Rust output with Python output for validation

---

## Appendix A: Cargo.toml Dependencies

```toml
[dependencies]
# Existing
clap = { version = "4.4", features = ["derive"] }
sqlx = { version = "0.7", features = ["runtime-tokio", "sqlite"] }
tokio = { version = "1.35", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
axum = "0.7"
tower-http = { version = "0.5", features = ["cors"] }
chrono = { version = "0.4", features = ["serde"] }
anyhow = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"

# New for port
git2 = "0.18"                    # Native git operations
notify = "6.1"                   # File system watching
notify-debouncer-mini = "0.4"    # Event debouncing
r2d2 = "0.8"                     # Connection pooling
r2d2_sqlite = "0.24"             # SQLite connection manager
rusqlite = { version = "0.31", features = ["bundled"] }  # Direct SQLite access
dirs = "5.0"                     # Home directory resolution
```

---

## Appendix B: Database Schema Reference

Tables to support (from Python schema.py):

```sql
-- Git commits
CREATE TABLE IF NOT EXISTS git_commits (
    id INTEGER PRIMARY KEY,
    hash TEXT UNIQUE NOT NULL,
    short_hash TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    author_name TEXT NOT NULL,
    author_email TEXT NOT NULL,
    subject TEXT NOT NULL,
    parent_hashes TEXT,
    is_agent_commit INTEGER DEFAULT 0,
    project_path TEXT
);

-- Claude sessions
CREATE TABLE IF NOT EXISTS claude_sessions (
    id INTEGER PRIMARY KEY,
    session_id TEXT UNIQUE NOT NULL,
    project_path TEXT NOT NULL,
    started_at TEXT NOT NULL,
    ended_at TEXT,
    message_count INTEGER DEFAULT 0,
    user_messages INTEGER DEFAULT 0,
    assistant_messages INTEGER DEFAULT 0,
    leaf_uuid TEXT,
    parent_session_id TEXT,
    file_size INTEGER DEFAULT 0
);

-- Tool uses
CREATE TABLE IF NOT EXISTS tool_uses (
    id INTEGER PRIMARY KEY,
    session_id TEXT NOT NULL,
    tool_name TEXT NOT NULL,
    file_path TEXT,
    timestamp TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES claude_sessions(session_id)
);

-- Chains
CREATE TABLE IF NOT EXISTS chains (
    id TEXT PRIMARY KEY,
    root_session_id TEXT NOT NULL,
    session_count INTEGER DEFAULT 0,
    agent_count INTEGER DEFAULT 0,
    created_at TEXT NOT NULL,
    last_activity TEXT NOT NULL,
    FOREIGN KEY (root_session_id) REFERENCES claude_sessions(session_id)
);

-- Chain graph (session-to-chain mapping)
CREATE TABLE IF NOT EXISTS chain_graph (
    session_id TEXT PRIMARY KEY,
    chain_id TEXT NOT NULL,
    parent_id TEXT,
    depth INTEGER DEFAULT 0,
    is_agent_session INTEGER DEFAULT 0,
    FOREIGN KEY (session_id) REFERENCES claude_sessions(session_id),
    FOREIGN KEY (chain_id) REFERENCES chains(id)
);
```

---

## Appendix C: Test Data Validation

To validate Rust port matches Python output:

```bash
# Python baseline
python -c "
from context_os_events.index.chain_graph import build_chain_graph, persist_chains
from context_os_events.db.connection import get_connection
from pathlib import Path

db = get_connection()
chains = build_chain_graph(Path.home() / '.claude' / 'projects' / 'YOUR_PROJECT')
print(f'Chains: {len(chains)}')
print(f'Largest chain: {max(len(c.sessions) for c in chains)} sessions')
"

# Rust validation
./target/release/context-os build-chains
sqlite3 ~/.context-os/context_os_events.db "
SELECT COUNT(*) as chains FROM chains;
SELECT MAX(session_count) as largest FROM chains;
"
```

Expected: Rust output matches Python output (within 1-2% variance for edge cases).
