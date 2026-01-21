# Rust Port Type Contracts

**Purpose:** Exact type definitions for porting Python daemon to Rust
**Status:** In Progress
**Created:** 2026-01-17

---

## Overview

This document defines the exact type contracts for each phase of the Rust port. Each section maps Python dataclasses/TypedDicts to Rust structs with serde serialization.

**Design Principle:** Types are the contract. If Rust types serialize to identical JSON as Python, the port is correct.

---

## Phase 3: Git Sync

### Python Source: `capture/git_sync.py`

```python
@dataclass
class GitCommit:
    hash: str                    # Full 40-char hash
    short_hash: str              # First 7 chars
    timestamp: datetime          # Author date
    author_name: str
    author_email: str
    message: str                 # Full commit message (first line)
    files_changed: List[str]     # All files (A/M/D)
    files_added: List[str]       # Files with 'A' status
    files_modified: List[str]    # Files with 'M' status
    files_deleted: List[str]     # Files with 'D' status
    insertions: int              # Lines added
    deletions: int               # Lines removed
    files_count: int             # Total files changed
    is_agent_commit: bool        # Contains Claude Code signature
    is_merge_commit: bool        # Has multiple parents
```

### Rust Target: `capture/git_sync.rs`

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Parsed git commit with all metadata.
/// Maps 1:1 to Python GitCommit dataclass.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitCommit {
    /// Full 40-char hash
    pub hash: String,
    /// First 7 chars
    pub short_hash: String,
    /// Author date (ISO8601)
    pub timestamp: DateTime<Utc>,
    /// Author name
    pub author_name: String,
    /// Author email
    pub author_email: String,
    /// Commit message (first line / subject)
    pub message: String,
    /// All files changed (A/M/D combined)
    pub files_changed: Vec<String>,
    /// Files with 'A' status
    pub files_added: Vec<String>,
    /// Files with 'M' status
    pub files_modified: Vec<String>,
    /// Files with 'D' status
    pub files_deleted: Vec<String>,
    /// Lines added
    pub insertions: i32,
    /// Lines removed
    pub deletions: i32,
    /// Total files changed count
    pub files_count: i32,
    /// True if commit contains Claude Code signature
    pub is_agent_commit: bool,
    /// True if commit has multiple parents
    pub is_merge_commit: bool,
}

/// Options for git sync operation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SyncOptions {
    /// Time range: "90 days", "2025-01-01", etc.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub since: Option<String>,
    /// Upper bound date
    #[serde(skip_serializing_if = "Option::is_none")]
    pub until: Option<String>,
    /// Path to git repository (default: ".")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo_path: Option<String>,
    /// Only sync new commits (default: true)
    #[serde(default = "default_true")]
    pub incremental: bool,
}

fn default_true() -> bool { true }

/// Result of sync operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    /// Number of commits successfully synced
    pub commits_synced: i32,
    /// Number of commits skipped (already in DB)
    pub commits_skipped: i32,
    /// Hash of most recent commit synced
    pub last_hash: String,
    /// Non-fatal parse errors
    pub errors: Vec<String>,
}

/// Agent commit detection signatures
pub const AGENT_SIGNATURES: &[&str] = &[
    "generated with claude code",
    "🤖 generated with",
    "co-authored-by: claude",
];

/// Detect if commit message contains Claude Code signature.
pub fn detect_agent_commit(message: &str) -> bool {
    let message_lower = message.to_lowercase();
    AGENT_SIGNATURES.iter().any(|sig| message_lower.contains(sig))
}
```

### Key Functions to Port

| Python Function | Rust Function | Notes |
|-----------------|---------------|-------|
| `sync_commits(db, options)` | `pub fn sync_commits(storage: &Storage, options: &SyncOptions) -> Result<SyncResult>` | Main entry |
| `parse_commit_block(block)` | `fn parse_commit_block(block: &str) -> Result<GitCommit>` | Parse git log output |
| `split_commit_blocks(raw)` | `fn split_commit_blocks(raw: &str) -> Vec<String>` | Split by 40-char hash§ |
| `detect_agent_commit(msg)` | `pub fn detect_agent_commit(message: &str) -> bool` | Check signatures |
| `get_commit_body(hash, repo)` | `fn get_commit_body(hash: &str, repo: &Path) -> String` | Fetch full body |

### Dependencies

```toml
[dependencies]
git2 = { version = "0.19", features = ["bundled"] }
chrono = { version = "0.4", features = ["serde"] }
```

---

## Phase 4: JSONL Parser

### Python Source: `capture/jsonl_parser.py`

```python
@dataclass
class ToolUse:
    id: str
    name: str                    # Read, Edit, Write, Grep, etc.
    input: Dict[str, Any]        # Tool-specific inputs
    timestamp: datetime
    file_path: Optional[str]     # Primary file being accessed
    is_read: bool                # True for Read, Grep, Glob
    is_write: bool               # True for Edit, Write

