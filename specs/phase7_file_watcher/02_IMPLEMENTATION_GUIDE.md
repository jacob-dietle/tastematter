# Phase 7: File Watcher - Implementation Guide

**Purpose:** Step-by-step implementation guide for porting file_watcher.py to Rust
**Reading Time:** 15 minutes
**Implementation Time:** 5-6 hours
**Prerequisites:** Phases 0-6 complete, 130 Rust tests passing

---

## Before You Start

### Step 1: Verify Baseline (5 min)

```bash
# Confirm current state
cd apps/tastematter/core
cargo test --lib
# Expected: 130 tests passing

# Build release
cargo build --release
# Expected: Success
```

### Step 2: Add Dependencies (2 min)

Edit `core/Cargo.toml`:

```toml
[dependencies]
# ... existing deps ...
notify = "6.1"                    # File system notifications
glob = "0.3"                      # Already exists, used for pattern matching
```

**Note:** We'll implement custom debouncing to match Python behavior exactly, rather than using `notify-debouncer-mini`.

### Step 3: Read Reference Files (10 min)

1. [[specs/phase7_file_watcher/00_ARCHITECTURE_GUIDE.md]] - Architecture overview
2. [[specs/phase7_file_watcher/01_TYPE_CONTRACTS.rs]] - Type definitions
3. [[cli/src/context_os_events/capture/file_watcher.py]] - Python implementation

---

## Implementation Steps (TDD Order)

Follow Kent Beck's Red-Green-Refactor for each cycle.

### Cycle 1: EventFilter - Ignore Pattern Matching (1 hour)

**Create file:** `core/src/capture/file_watcher.rs`

**RED - Write failing tests first:**

```rust
// core/src/capture/file_watcher.rs

//! File watcher for capturing file system events.
//!
//! Captures file creation, modification, deletion, and rename events
//! using the notify crate. Events are filtered to exclude noise
//! (.git, __pycache__, node_modules, etc.) and debounced to consolidate
//! rapid saves from IDEs.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

// ... type definitions from 01_TYPE_CONTRACTS.rs ...

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Cycle 1: EventFilter - Ignore Pattern Matching (6 tests)
    // ========================================================================

    #[test]
    fn test_filter_ignores_git_directory() {
        let filter = EventFilter::new("/repo");
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
    }

    #[test]
    fn test_filter_ignores_by_extension() {
        let filter = EventFilter::new("/repo");
        assert!(filter.should_ignore("/repo/file.pyc"));
        assert!(filter.should_ignore("/repo/file.log"));
        assert!(filter.should_ignore("/repo/data.db"));
        assert!(filter.should_ignore("/repo/backup.bak"));
    }

    #[test]
    fn test_filter_allows_normal_files() {
        let filter = EventFilter::new("/repo");
        assert!(!filter.should_ignore("/repo/src/main.rs"));
        assert!(!filter.should_ignore("/repo/README.md"));
        assert!(!filter.should_ignore("/repo/package.json"));
    }

    #[test]
    fn test_filter_relative_path_extraction() {
        let filter = EventFilter::new("/repo");
        assert_eq!(filter.get_relative_path("/repo/src/main.rs"), "src/main.rs");
        assert_eq!(filter.get_relative_path("/repo/README.md"), "README.md");
    }
}
```

**Run tests:** `cargo test filter --lib` → Should FAIL

**GREEN - Implement EventFilter:**

