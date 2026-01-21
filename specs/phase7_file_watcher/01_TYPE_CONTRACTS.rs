//! Phase 7: File Watcher - Type Contracts
//!
//! Exact Rust type definitions for porting Python file_watcher.py
//! Design Principle: Types are the contract. If Rust types serialize to
//! identical JSON as Python, the port is correct.
//!
//! Created: 2026-01-18
//! Python Source: cli/src/context_os_events/capture/file_watcher.py (568 lines)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

// ============================================================================
// FILE EVENT (matches Python FileEvent dataclass, line 28-36)
// ============================================================================

/// A file system event.
///
/// Maps 1:1 to Python FileEvent dataclass.
///
/// Python source:
/// ```python
/// @dataclass
/// class FileEvent:
///     timestamp: datetime
///     path: str                    # Relative to repo root
///     event_type: str              # create, write, delete, rename
///     size_bytes: Optional[int]    # File size (None for delete)
///     old_path: Optional[str]      # Previous path for renames
///     is_directory: bool
///     extension: Optional[str]     # File extension
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEvent {
    /// Event timestamp (ISO8601)
    pub timestamp: DateTime<Utc>,
    /// Path relative to repo root
    pub path: String,
    /// Event type: "create", "write", "delete", "rename"
    pub event_type: String,
    /// File size in bytes (None for delete events)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<i64>,
    /// Previous path for rename events
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_path: Option<String>,
    /// Whether this is a directory event
    pub is_directory: bool,
    /// File extension (e.g., ".rs", ".py")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extension: Option<String>,
}

// ============================================================================
// EVENT TYPES (string constants)
// ============================================================================

/// Valid event types matching Python implementation.
pub mod event_types {
    pub const CREATE: &str = "create";
    pub const WRITE: &str = "write";
    pub const DELETE: &str = "delete";
    pub const RENAME: &str = "rename";
}

// ============================================================================
// DEFAULT IGNORE PATTERNS (matches Python, lines 43-113)
// ============================================================================

/// Default ignore patterns for file watching.
///
/// 40+ patterns matching Python DEFAULT_IGNORE_PATTERNS.
/// Patterns use glob/fnmatch syntax.
pub const DEFAULT_IGNORE_PATTERNS: &[&str] = &[
    // Version control
    ".git",
    ".git/*",
    "*/.git/*",
    ".svn",
    ".svn/*",
    "*/.svn/*",
    ".hg",
    ".hg/*",
    "*/.hg/*",

    // Python
    "__pycache__",
    "__pycache__/*",
    "*/__pycache__/*",
    "*.pyc",
    "*.pyo",
    "*.pyd",
    ".pytest_cache",
    ".pytest_cache/*",
    "*/.pytest_cache/*",
    ".venv",
    ".venv/*",
    "*/.venv/*",
    "venv",
    "venv/*",
    "*/venv/*",
    "*.egg-info",
    "*.egg-info/*",

    // Node.js
    "node_modules",
    "node_modules/*",
    "*/node_modules/*",
    "*.min.js",
    "*.min.css",

    // IDE
    ".idea",
    ".idea/*",
    "*/.idea/*",
    ".vscode",
    ".vscode/*",
    "*/.vscode/*",
    "*.swp",
    "*.swo",
    "*~",
    ".DS_Store",

    // Build artifacts
    "dist",
    "dist/*",
    "build",
    "build/*",
    "*.egg",

    // SQLite
    "*.db",
    "*.db-journal",
    "*.db-wal",
    "*.db-shm",
    "*.sqlite",
    "*.sqlite3",

    // Logs and temp
    "*.log",
    "*.tmp",
    "*.temp",
    "*.bak",
];

// ============================================================================
// EVENT FILTER (matches Python EventFilter class, lines 120-188)
// ============================================================================

/// Filters file events based on ignore patterns.
///
/// Python source: EventFilter class (lines 120-188)
pub struct EventFilter {
    /// Root directory being watched (resolved absolute path)
    pub watch_path: PathBuf,
    /// Patterns to ignore
    pub ignore_patterns: Vec<String>,
}

impl EventFilter {
    /// Create a new filter with default ignore patterns.
    pub fn new(watch_path: &str) -> Self {
        Self::with_patterns(watch_path, None)
    }

    /// Create a new filter with custom ignore patterns.
    pub fn with_patterns(watch_path: &str, patterns: Option<Vec<String>>) -> Self {
        let patterns = patterns.unwrap_or_else(|| {
            DEFAULT_IGNORE_PATTERNS.iter().map(|s| s.to_string()).collect()
        });
        Self {
            watch_path: PathBuf::from(watch_path).canonicalize().unwrap_or_else(|_| PathBuf::from(watch_path)),
            ignore_patterns: patterns,
        }
    }

    /// Check if a path should be ignored.
    ///
    /// Returns true if path matches any ignore pattern.
    /// Matches both full path and individual path components.
    pub fn should_ignore(&self, path: &str) -> bool {
        // Implementation in file_watcher.rs
        todo!()
    }

    /// Convert absolute path to relative path.
    pub fn get_relative_path(&self, path: &str) -> String {
        // Implementation in file_watcher.rs
        todo!()
    }
}