@dataclass
class ParsedMessage:
    type: str                    # user, assistant, tool_result
    role: Optional[str]          # user, assistant
    content: Any                 # str or List[content_block]
    timestamp: datetime
    tool_uses: List[ToolUse]     # Extracted from content if assistant

@dataclass
class SessionSummary:
    session_id: str              # UUID from filename
    project_path: str            # Decoded project path
    started_at: datetime
    ended_at: datetime
    duration_seconds: int
    user_message_count: int
    assistant_message_count: int
    total_messages: int
    files_read: List[str]        # Unique files read
    files_written: List[str]     # Unique files written/edited
    files_created: List[str]     # Files created (Write to new path)
    tools_used: Dict[str, int]   # {"Read": 15, "Edit": 8, ...}
    grep_patterns: List[str]     # Patterns used in Grep calls
    file_size_bytes: int         # JSONL file size
```

### Rust Target: `capture/jsonl_parser.rs`

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Extracted tool use from assistant message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUse {
    /// Tool use ID
    pub id: String,
    /// Tool name (Read, Edit, Write, Grep, etc.)
    pub name: String,
    /// Tool-specific inputs (preserved as JSON)
    pub input: Value,
    /// Timestamp of the message
    pub timestamp: DateTime<Utc>,
    /// Primary file being accessed (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
    /// True for Read, Grep, Glob, WebFetch, WebSearch
    pub is_read: bool,
    /// True for Edit, Write, NotebookEdit
    pub is_write: bool,
}

/// Single parsed message from JSONL.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedMessage {
    /// Message type: user, assistant, tool_result, file-history-snapshot
    #[serde(rename = "type")]
    pub msg_type: String,
    /// Role: user or assistant
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// Content (string or array of content blocks)
    pub content: Value,
    /// Message timestamp
    pub timestamp: DateTime<Utc>,
    /// Extracted tool uses from content
    pub tool_uses: Vec<ToolUse>,
}

/// Aggregated session data for database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    /// Session ID (UUID from filename)
    pub session_id: String,
    /// Decoded project path
    pub project_path: String,
    /// Session start time
    pub started_at: DateTime<Utc>,
    /// Session end time
    pub ended_at: DateTime<Utc>,
    /// Duration in seconds
    pub duration_seconds: i32,
    /// Number of user messages
    pub user_message_count: i32,
    /// Number of assistant messages
    pub assistant_message_count: i32,
    /// Total message count
    pub total_messages: i32,
    /// Unique files read
    pub files_read: Vec<String>,
    /// Unique files written/edited
    pub files_written: Vec<String>,
    /// Files created (Write to new path)
    pub files_created: Vec<String>,
    /// Tool usage counts: {"Read": 15, "Edit": 8, ...}
    pub tools_used: HashMap<String, i32>,
    /// Grep patterns used (automation candidates)
    pub grep_patterns: Vec<String>,
    /// JSONL file size in bytes
    pub file_size_bytes: i64,
}

/// Options for parsing sessions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ParseOptions {
    /// Path to project (will be encoded)
    pub project_path: String,
    /// Only parse new/modified sessions (default: true)
    #[serde(default = "default_true")]
    pub incremental: bool,
}

/// Result of parse operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseResult {
    /// Number of sessions parsed
    pub sessions_parsed: i32,
    /// Number of sessions skipped (already in DB, unchanged)
    pub sessions_skipped: i32,
    /// Total tool uses extracted
    pub total_tool_uses: i32,
    /// Non-fatal errors
    pub errors: Vec<String>,
}

/// Read tools (is_read = true)
pub const READ_TOOLS: &[&str] = &["Read", "Grep", "Glob", "WebFetch", "WebSearch"];

/// Write tools (is_write = true)
pub const WRITE_TOOLS: &[&str] = &["Edit", "Write", "NotebookEdit"];
```

### 3-Source Extraction (Critical)

The JSONL parser must extract file paths from THREE sources:

