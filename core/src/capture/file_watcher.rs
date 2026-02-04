//! File watcher for capturing file system events.
//!
//! Captures file creation, modification, deletion, and rename events
//! using the notify crate. Events are filtered to exclude noise
//! (.git, __pycache__, node_modules, etc.) and debounced to consolidate
//! rapid saves from IDEs.
//!
//! Python source: cli/src/context_os_events/capture/file_watcher.py (568 lines)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Instant;

// ============================================================================
// FILE EVENT (matches Python FileEvent dataclass, line 28-36)
// ============================================================================

/// A file system event.
///
/// Maps 1:1 to Python FileEvent dataclass.
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
            DEFAULT_IGNORE_PATTERNS
                .iter()
                .map(|s| s.to_string())
                .collect()
        });

        // Resolve to absolute path (handle non-existent paths gracefully)
        let watch_path_buf = PathBuf::from(watch_path);
        let resolved = watch_path_buf.canonicalize().unwrap_or(watch_path_buf);

        Self {
            watch_path: resolved,
            ignore_patterns: patterns,
        }
    }

    /// Check if a path should be ignored.
    ///
    /// Returns true if path matches any ignore pattern.
    /// Matches both full path and individual path components.
    pub fn should_ignore(&self, path: &str) -> bool {
        let relative = self.get_relative_path(path);
        // Normalize path separators to forward slashes for pattern matching
        let relative = relative.replace('\\', "/");

        for pattern in &self.ignore_patterns {
            // Check full relative path against pattern
            if Self::fnmatch(&relative, pattern) {
                return true;
            }

            // Check each path component against pattern
            for part in relative.split('/') {
                if !part.is_empty() && Self::fnmatch(part, pattern) {
                    return true;
                }
            }
        }

        false
    }

    /// Convert absolute path to relative path.
    pub fn get_relative_path(&self, path: &str) -> String {
        let path_buf = PathBuf::from(path);
        // Try to canonicalize, but fall back to original if it fails
        let path_buf = path_buf.canonicalize().unwrap_or(path_buf);

        if let Ok(relative) = path_buf.strip_prefix(&self.watch_path) {
            relative.to_string_lossy().replace('\\', "/")
        } else {
            // If can't strip prefix, return original with normalized separators
            path_buf.to_string_lossy().replace('\\', "/")
        }
    }

    /// Simple fnmatch-style pattern matching.
    fn fnmatch(name: &str, pattern: &str) -> bool {
        // Use glob crate for pattern matching
        glob::Pattern::new(pattern)
            .map(|p| p.matches(name))
            .unwrap_or(false)
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
    timestamps: Mutex<HashMap<String, Instant>>,
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
        let path = event.path.clone();
        let mut pending = self.pending.lock().unwrap();
        let mut timestamps = self.timestamps.lock().unwrap();

        pending.insert(path.clone(), event);
        timestamps.insert(path, Instant::now());
    }

    /// Get the number of pending events.
    pub fn pending_count(&self) -> usize {
        self.pending.lock().unwrap().len()
    }

    /// Flush events that have passed the debounce window.
    ///
    /// Returns list of events ready to be processed.
    pub fn flush(&self) -> Vec<FileEvent> {
        let threshold = std::time::Duration::from_millis(self.debounce_ms);
        let now = Instant::now();

        let mut pending = self.pending.lock().unwrap();
        let mut timestamps = self.timestamps.lock().unwrap();

        let mut flushed = Vec::new();
        let mut to_remove = Vec::new();

        for (path, timestamp) in timestamps.iter() {
            if now.duration_since(*timestamp) >= threshold {
                if let Some(event) = pending.get(path) {
                    flushed.push(event.clone());
                }
                to_remove.push(path.clone());
            }
        }

        for path in to_remove {
            pending.remove(&path);
            timestamps.remove(&path);
        }

        flushed
    }

    /// Flush all pending events regardless of time.
    pub fn flush_all(&self) -> Vec<FileEvent> {
        let mut pending = self.pending.lock().unwrap();
        let mut timestamps = self.timestamps.lock().unwrap();

        let flushed: Vec<FileEvent> = pending.values().cloned().collect();
        pending.clear();
        timestamps.clear();

        flushed
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

fn default_debounce_ms() -> u64 {
    100
}

fn default_true() -> bool {
    true
}

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
// CREATE EVENT HELPER
// ============================================================================

/// Create a FileEvent from a path.
pub fn create_event_from_path(
    path: &str,
    event_type: &str,
    watch_path: &str,
    old_path: Option<&str>,
) -> Option<FileEvent> {
    let filter = EventFilter::new(watch_path);
    let relative_path = filter.get_relative_path(path);

    let p = Path::new(path);
    let is_directory = p.is_dir();

    let size_bytes = if event_type == event_types::DELETE {
        None
    } else {
        p.metadata().ok().map(|m| m.len() as i64)
    };

    let extension = p.extension().map(|e| format!(".{}", e.to_string_lossy()));

    let relative_old_path = old_path.map(|op| filter.get_relative_path(op));

    Some(FileEvent {
        timestamp: Utc::now(),
        path: relative_path,
        event_type: event_type.to_string(),
        size_bytes,
        old_path: relative_old_path,
        is_directory,
        extension,
    })
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Cycle 1: EventFilter - Ignore Pattern Matching (6 tests)
    // ========================================================================

    #[test]
    fn test_filter_ignores_git_directory() {
        // Given: A filter for a repo
        let filter = EventFilter::new("/repo");

        // When/Then: .git paths should be ignored
        assert!(filter.should_ignore("/repo/.git/objects/abc"));
        assert!(filter.should_ignore("/repo/.git/HEAD"));
        assert!(filter.should_ignore("/repo/subdir/.git/config"));
    }

    #[test]
    fn test_filter_ignores_node_modules() {
        let filter = EventFilter::new("/repo");

        assert!(filter.should_ignore("/repo/node_modules/lodash/index.js"));
        assert!(filter.should_ignore("/repo/frontend/node_modules/react/index.js"));
    }

    #[test]
    fn test_filter_ignores_pycache() {
        let filter = EventFilter::new("/repo");

        assert!(filter.should_ignore("/repo/__pycache__/module.pyc"));
        assert!(filter.should_ignore("/repo/src/__pycache__/test.pyc"));
        assert!(filter.should_ignore("/repo/deep/nested/__pycache__/file.pyc"));
    }

    #[test]
    fn test_filter_ignores_by_extension() {
        let filter = EventFilter::new("/repo");

        // Python bytecode
        assert!(filter.should_ignore("/repo/file.pyc"));
        assert!(filter.should_ignore("/repo/file.pyo"));

        // Logs and temp
        assert!(filter.should_ignore("/repo/app.log"));
        assert!(filter.should_ignore("/repo/file.tmp"));
        assert!(filter.should_ignore("/repo/backup.bak"));

        // SQLite
        assert!(filter.should_ignore("/repo/data.db"));
        assert!(filter.should_ignore("/repo/cache.sqlite"));
    }

    #[test]
    fn test_filter_allows_normal_files() {
        let filter = EventFilter::new("/repo");

        // Source files - should NOT be ignored
        assert!(!filter.should_ignore("/repo/src/main.rs"));
        assert!(!filter.should_ignore("/repo/src/lib.py"));
        assert!(!filter.should_ignore("/repo/index.js"));

        // Config files
        assert!(!filter.should_ignore("/repo/package.json"));
        assert!(!filter.should_ignore("/repo/Cargo.toml"));

        // Docs
        assert!(!filter.should_ignore("/repo/README.md"));
    }

    #[test]
    fn test_filter_relative_path_extraction() {
        // Use a path that exists on any system for testing
        let tmp_dir = std::env::temp_dir();
        let watch_path = tmp_dir.to_string_lossy().to_string();
        let filter = EventFilter::new(&watch_path);

        // Create a test path
        let test_path = tmp_dir.join("src").join("main.rs");
        let test_path_str = test_path.to_string_lossy().to_string();

        // Since the path doesn't actually exist, it won't canonicalize
        // but we can at least verify the function works with real paths
        let relative = filter.get_relative_path(&test_path_str);
        // Should contain src/main.rs or src\main.rs (depending on OS)
        assert!(relative.contains("main.rs"));
    }

    // ========================================================================
    // Cycle 2: EventDebouncer (4 tests)
    // ========================================================================

    #[test]
    fn test_debouncer_add_and_count() {
        // Given: Empty debouncer
        let debouncer = EventDebouncer::new();

        // When: Add one event
        let event = create_test_event("src/main.rs", "write");
        debouncer.add(event);

        // Then: Count should be 1
        assert_eq!(debouncer.pending_count(), 1);
    }

    #[test]
    fn test_debouncer_replaces_same_path() {
        let debouncer = EventDebouncer::new();

        // When: Add two events for same path
        debouncer.add(create_test_event("src/main.rs", "write"));
        debouncer.add(create_test_event("src/main.rs", "write"));

        // Then: Only latest event kept (count = 1)
        assert_eq!(debouncer.pending_count(), 1);
    }

    #[test]
    fn test_debouncer_keeps_different_paths() {
        let debouncer = EventDebouncer::new();

        // When: Add events for different paths
        debouncer.add(create_test_event("src/main.rs", "write"));
        debouncer.add(create_test_event("src/lib.rs", "write"));
        debouncer.add(create_test_event("Cargo.toml", "write"));

        // Then: All events kept
        assert_eq!(debouncer.pending_count(), 3);
    }

    #[test]
    fn test_debouncer_flush_all_clears_buffer() {
        let debouncer = EventDebouncer::new();

        // Given: Buffer with 3 events
        debouncer.add(create_test_event("file1.rs", "create"));
        debouncer.add(create_test_event("file2.rs", "write"));
        debouncer.add(create_test_event("file3.rs", "delete"));

        // When: Flush all
        let flushed = debouncer.flush_all();

        // Then: All events returned, buffer empty
        assert_eq!(flushed.len(), 3);
        assert_eq!(debouncer.pending_count(), 0);
    }

    // ========================================================================
    // Cycle 3: FileEvent + Database (4 tests)
    // ========================================================================

    #[test]
    fn test_file_event_write_has_size() {
        let event = FileEvent {
            timestamp: Utc::now(),
            path: "src/main.rs".to_string(),
            event_type: event_types::WRITE.to_string(),
            size_bytes: Some(1234),
            old_path: None,
            is_directory: false,
            extension: Some(".rs".to_string()),
        };

        assert_eq!(event.event_type, "write");
        assert_eq!(event.size_bytes, Some(1234));
        assert!(!event.is_directory);
    }

    #[test]
    fn test_file_event_delete_has_no_size() {
        let event = FileEvent {
            timestamp: Utc::now(),
            path: "deleted.rs".to_string(),
            event_type: event_types::DELETE.to_string(),
            size_bytes: None,
            old_path: None,
            is_directory: false,
            extension: Some(".rs".to_string()),
        };

        assert!(event.size_bytes.is_none());
    }

    #[test]
    fn test_file_event_rename_has_old_path() {
        let event = FileEvent {
            timestamp: Utc::now(),
            path: "new_name.rs".to_string(),
            event_type: event_types::RENAME.to_string(),
            size_bytes: Some(500),
            old_path: Some("old_name.rs".to_string()),
            is_directory: false,
            extension: Some(".rs".to_string()),
        };

        assert_eq!(event.old_path, Some("old_name.rs".to_string()));
    }

    #[test]
    fn test_default_ignore_patterns_count() {
        // Python has 40+ patterns, ensure we have parity
        assert!(
            DEFAULT_IGNORE_PATTERNS.len() >= 40,
            "Expected >= 40 patterns, got {}",
            DEFAULT_IGNORE_PATTERNS.len()
        );
    }

    // ========================================================================
    // Cycle 4: FileWatcher Integration (4 tests)
    // ========================================================================

    #[test]
    fn test_watcher_config_defaults() {
        let config = WatcherConfig::default();

        assert_eq!(config.debounce_ms, 100);
        assert!(config.recursive);
        assert!(config.ignore_patterns.is_none());
    }

    #[test]
    fn test_watcher_stats_initial_zeroes() {
        let stats = WatcherStats::default();

        assert_eq!(stats.events_captured, 0);
        assert_eq!(stats.events_filtered, 0);
        assert_eq!(stats.events_debounced, 0);
        assert_eq!(stats.events_persisted, 0);
    }

    #[test]
    fn test_create_event_from_existing_file() {
        // Given: A real file
        let tmp_dir = tempfile::TempDir::new().unwrap();
        let test_file = tmp_dir.path().join("test.rs");
        std::fs::write(&test_file, "fn main() {}").unwrap();

        let watch_path = tmp_dir.path().to_string_lossy().to_string();

        // When: Create event
        let event = create_event_from_path(
            &test_file.to_string_lossy(),
            event_types::CREATE,
            &watch_path,
            None,
        );

        // Then: Event has correct fields
        assert!(event.is_some());
        let event = event.unwrap();
        assert_eq!(event.event_type, "create");
        assert_eq!(event.extension, Some(".rs".to_string()));
        assert!(event.size_bytes.is_some());
        assert!(!event.is_directory);
    }

    #[test]
    fn test_create_event_for_directory() {
        // Given: A directory
        let tmp_dir = tempfile::TempDir::new().unwrap();
        let sub_dir = tmp_dir.path().join("subdir");
        std::fs::create_dir(&sub_dir).unwrap();

        let watch_path = tmp_dir.path().to_string_lossy().to_string();

        // When: Create event
        let event = create_event_from_path(
            &sub_dir.to_string_lossy(),
            event_types::CREATE,
            &watch_path,
            None,
        );

        // Then: is_directory = true
        assert!(event.is_some());
        let event = event.unwrap();
        assert!(event.is_directory);
    }

    // ========================================================================
    // Database Persistence Test
    // ========================================================================

    #[tokio::test]
    async fn test_insert_file_event_to_database() {
        use crate::query::QueryEngine;
        use crate::storage::Database;

        // Create temp database with file_events schema
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute(
            "CREATE TABLE file_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                path TEXT NOT NULL,
                event_type TEXT NOT NULL,
                size_bytes INTEGER,
                old_path TEXT,
                is_directory BOOLEAN NOT NULL,
                extension TEXT
            )",
            [],
        )
        .unwrap();
        drop(conn);

        // Open database with QueryEngine
        let db = Database::open_rw(&db_path).await.unwrap();
        let engine = QueryEngine::new(db);

        // Create test event
        let event = FileEvent {
            timestamp: Utc::now(),
            path: "test.rs".to_string(),
            event_type: "create".to_string(),
            size_bytes: Some(100),
            old_path: None,
            is_directory: false,
            extension: Some(".rs".to_string()),
        };

        // Insert and verify
        let result = engine.insert_file_event(&event).await;
        assert!(result.is_ok(), "Insert should succeed");
        assert_eq!(result.unwrap().rows_affected, 1);
    }

    // ========================================================================
    // Test Helper
    // ========================================================================

    fn create_test_event(path: &str, event_type: &str) -> FileEvent {
        FileEvent {
            timestamp: Utc::now(),
            path: path.to_string(),
            event_type: event_type.to_string(),
            size_bytes: Some(100),
            old_path: None,
            is_directory: false,
            extension: path.rsplit('.').next().map(|e| format!(".{}", e)),
        }
    }
}