// ============================================================================
// EVENT DEBOUNCER (matches Python EventDebouncer class, lines 195-266)
// ============================================================================

/// Consolidates rapid events on the same file.
///
/// Python source: EventDebouncer class (lines 195-266)
///
/// Key behaviors:
/// - Events for same path replace previous event
/// - Events are flushed after debounce_ms milliseconds
/// - Thread-safe via Mutex
pub struct EventDebouncer {
    /// Debounce window in milliseconds
    pub debounce_ms: u64,
    /// Pending events by path
    pending: Mutex<HashMap<String, FileEvent>>,
    /// Timestamps by path (for debounce calculation)
    timestamps: Mutex<HashMap<String, std::time::Instant>>,
}

impl EventDebouncer {
    /// Create a new debouncer with default 100ms window.
    pub fn new() -> Self {
        Self::with_debounce(100)
    }

    /// Create a new debouncer with custom debounce window.
    pub fn with_debounce(debounce_ms: u64) -> Self {
        Self {
            debounce_ms,
            pending: Mutex::new(HashMap::new()),
            timestamps: Mutex::new(HashMap::new()),
        }
    }

    /// Add an event to the buffer.
    ///
    /// If an event for the same path exists, it will be replaced.
    pub fn add(&self, event: FileEvent) {
        // Implementation in file_watcher.rs
        todo!()
    }

    /// Get the number of pending events.
    pub fn pending_count(&self) -> usize {
        // Implementation in file_watcher.rs
        todo!()
    }

    /// Flush events that have passed the debounce window.
    ///
    /// Returns list of events ready to be processed.
    pub fn flush(&self) -> Vec<FileEvent> {
        // Implementation in file_watcher.rs
        todo!()
    }

    /// Flush all pending events regardless of time.
    pub fn flush_all(&self) -> Vec<FileEvent> {
        // Implementation in file_watcher.rs
        todo!()
    }
}

impl Default for EventDebouncer {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// WATCHER STATS (for monitoring)
// ============================================================================

/// Statistics for file watcher operation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WatcherStats {
    /// Total events captured (after filtering)
    pub events_captured: i64,
    /// Events filtered out by ignore patterns
    pub events_filtered: i64,
    /// Events consolidated by debouncing
    pub events_debounced: i64,
    /// Events persisted to database
    pub events_persisted: i64,
}

// ============================================================================
// WATCHER CONFIG
// ============================================================================

/// Configuration for file watcher.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatcherConfig {
    /// Directory to watch
    pub watch_path: String,
    /// Custom ignore patterns (None = use defaults)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore_patterns: Option<Vec<String>>,
    /// Debounce window in milliseconds
    #[serde(default = "default_debounce_ms")]
    pub debounce_ms: u64,
    /// Whether to watch recursively
    #[serde(default = "default_true")]
    pub recursive: bool,
}

fn default_debounce_ms() -> u64 { 100 }
fn default_true() -> bool { true }

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            watch_path: ".".to_string(),
            ignore_patterns: None,
            debounce_ms: 100,
            recursive: true,
        }
    }
}

// ============================================================================
// FUNCTION SIGNATURES (for reference)
// ============================================================================

// Main functions to implement:
//
// EventFilter:
//   pub fn should_ignore(&self, path: &str) -> bool
//   pub fn get_relative_path(&self, path: &str) -> String
//
// EventDebouncer:
//   pub fn add(&self, event: FileEvent)
//   pub fn pending_count(&self) -> usize
//   pub fn flush(&self) -> Vec<FileEvent>
//   pub fn flush_all(&self) -> Vec<FileEvent>
//
// FileWatcher:
//   pub fn new(config: WatcherConfig, storage: &Storage) -> Result<Self>
//   pub fn start(&mut self) -> Result<()>
//   pub fn stop(&mut self) -> Result<WatcherStats>
//   pub fn is_running(&self) -> bool
//
// Database operations:
//   pub fn insert_file_event(storage: &Storage, event: &FileEvent) -> Result<()>
//   pub fn insert_file_events(storage: &Storage, events: &[FileEvent]) -> Result<i32>
//
// CLI command:
//   pub fn handle_watch_command(args: WatchArgs) -> Result<()>

// ============================================================================
// TESTS (verify type contracts)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_event_serialization() {
        let event = FileEvent {
            timestamp: Utc::now(),
            path: "src/main.rs".to_string(),
            event_type: event_types::WRITE.to_string(),
            size_bytes: Some(1234),
            old_path: None,
            is_directory: false,
            extension: Some(".rs".to_string()),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("src/main.rs"));
        assert!(json.contains("write"));
    }

    #[test]
    fn test_default_ignore_patterns_count() {
        // Python has 40+ patterns, ensure we have parity
        assert!(DEFAULT_IGNORE_PATTERNS.len() >= 40);
    }

    #[test]
    fn test_watcher_config_defaults() {
        let config = WatcherConfig::default();
        assert_eq!(config.debounce_ms, 100);
        assert!(config.recursive);
        assert!(config.ignore_patterns.is_none());
    }

    #[test]
    fn test_watcher_stats_default() {
        let stats = WatcherStats::default();
        assert_eq!(stats.events_captured, 0);
        assert_eq!(stats.events_filtered, 0);
    }
}