```rust
/// Extract tool uses from a JSONL record.
///
/// CRITICAL: Three extraction sources (Phase 2.5 fix):
/// 1. `assistant.tool_use` blocks - standard tool calls
/// 2. `user.toolUseResult` - Gap 1 fix (file paths in results)
/// 3. `file-history-snapshot` - Gap 2 fix (tracked file backups)
pub fn extract_tool_uses(record: &Value, timestamp: DateTime<Utc>) -> Vec<ToolUse> {
    let mut tool_uses = Vec::new();

    let msg_type = record.get("type").and_then(|v| v.as_str()).unwrap_or("");

    match msg_type {
        // Source 1: Assistant messages with tool_use blocks
        "assistant" => {
            if let Some(content) = record.get("message")
                .and_then(|m| m.get("content"))
                .and_then(|c| c.as_array())
            {
                for block in content {
                    if block.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
                        // Extract tool use...
                    }
                }
            }
        }

        // Source 2: User messages with toolUseResult (Gap 1)
        "user" => {
            if let Some(result) = record.get("toolUseResult") {
                // Extract file path from result.filePath or result.file.filePath
                // Map result.type to is_read/is_write
            }
        }

        // Source 3: file-history-snapshot records (Gap 2)
        "file-history-snapshot" => {
            if let Some(tracked) = record.get("snapshot")
                .and_then(|s| s.get("trackedFileBackups"))
                .and_then(|t| t.as_object())
            {
                for file_path in tracked.keys() {
                    // Create tool use with is_read=true
                }
            }
        }

        _ => {}
    }

    tool_uses
}
```

### Path Encoding (Critical)

```rust
/// Encode filesystem path to Claude project directory name.
///
/// Windows: C:\Users\dietl\Project → C--Users-dietl-Project
/// Unix: /home/user/project → -home-user-project
pub fn encode_project_path(path: &Path) -> String {
    let path_str = path.to_string_lossy();

    if path_str.contains(':') {
        // Windows: C:\foo → C--foo
        path_str
            .replace(":\\", "--")
            .replace('\\', "-")
            .replace(' ', "-")
            .replace('_', "-")
    } else {
        // Unix: /foo/bar → -foo-bar
        path_str
            .replace('/', "-")
            .replace(' ', "-")
            .replace('_', "-")
    }
}

/// Decode Claude project directory name to filesystem path.
pub fn decode_project_path(encoded: &str) -> String {
    // Detect Windows (starts with X--)
    if encoded.len() >= 3 && encoded.chars().nth(1) == Some('-') && encoded.chars().nth(2) == Some('-') {
        let drive = &encoded[0..1];
        let rest = &encoded[3..];
        format!("{}:\\{}", drive, rest.replace('-', "\\"))
    } else {
        // Unix
        encoded.replace('-', "/")
    }
}
```

### Key Functions to Port