```rust
/// Default ignore patterns for file watching.
pub const DEFAULT_IGNORE_PATTERNS: &[&str] = &[
    // ... 40+ patterns from type contracts ...
];

/// Filters file events based on ignore patterns.
pub struct EventFilter {
    pub watch_path: PathBuf,
    pub ignore_patterns: Vec<String>,
}

impl EventFilter {
    pub fn new(watch_path: &str) -> Self {
        Self::with_patterns(watch_path, None)
    }

    pub fn with_patterns(watch_path: &str, patterns: Option<Vec<String>>) -> Self {
        let patterns = patterns.unwrap_or_else(|| {
            DEFAULT_IGNORE_PATTERNS.iter().map(|s| s.to_string()).collect()
        });

        // Resolve to absolute path
        let watch_path = PathBuf::from(watch_path);
        let watch_path = watch_path.canonicalize().unwrap_or(watch_path);

        Self { watch_path, ignore_patterns: patterns }
    }

    pub fn should_ignore(&self, path: &str) -> bool {
        let relative = self.get_relative_path(path);
        let relative = relative.replace('\\', "/");  // Normalize

        for pattern in &self.ignore_patterns {
            // Check full path match
            if Self::fnmatch(&relative, pattern) {
                return true;
            }

            // Check each path component
            for part in relative.split('/') {
                if Self::fnmatch(part, pattern) {
                    return true;
                }
            }
        }

        false
    }

    pub fn get_relative_path(&self, path: &str) -> String {
        let path = PathBuf::from(path);
        let path = path.canonicalize().unwrap_or(path);

        if let Ok(relative) = path.strip_prefix(&self.watch_path) {
            relative.to_string_lossy().to_string()
        } else {
            path.to_string_lossy().to_string()
        }
    }

    /// Simple fnmatch implementation.
    fn fnmatch(name: &str, pattern: &str) -> bool {
        // Use glob crate for pattern matching
        glob::Pattern::new(pattern)
            .map(|p| p.matches(name))
            .unwrap_or(false)
    }
}
```

**Run tests:** `cargo test filter --lib` → Should PASS

---

### Cycle 2: EventDebouncer (45 min)

**RED - Write failing tests:**

```rust
    // ========================================================================
    // Cycle 2: EventDebouncer (4 tests)
    // ========================================================================

    #[test]
    fn test_debouncer_add_and_count() {
        let debouncer = EventDebouncer::new();
        let event = create_test_event("src/main.rs", "write");

        debouncer.add(event);
        assert_eq!(debouncer.pending_count(), 1);
    }

    #[test]
    fn test_debouncer_replaces_same_path() {
        let debouncer = EventDebouncer::new();

        let event1 = create_test_event("src/main.rs", "write");
        let event2 = create_test_event("src/main.rs", "write");

        debouncer.add(event1);
        debouncer.add(event2);

        assert_eq!(debouncer.pending_count(), 1);  // Only 1, not 2
    }

    #[test]
    fn test_debouncer_keeps_different_paths() {
        let debouncer = EventDebouncer::new();

        debouncer.add(create_test_event("src/main.rs", "write"));
        debouncer.add(create_test_event("src/lib.rs", "write"));

        assert_eq!(debouncer.pending_count(), 2);
    }

    #[test]
    fn test_debouncer_flush_all() {
        let debouncer = EventDebouncer::new();

        debouncer.add(create_test_event("file1.rs", "create"));
        debouncer.add(create_test_event("file2.rs", "write"));
        debouncer.add(create_test_event("file3.rs", "delete"));

        let flushed = debouncer.flush_all();
        assert_eq!(flushed.len(), 3);
        assert_eq!(debouncer.pending_count(), 0);
    }

    // Test helper
    fn create_test_event(path: &str, event_type: &str) -> FileEvent {
        FileEvent {
            timestamp: Utc::now(),
            path: path.to_string(),
            event_type: event_type.to_string(),
            size_bytes: Some(100),
            old_path: None,
            is_directory: false,
            extension: Some(".rs".to_string()),
        }
    }
```

**GREEN - Implement EventDebouncer:**

```rust
use std::time::Instant;

/// Consolidates rapid events on the same file.
pub struct EventDebouncer {
    pub debounce_ms: u64,
    pending: Mutex<HashMap<String, FileEvent>>,
    timestamps: Mutex<HashMap<String, Instant>>,
}

impl EventDebouncer {
    pub fn new() -> Self {
        Self::with_debounce(100)
    }

    pub fn with_debounce(debounce_ms: u64) -> Self {
        Self {
            debounce_ms,
            pending: Mutex::new(HashMap::new()),
            timestamps: Mutex::new(HashMap::new()),
        }
    }

    pub fn add(&self, event: FileEvent) {
        let path = event.path.clone();
        let mut pending = self.pending.lock().unwrap();
        let mut timestamps = self.timestamps.lock().unwrap();

        pending.insert(path.clone(), event);
        timestamps.insert(path, Instant::now());
    }

    pub fn pending_count(&self) -> usize {
        self.pending.lock().unwrap().len()
    }

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
```

