# Phase 7: File Watcher - Test-Driven Plan

**Philosophy:** "Write the test first. Always." - Kent Beck

**Methodology:** Red-Green-Refactor for each cycle
**Total Tests:** 18 (4 cycles)
**Estimated Time:** 5-6 hours

---

## TDD Cycle Overview

| Cycle | Component | Tests | Time | Focus |
|-------|-----------|-------|------|-------|
| 1 | EventFilter | 6 | 1 hour | Ignore pattern matching |
| 2 | EventDebouncer | 4 | 45 min | Event consolidation |
| 3 | FileEvent + DB | 4 | 45 min | Types + persistence |
| 4 | FileWatcher | 4 | 1.5 hours | Integration + CLI |

---

## Cycle 1: EventFilter - Ignore Pattern Matching

**Goal:** Filter out noise files (.git, node_modules, __pycache__, etc.)

### RED - Write Failing Tests First

```rust
// core/src/capture/file_watcher.rs

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Cycle 1: EventFilter (6 tests)
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

        // Source files
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
        let filter = EventFilter::new("/repo");

        assert_eq!(
            filter.get_relative_path("/repo/src/main.rs"),
            "src/main.rs"
        );
        assert_eq!(
            filter.get_relative_path("/repo/README.md"),
            "README.md"
        );
        assert_eq!(
            filter.get_relative_path("/repo/deep/nested/file.txt"),
            "deep/nested/file.txt"
        );
    }
}
```

### Run Tests (Expect FAIL)

```bash
cargo test filter --lib
# Expected: 6 tests, all FAIL (functions not implemented)
```

### GREEN - Minimal Implementation

Implement `EventFilter` with:
- `DEFAULT_IGNORE_PATTERNS` constant (40+ patterns)
- `new()` and `with_patterns()` constructors
- `should_ignore()` with glob pattern matching
- `get_relative_path()` for path normalization

### REFACTOR

- Extract pattern matching to separate function
- Add Windows path separator handling

### Commit

```bash
git add src/capture/file_watcher.rs
git commit -m "feat(file_watcher): add EventFilter with ignore patterns - TDD Cycle 1

- 40+ default ignore patterns (parity with Python)
- Pattern matching for .git, node_modules, __pycache__, etc.
- Relative path extraction

Tests: 6 passing"
```

---

## Cycle 2: EventDebouncer - Event Consolidation

**Goal:** Consolidate rapid saves into single events

### RED - Write Failing Tests First

```rust
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

    // Test helper
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
```

### Run Tests (Expect FAIL)

```bash
cargo test debouncer --lib
# Expected: 4 tests, all FAIL
```

### GREEN - Minimal Implementation

Implement `EventDebouncer` with:
- `Mutex<HashMap<String, FileEvent>>` for pending events
- `Mutex<HashMap<String, Instant>>` for timestamps
- `add()`, `pending_count()`, `flush()`, `flush_all()`

### Commit

```bash
git commit -m "feat(file_watcher): add EventDebouncer - TDD Cycle 2

- 100ms default debounce window
- Thread-safe with Mutex
- Replaces events for same path

Tests: 10 passing (6 + 4)"
```

---

## Cycle 3: FileEvent + Database Operations

**Goal:** Create events and persist to SQLite

### RED - Write Failing Tests First

```rust
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

    #[tokio::test]
    async fn test_insert_file_event_to_database() {
        // Given: Fresh database
        let tmp_dir = tempfile::TempDir::new().unwrap();
        let db_path = tmp_dir.path().join("test.db");
        let storage = Storage::open_rw(&db_path).await.unwrap();

        // When: Insert event
        let event = FileEvent {
            timestamp: Utc::now(),
            path: "test.rs".to_string(),
            event_type: "create".to_string(),
            size_bytes: Some(100),
            old_path: None,
            is_directory: false,
            extension: Some(".rs".to_string()),
        };

        let result = storage.insert_file_event(&event).await;

        // Then: Success
        assert!(result.is_ok());
    }
```

### Run Tests (Expect FAIL)

```bash
cargo test file_event --lib
# Expected: 4 tests, 3 pass (pure struct tests), 1 fails (DB not implemented)
```

### GREEN - Implement Database Operations

Add to `storage.rs`:
- `insert_file_event()`
- `insert_file_events()` (batch)
- Ensure `file_events` table exists in schema

### Commit

```bash
git commit -m "feat(file_watcher): add FileEvent types and DB persistence - TDD Cycle 3

- FileEvent struct with all fields
- event_types constants
- insert_file_event() and insert_file_events()

Tests: 14 passing (10 + 4)"
```

---

## Cycle 4: FileWatcher Integration

**Goal:** End-to-end watcher with notify crate

### RED - Write Failing Tests First

```rust
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
            None
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
            None
        );

        // Then: is_directory = true
        assert!(event.is_some());
        let event = event.unwrap();
        assert!(event.is_directory);
    }
```

### Run Tests (Expect FAIL)

```bash
cargo test watcher --lib
cargo test create_event --lib
# Expected: 4 tests fail
```

### GREEN - Implement FileWatcher

1. Add `notify = "6.1"` to Cargo.toml
2. Implement `WatcherConfig` and `WatcherStats` structs
3. Implement `create_event_from_path()` helper
4. Implement `FileWatcher` with notify integration

### Add CLI Command

```rust
// In main.rs Commands enum
Watch(WatchArgs),

// WatchArgs struct
#[derive(Debug, Clone, clap::Args)]
pub struct WatchArgs {
    #[arg(long, default_value = ".")]
    pub path: String,

    #[arg(long, default_value = "100")]
    pub debounce_ms: u64,

    #[arg(long)]
    pub duration: Option<u64>,
}
```

### Commit

```bash
git commit -m "feat(file_watcher): add FileWatcher with notify integration - TDD Cycle 4

- FileWatcher struct with notify crate
- WatcherConfig and WatcherStats
- create_event_from_path helper
- CLI watch command

Tests: 18 passing (14 + 4)"
```

---

## Final Verification

```bash
# Run all tests
cd apps/tastematter/core
cargo test --lib
# Expected: 148+ tests (130 existing + 18 new)

# Run file_watcher tests specifically
cargo test file_watcher --lib
# Expected: 18 tests passing

# Build and test CLI
cargo build --release
./target/release/context-os watch --path "." --duration 5

# Test with actual file changes
# 1. Start watcher: ./target/release/context-os watch --path "/tmp/test"
# 2. In another terminal: touch /tmp/test/file.txt
# 3. Verify event is captured
```

---

## Test Summary

| Cycle | Tests | Description |
|-------|-------|-------------|
| 1 | 6 | EventFilter - 40+ ignore patterns |
| 2 | 4 | EventDebouncer - consolidation |
| 3 | 4 | FileEvent + DB persistence |
| 4 | 4 | FileWatcher + CLI integration |
| **Total** | **18** | |

---

## Kent Beck Reminders

1. **"Never trust a test you haven't seen fail."**
   - Run tests BEFORE implementing. Confirm they fail.

2. **"Make it work, make it right, make it fast."**
   - First pass: Just make tests green
   - Second pass: Refactor for clarity
   - Third pass: Optimize if needed

3. **"The tests ARE the specification."**
   - Tests document expected behavior
   - Anyone can read tests to understand the system

4. **"If you can't write the test, you don't understand the problem."**
   - Difficulty writing tests = design problem
   - Step back and clarify requirements