| Python Function | Rust Function | Notes |
|-----------------|---------------|-------|
| `sync_sessions(db, options)` | `pub fn sync_sessions(storage: &Storage, options: &ParseOptions) -> Result<ParseResult>` | Main entry |
| `parse_jsonl_line(line)` | `fn parse_jsonl_line(line: &str) -> Option<ParsedMessage>` | Parse single line |
| `extract_tool_uses(content, ts)` | `fn extract_tool_uses(record: &Value, ts: DateTime<Utc>) -> Vec<ToolUse>` | 3-source extraction |
| `extract_file_path(name, input)` | `fn extract_file_path(tool_name: &str, input: &Value) -> Option<String>` | Get primary file |
| `aggregate_session(...)` | `fn aggregate_session(...) -> SessionSummary` | Build summary |
| `find_session_files(path)` | `fn find_session_files(path: &Path) -> Vec<PathBuf>` | **/*.jsonl glob |
| `encode_project_path(path)` | `pub fn encode_project_path(path: &Path) -> String` | Path encoding |

---

## Phase 5: Chain Graph

### Python Source: `index/chain_graph.py`

```python
@dataclass
class ChainNode:
    session_id: str
    parent_session_id: Optional[str]  # Session containing the leafUuid message
    parent_message_uuid: str          # The actual leafUuid value
    children: List[str] = field(default_factory=list)

@dataclass
class Chain:
    chain_id: str                     # Generated hash of root session
    root_session: str                 # First session (no parent)
    sessions: List[str]               # All sessions in order
    branches: Dict[str, List[str]]    # parent -> [children]
    time_range: Optional[Tuple[datetime, datetime]]
    total_duration_seconds: int
    files_bloom: Optional[bytes]      # Bloom filter (serialized)
    files_list: List[str]             # All unique files touched
```

### Rust Target: `index/chain_graph.rs`

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Single session's position in the chain graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainNode {
    /// Session ID (UUID)
    pub session_id: String,
    /// Parent session ID (contains the leafUuid message)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_session_id: Option<String>,
    /// The leafUuid value that links to parent
    pub parent_message_uuid: String,
    /// Session IDs that continue from this session
    pub children: Vec<String>,
}

/// A connected chain of sessions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chain {
    /// Chain ID (MD5 hash of root session, first 8 chars)
    pub chain_id: String,
    /// First session in chain (no parent)
    pub root_session: String,
    /// All sessions in traversal order
    pub sessions: Vec<String>,
    /// Branch structure: parent_session -> [child_sessions]
    pub branches: HashMap<String, Vec<String>>,
    /// Time range (start, end)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
    /// Total duration across all sessions
    pub total_duration_seconds: i32,
    /// Bloom filter of all files (serialized)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files_bloom: Option<Vec<u8>>,
    /// All unique files touched
    pub files_list: Vec<String>,
}

/// Result of chain building.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainBuildResult {
    /// Chains stored
    pub chains_stored: i32,
    /// Sessions linked
    pub sessions_stored: i32,
}
```

### 5-Pass Algorithm (Critical)

```rust
/// Build chain graph from session linking in JSONL files.
///
/// Algorithm (5 passes):
/// 1. Extract leafUuid from LAST summary record (not first!)
/// 2. Extract sessionId from agent sessions (agent-* files)
/// 3. Extract message.uuid ownership (who owns which UUID)
/// 4. Build parent-child relationships from both mechanisms
/// 5. Group into chains via BFS connected components
///
/// CRITICAL: Use LAST summary's leafUuid for immediate parent,
/// not first (which points to root ancestor).
pub fn build_chain_graph(jsonl_dir: &Path) -> Result<HashMap<String, Chain>> {
    // Find all JSONL files (recursive: **/*.jsonl)
    let jsonl_files: Vec<PathBuf> = glob::glob(&format!("{}/**/*.jsonl", jsonl_dir.display()))?
        .filter_map(|e| e.ok())
        .collect();

    // Separate regular and agent sessions
    let regular_files: Vec<_> = jsonl_files.iter()
        .filter(|f| !f.file_stem().unwrap().to_string_lossy().starts_with("agent-"))
        .collect();
    let agent_files: Vec<_> = jsonl_files.iter()
        .filter(|f| f.file_stem().unwrap().to_string_lossy().starts_with("agent-"))
        .collect();

    // Pass 1: Collect leafUuid -> [child sessions]
    let mut leaf_refs: HashMap<String, Vec<String>> = HashMap::new();
    for file in &regular_files {
        let session_id = file.file_stem().unwrap().to_string_lossy().to_string();
        if let Some(leaf_uuid) = extract_last_leaf_uuid(file)? {
            leaf_refs.entry(leaf_uuid).or_default().push(session_id);
        }
    }

    // Pass 2: Collect agent -> parent relationships
    let mut agent_parents: HashMap<String, String> = HashMap::new();
    for file in &agent_files {
        let session_id = file.file_stem().unwrap().to_string_lossy().to_string();
        if let Some(parent_id) = extract_agent_parent(file)? {
            agent_parents.insert(session_id, parent_id);
        }
    }

    // Pass 3: Collect uuid -> owning session
    let mut uuid_to_session: HashMap<String, String> = HashMap::new();
    for file in &jsonl_files {
        let session_id = file.file_stem().unwrap().to_string_lossy().to_string();
        for uuid in extract_message_uuids(file)? {
            uuid_to_session.insert(uuid, session_id.clone());
        }
    }

    // Pass 4: Build parent -> children map
    let mut children_map: HashMap<String, Vec<String>> = HashMap::new();

    // 4a: Regular session linking via leafUuid
    for (leaf_uuid, child_sessions) in &leaf_refs {
        if let Some(parent_session) = uuid_to_session.get(leaf_uuid) {
            for child in child_sessions {
                if child != parent_session {
                    children_map.entry(parent_session.clone())
                        .or_default()
                        .push(child.clone());
                }
            }
        }
    }

    // 4b: Agent session linking via sessionId
    for (agent_session, parent_session) in &agent_parents {
        if agent_session != parent_session {
            children_map.entry(parent_session.clone())
                .or_default()
                .push(agent_session.clone());
        }
    }

    // Pass 5: Group into chains via BFS
    // ... (connected components algorithm)

    Ok(chains)
}