**Run tests:** `cargo test debouncer --lib` → Should PASS

---

### Cycle 3: FileEvent + Database Operations (45 min)

**RED - Write failing tests:**

```rust
    // ========================================================================
    // Cycle 3: FileEvent + Database (4 tests)
    // ========================================================================

    #[test]
    fn test_file_event_creation() {
        let event = FileEvent {
            timestamp: Utc::now(),
            path: "src/main.rs".to_string(),
            event_type: event_types::WRITE.to_string(),
            size_bytes: Some(1234),
            old_path: None,
            is_directory: false,
            extension: Some(".rs".to_string()),
        };

        assert_eq!(event.path, "src/main.rs");
        assert_eq!(event.event_type, "write");
        assert!(!event.is_directory);
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
    fn test_file_event_delete_has_no_size() {
        let event = FileEvent {
            timestamp: Utc::now(),
            path: "deleted.rs".to_string(),
            event_type: event_types::DELETE.to_string(),
            size_bytes: None,  // Deleted files have no size
            old_path: None,
            is_directory: false,
            extension: Some(".rs".to_string()),
        };

        assert!(event.size_bytes.is_none());
    }

    #[tokio::test]
    async fn test_insert_file_event() {
        let tmp_dir = tempfile::TempDir::new().unwrap();
        let db_path = tmp_dir.path().join("test.db");

        let storage = Storage::open_rw(&db_path).await.unwrap();

        let event = FileEvent {
            timestamp: Utc::now(),
            path: "test.rs".to_string(),
            event_type: "create".to_string(),
            size_bytes: Some(100),
            old_path: None,
            is_directory: false,
            extension: Some(".rs".to_string()),
        };

        let result = insert_file_event(&storage, &event).await;
        assert!(result.is_ok());
    }
```

**GREEN - Implement database operations:**

Add to `storage.rs`:

```rust
/// Insert a file event into the database.
pub async fn insert_file_event(&self, event: &FileEvent) -> Result<(), StorageError> {
    sqlx::query(r#"
        INSERT INTO file_events (
            timestamp, path, event_type, size_bytes,
            old_path, is_directory, extension
        ) VALUES (?, ?, ?, ?, ?, ?, ?)
    "#)
    .bind(event.timestamp.to_rfc3339())
    .bind(&event.path)
    .bind(&event.event_type)
    .bind(event.size_bytes)
    .bind(&event.old_path)
    .bind(event.is_directory)
    .bind(&event.extension)
    .execute(&self.pool)
    .await?;

    Ok(())
}

/// Insert multiple file events.
pub async fn insert_file_events(&self, events: &[FileEvent]) -> Result<i32, StorageError> {
    let mut count = 0;
    for event in events {
        self.insert_file_event(event).await?;
        count += 1;
    }
    Ok(count)
}
```

---

### Cycle 4: FileWatcher Integration (1.5 hours)

**RED - Write integration tests:**