/// Extract leafUuid from LAST summary record.
///
/// CRITICAL: Claude stacks summaries oldest-first:
/// - Session B continues from A: B has summary with leafUuid -> A
/// - Session C continues from B: C has [summary A, summary B]
/// - FIRST summary points to root ancestor
/// - LAST summary points to immediate parent
fn extract_last_leaf_uuid(filepath: &Path) -> Result<Option<String>> {
    let file = std::fs::File::open(filepath)?;
    let reader = std::io::BufReader::new(file);

    let mut last_leaf_uuid: Option<String> = None;

    for line in reader.lines() {
        let line = line?;
        if line.is_empty() { continue; }

        let record: Value = serde_json::from_str(&line)?;

        if record.get("type").and_then(|t| t.as_str()) == Some("summary") {
            if let Some(leaf) = record.get("leafUuid").and_then(|l| l.as_str()) {
                last_leaf_uuid = Some(leaf.to_string());
            }
        } else {
            // Stop at first non-summary record
            break;
        }
    }

    Ok(last_leaf_uuid)
}
```

---

## Phase 6: Inverted Index

### Python Source: `index/inverted_index.py`

```python
@dataclass
class FileAccess:
    file_path: str
    session_id: str
    chain_id: Optional[str]
    timestamp: datetime
    access_type: str             # read, write, create
    tool_name: str               # Read, Edit, Write, etc.
```

### Rust Target: `index/inverted_index.rs`

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Single file access record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAccess {
    /// File path (relative or absolute)
    pub file_path: String,
    /// Session that accessed the file
    pub session_id: String,
    /// Chain the session belongs to
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chain_id: Option<String>,
    /// Access timestamp
    pub timestamp: DateTime<Utc>,
    /// Access type: read, write, create
    pub access_type: String,
    /// Tool used: Read, Edit, Write, etc.
    pub tool_name: String,
}

/// Result of index building.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexBuildResult {
    /// Total file accesses indexed
    pub accesses_indexed: i32,
    /// Unique files
    pub unique_files: i32,
    /// Unique sessions
    pub unique_sessions: i32,
}
```

---

## Phase 7: File Watcher

### Python Source: `capture/file_watcher.py`

```python
@dataclass
class FileEvent:
    timestamp: datetime
    path: str                    # Relative to repo root
    event_type: str              # create, write, delete, rename
    size_bytes: Optional[int]    # File size (None for delete)
    old_path: Optional[str]      # Previous path for renames
    is_directory: bool
    extension: Optional[str]     # File extension
```

### Rust Target: `capture/file_watcher.rs`

```rust
use chrono::{DateTime, Utc};
use notify::{Event, EventKind};
use serde::{Deserialize, Serialize};

/// A file system event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEvent {
    /// Event timestamp
    pub timestamp: DateTime<Utc>,
    /// Path relative to repo root
    pub path: String,
    /// Event type: create, write, delete, rename
    pub event_type: String,
    /// File size in bytes (None for delete)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<i64>,
    /// Previous path (for renames)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_path: Option<String>,
    /// True if directory
    pub is_directory: bool,
    /// File extension
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extension: Option<String>,
}

/// Default ignore patterns (version control, build artifacts, etc.)
pub const DEFAULT_IGNORE_PATTERNS: &[&str] = &[
    ".git", ".git/*", "*/.git/*",
    "__pycache__", "__pycache__/*", "*/__pycache__/*",
    "*.pyc", "*.pyo", "*.pyd",
    "node_modules", "node_modules/*", "*/node_modules/*",
    ".idea", ".idea/*", ".vscode", ".vscode/*",
    "*.db", "*.db-journal", "*.db-wal", "*.db-shm",
    "*.log", "*.tmp", "*.temp", "*.bak",
    "dist", "dist/*", "build", "build/*",
    "target", "target/*",  // Rust builds
];
```

### Dependencies

```toml
[dependencies]
notify = "6.1"
notify-debouncer-mini = "0.4"
```

---

## Phase 8: Daemon Runner

### Python Source: `daemon/runner.py`

```python
class ContextOSDaemon:
    config: DaemonConfig
    state: DaemonState

    def start(self) -> None
    def stop(self) -> None
    def run_sync(self) -> None
    def on(self, event: str, handler: EventHandler) -> None
    def emit(self, event: str, data: dict) -> None
```

### Rust Target: `daemon/runner.rs`

```rust
use std::sync::{Arc, Mutex};
use tokio::time::{interval, Duration};

/// Daemon configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    pub sync: SyncConfig,
    pub watch: WatchConfig,
    pub project: ProjectConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    /// Sync interval in minutes
    pub interval_minutes: u32,
    /// Git history days
    pub git_since_days: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchConfig {
    /// Enable file watching
    pub enabled: bool,
    /// Paths to watch
    pub paths: Vec<String>,
    /// Debounce window in ms
    pub debounce_ms: u32,
}

/// Main daemon orchestrator.
pub struct ContextOSDaemon {
    config: DaemonConfig,
    storage: Arc<Storage>,
    running: Arc<Mutex<bool>>,
}

impl ContextOSDaemon {
    pub fn new(config: DaemonConfig, storage: Storage) -> Self { ... }

    /// Start daemon: file watcher + scheduler.
    pub async fn start(&self) -> Result<()> { ... }

    /// Graceful shutdown.
    pub async fn stop(&self) -> Result<()> { ... }

    /// Run git sync + session parse + chain building.
    pub async fn run_sync(&self) -> Result<SyncStats> { ... }
}

/// Daemon entry point.
pub async fn run_daemon(config: DaemonConfig) -> Result<()> {
    let storage = Storage::open_rw(&get_db_path())?;
    let daemon = ContextOSDaemon::new(config, storage);

    // Setup signal handlers
    let running = daemon.running.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        *running.lock().unwrap() = false;
    });

    // Start daemon
    daemon.start().await?;

    // Run until stopped
    while *daemon.running.lock().unwrap() {
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    daemon.stop().await
}
```

---

## Database Schema Extensions

The Rust storage layer needs these INSERT operations:

```rust
impl Storage {
    /// Insert a git commit.
    pub fn insert_commit(&self, commit: &GitCommit) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO git_commits (...) VALUES (...)",
            params![...]
        )?;
        Ok(())
    }

    /// Insert a session.
    pub fn insert_session(&self, session: &SessionSummary) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO claude_sessions (...) VALUES (...)",
            params![...]
        )?;
        Ok(())
    }

    /// Insert a chain.
    pub fn insert_chain(&self, chain: &Chain) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO chains (...) VALUES (...)",
            params![...]
        )?;
        Ok(())
    }

    /// Insert a chain graph node.
    pub fn insert_chain_node(&self, node: &ChainNode, chain_id: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO chain_graph (...) VALUES (...)",
            params![...]
        )?;
        Ok(())
    }

    /// Insert a file access.
    pub fn insert_file_access(&self, access: &FileAccess) -> Result<()> {
        self.conn.execute(
            "INSERT INTO file_accesses (...) VALUES (...)",
            params![...]
        )?;
        Ok(())
    }

    /// Insert a file event.
    pub fn insert_file_event(&self, event: &FileEvent) -> Result<()> {
        self.conn.execute(
            "INSERT INTO file_events (...) VALUES (...)",
            params![...]
        )?;
        Ok(())
    }
}
```

---

## CLI Commands

Each phase adds new CLI commands:

```bash
# Phase 3
context-os sync-git --since 30d [--repo PATH] [--incremental]

# Phase 4
context-os parse-sessions --project . [--incremental]

# Phase 5
context-os build-chains

# Phase 6
context-os build-index

# Phase 7 + 8
context-os daemon [--once] [--interval 30]
```

---

## Verification Strategy

After each phase:

```bash
# Run Python to generate baseline
tastematter sync-git --since 30d > /tmp/py_commits.json
tastematter parse-sessions > /tmp/py_sessions.json

# Run Rust
context-os sync-git --since 30d > /tmp/rs_commits.json
context-os parse-sessions > /tmp/rs_sessions.json

# Compare (counts should match)
jq '.commits_synced' /tmp/py_commits.json /tmp/rs_commits.json
jq '.sessions_parsed' /tmp/py_sessions.json /tmp/rs_sessions.json
```

---

## Next Steps

1. **Phase 3 Implementation**
   - Add `git2` dependency
   - Create `capture/git_sync.rs`
   - Port `sync_commits`, `parse_commit_block`, `detect_agent_commit`
   - Add CLI command
   - Write TDD tests

2. **Phase 4 Implementation**
   - Create `capture/jsonl_parser.rs`
   - Port 3-source extraction
   - Port path encoding
   - Add CLI command
   - Write TDD tests

Continue with subsequent phases following TDD approach.