```rust
    // ========================================================================
    // Cycle 4: FileWatcher Integration (4 tests)
    // ========================================================================

    #[test]
    fn test_watcher_config_defaults() {
        let config = WatcherConfig::default();
        assert_eq!(config.debounce_ms, 100);
        assert!(config.recursive);
    }

    #[test]
    fn test_watcher_stats_initial() {
        let stats = WatcherStats::default();
        assert_eq!(stats.events_captured, 0);
        assert_eq!(stats.events_filtered, 0);
        assert_eq!(stats.events_persisted, 0);
    }

    #[tokio::test]
    async fn test_watcher_creates_and_stops() {
        let tmp_dir = tempfile::TempDir::new().unwrap();
        let db_path = tmp_dir.path().join("test.db");
        let storage = Storage::open_rw(&db_path).await.unwrap();

        let config = WatcherConfig {
            watch_path: tmp_dir.path().to_string_lossy().to_string(),
            ..Default::default()
        };

        // Just test that watcher can be created
        // Full integration test would require async file operations
        assert!(config.watch_path.len() > 0);
    }

    #[test]
    fn test_create_event_from_path() {
        let tmp_dir = tempfile::TempDir::new().unwrap();
        let watch_path = tmp_dir.path().to_string_lossy().to_string();

        // Create a test file
        let test_file = tmp_dir.path().join("test.rs");
        std::fs::write(&test_file, "fn main() {}").unwrap();

        let event = create_event_from_path(
            &test_file.to_string_lossy(),
            event_types::CREATE,
            &watch_path,
            None
        );

        assert!(event.is_some());
        let event = event.unwrap();
        assert_eq!(event.event_type, "create");
        assert_eq!(event.extension, Some(".rs".to_string()));
        assert!(!event.is_directory);
    }
```

**GREEN - Implement FileWatcher with notify crate:**

```rust
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::sync::mpsc::{channel, Receiver};
use std::thread;
use std::time::Duration;

/// Main file watcher orchestrator.
pub struct FileWatcher {
    config: WatcherConfig,
    filter: EventFilter,
    debouncer: EventDebouncer,
    stats: Mutex<WatcherStats>,
    running: Mutex<bool>,
}

impl FileWatcher {
    pub fn new(config: WatcherConfig) -> Self {
        let filter = EventFilter::with_patterns(
            &config.watch_path,
            config.ignore_patterns.clone()
        );
        let debouncer = EventDebouncer::with_debounce(config.debounce_ms);

        Self {
            config,
            filter,
            debouncer,
            stats: Mutex::new(WatcherStats::default()),
            running: Mutex::new(false),
        }
    }

    // ... implementation continues ...
}

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

    let extension = p.extension()
        .map(|e| format!(".{}", e.to_string_lossy()));

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
```

---

### Cycle 5: CLI Command (30 min)

Add to `main.rs`:

```rust
/// Watch a directory for file changes
#[derive(Debug, Clone, clap::Args)]
pub struct WatchArgs {
    /// Directory to watch
    #[arg(long, default_value = ".")]
    pub path: String,

    /// Debounce window in milliseconds
    #[arg(long, default_value = "100")]
    pub debounce_ms: u64,

    /// Run for a maximum duration (seconds), then exit
    #[arg(long)]
    pub duration: Option<u64>,
}

// In match handler:
Commands::Watch(args) => {
    eprintln!("Watching {} for file changes...", args.path);
    eprintln!("Press Ctrl+C to stop");

    let config = WatcherConfig {
        watch_path: args.path.clone(),
        debounce_ms: args.debounce_ms,
        ..Default::default()
    };

    // ... start watcher ...
}
```

---

## Verification Commands

After implementation:

```bash
# Run all tests
cd apps/tastematter/core && cargo test --lib
# Expected: 148+ tests (130 existing + 18 new)

# Test specific module
cargo test file_watcher --lib

# Build release
cargo build --release

# Test CLI command
./target/release/context-os watch --path "." --duration 5
```

---

## Common Pitfalls

1. **Path separator normalization** - Windows uses `\`, Unix uses `/`. Normalize to `/` for pattern matching.

2. **Canonicalization failures** - `canonicalize()` fails for non-existent paths. Use `unwrap_or` fallback.

3. **Mutex deadlocks** - Don't hold multiple locks simultaneously. Release first lock before acquiring second.

4. **Thread safety** - `notify` events arrive on a background thread. Use channels or `Arc<Mutex<>>`.

5. **Graceful shutdown** - Always flush remaining events before exiting.

---

## Success Criteria Checklist

- [ ] 18+ new tests passing
- [ ] 40+ ignore patterns working
- [ ] Debouncing consolidates rapid events
- [ ] Events persist to database
- [ ] CLI `watch` command works
- [ ] <5% CPU during idle watching
- [ ] Graceful shutdown with final flush
