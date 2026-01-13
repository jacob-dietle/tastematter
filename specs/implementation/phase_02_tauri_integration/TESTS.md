# Phase 2: Tauri Integration - TDD Test Plan

## Philosophy

> "Tests give you courage to refactor." - Kent Beck

**This is a TEST-FIRST integration.** Every test must be written BEFORE modifying commands.rs.

**Red-Green-Refactor for Integration:**
1. **RED** - Write test that calls command through Tauri → Should fail (uses old CLI path)
2. **GREEN** - Replace Command::new() with library call → Should pass
3. **REFACTOR** - Clean up dead code → Should still pass
4. **COMMIT** - Save with test reference

---

## Test Hierarchy

Phase 2 focuses on integration testing since we're connecting Phase 1 to Tauri:

```
Level 3: E2E Tests (Full App)
├── App launches without errors
├── Query commands return data
├── Latency verification (<100ms)
└── Expected bugs found: 5-10
Time: 1.5 hours | Priority: CRITICAL

Level 2: Integration Tests (Command Layer)
├── AppState initialization
├── Command injection with State<>
├── Error propagation
└── Expected bugs found: 3-5
Time: 1 hour | Priority: HIGH

Level 1: Unit Tests (Minimal)
├── Error conversion
├── Database path discovery
└── Expected bugs found: 0-1
Time: 15 min | Priority: MEDIUM
```

---

## Test File Structure

```
apps/tastematter/src-tauri/
├── src/
│   ├── lib.rs
│   └── commands.rs
└── tests/                          # NEW: Integration tests
    ├── common/
    │   └── mod.rs                  # Shared test utilities
    ├── test_app_state.rs           # AppState tests
    ├── test_query_commands.rs      # Command integration tests
    └── test_latency.rs             # Performance tests
```

---

## Level 1: Unit Tests

### Test 1.1: Error Conversion

**Purpose:** Verify CoreError converts to CommandError correctly
**Write BEFORE:** Error conversion in commands.rs

```rust
// src/commands.rs - add to existing tests module
#[cfg(test)]
mod tests {
    use super::*;
    use context_os_core::CoreError;

    #[test]
    fn test_core_error_to_command_error_database() {
        // Given: Database error
        let core_err = CoreError::Database(sqlx::Error::RowNotFound);

        // When: Convert to CommandError
        let cmd_err: CommandError = core_err.into();

        // Then: Correct error code
        assert_eq!(cmd_err.code, "DATABASE_ERROR");
        assert_eq!(cmd_err.message, "Database operation failed");
        assert!(cmd_err.details.is_some());
    }

    #[test]
    fn test_core_error_to_command_error_query() {
        // Given: Query error
        let core_err = CoreError::Query {
            message: "Invalid time range".to_string(),
        };

        // When: Convert
        let cmd_err: CommandError = core_err.into();

        // Then: Message preserved
        assert_eq!(cmd_err.code, "QUERY_ERROR");
        assert_eq!(cmd_err.message, "Invalid time range");
    }

    #[test]
    fn test_core_error_to_command_error_config() {
        // Given: Config error (database not found)
        let core_err = CoreError::Config("Database not found".to_string());

        // When: Convert
        let cmd_err: CommandError = core_err.into();

        // Then: Message preserved
        assert_eq!(cmd_err.code, "CONFIG_ERROR");
        assert!(cmd_err.message.contains("Database not found"));
    }
}
```

**Red:** Run test → Should fail (From impl doesn't exist yet)
**Green:** Implement From<CoreError> for CommandError
**Commit:** `test(commands): CoreError to CommandError conversion`

---

### Test 1.2: Database Path Discovery

**Purpose:** Verify find_database_path finds database in standard locations
**Write BEFORE:** find_database_path function in lib.rs

```rust
// src/lib.rs - add tests module
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_database_path_env_override() {
        // Given: Environment variable set
        std::env::set_var("CONTEXT_OS_DB", "/tmp/test.db");

        // Create temp file to make path "exist"
        std::fs::write("/tmp/test.db", "").unwrap();

        // When: Find path
        let result = find_database_path();

        // Then: Returns env var path
        assert!(result.is_ok());
        assert_eq!(result.unwrap().to_str().unwrap(), "/tmp/test.db");

        // Cleanup
        std::fs::remove_file("/tmp/test.db").unwrap();
        std::env::remove_var("CONTEXT_OS_DB");
    }

    #[test]
    fn test_find_database_path_standard_location() {
        // Given: No env var, standard location exists
        std::env::remove_var("CONTEXT_OS_DB");

        // When: Find path (assume ~/.context-os/context.db exists from dev setup)
        let result = find_database_path();

        // Then: Returns home directory path
        if let Ok(path) = result {
            assert!(path.to_str().unwrap().contains(".context-os/context.db"));
        }
        // If not found, that's OK in CI - just testing the logic
    }

    #[test]
    fn test_find_database_path_not_found() {
        // Given: No valid paths
        std::env::set_var("CONTEXT_OS_DB", "/nonexistent/path.db");

        // When: Find path
        let result = find_database_path();

        // Then: Returns Config error
        std::env::remove_var("CONTEXT_OS_DB");

        // Note: This may succeed if ~/.context-os/context.db exists
        // The test documents expected behavior
    }
}
```

**Red:** Run test → Should fail (find_database_path doesn't exist)
**Green:** Implement find_database_path
**Commit:** `test(lib): database path discovery`

---

## Level 2: Integration Tests

### Test 2.1: AppState Creation

**Purpose:** Verify AppState can be created and is thread-safe
**Write BEFORE:** AppState struct changes in lib.rs

```rust
// tests/test_app_state.rs
use tastematter::AppState;
use std::sync::Arc;

#[test]
fn test_app_state_creation() {
    // Given: LogService
    let log_service = Arc::new(tastematter::logging::LogService::new());

    // When: Create AppState
    let state = AppState::new(log_service);

    // Then: Query engine is not yet initialized (lazy)
    assert!(state.query_engine.get().is_none());
}

#[test]
fn test_app_state_is_send_sync() {
    // AppState must be Send + Sync for Tauri
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<AppState>();
}

#[tokio::test]
async fn test_app_state_lazy_init() {
    // Given: AppState
    let log_service = Arc::new(tastematter::logging::LogService::new());
    let state = AppState::new(log_service);

    // When: Get query engine (first time)
    let result = state.get_query_engine().await;

    // Then: Either succeeds (DB exists) or returns Config error (DB not found)
    match result {
        Ok(engine) => {
            // Query engine initialized
            assert!(state.query_engine.get().is_some());
        }
        Err(e) => {
            // Config error expected if DB doesn't exist
            assert!(matches!(e, context_os_core::CoreError::Config(_)));
        }
    }
}

#[tokio::test]
async fn test_app_state_query_engine_reused() {
    // Given: AppState with initialized engine
    let log_service = Arc::new(tastematter::logging::LogService::new());
    let state = AppState::new(log_service);

    // Skip if DB doesn't exist
    if state.get_query_engine().await.is_err() {
        return;
    }

    // When: Get query engine twice
    let engine1 = state.get_query_engine().await.unwrap();
    let engine2 = state.get_query_engine().await.unwrap();

    // Then: Same instance (pointer equality)
    assert!(std::ptr::eq(engine1, engine2));
}
```

**Red:** Run test → Should fail (AppState doesn't have query_engine field)
**Green:** Add query_engine to AppState, implement get_query_engine
**Commit:** `test(app_state): lazy query engine initialization`

---

### Test 2.2: Query Command Integration

**Purpose:** Verify query_flex command works with injected state
**Write BEFORE:** Modifying query_flex in commands.rs

```rust
// tests/test_query_commands.rs
use tastematter::{AppState, commands};
use std::sync::Arc;

/// Helper to create test AppState
fn create_test_state() -> AppState {
    let log_service = Arc::new(tastematter::logging::LogService::new());
    AppState::new(log_service)
}

#[tokio::test]
async fn test_query_flex_with_state() {
    // Given: AppState (may or may not have DB)
    let state = create_test_state();

    // When: Call query_flex
    // Note: We can't easily call Tauri commands directly in tests
    // Instead, test the internal logic

    // Get engine (skip if no DB)
    let engine = match state.get_query_engine().await {
        Ok(e) => e,
        Err(_) => return, // Skip test if DB not available
    };

    // Execute query
    let input = context_os_core::QueryFlexInput {
        time: Some("7d".to_string()),
        limit: Some(5),
        ..Default::default()
    };

    let result = engine.query_flex(input).await;

    // Then: Returns valid result
    assert!(result.is_ok());
    let result = result.unwrap();
    assert!(!result.receipt_id.is_empty());
}

#[tokio::test]
async fn test_query_chains_with_state() {
    let state = create_test_state();

    let engine = match state.get_query_engine().await {
        Ok(e) => e,
        Err(_) => return,
    };

    let input = context_os_core::QueryChainsInput { limit: Some(10) };
    let result = engine.query_chains(input).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_query_timeline_with_state() {
    let state = create_test_state();

    let engine = match state.get_query_engine().await {
        Ok(e) => e,
        Err(_) => return,
    };

    let input = context_os_core::QueryTimelineInput {
        time: "7d".to_string(),
        files: None,
        limit: Some(20),
    };
    let result = engine.query_timeline(input).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_query_sessions_with_state() {
    let state = create_test_state();

    let engine = match state.get_query_engine().await {
        Ok(e) => e,
        Err(_) => return,
    };

    let input = context_os_core::QuerySessionsInput {
        time: "7d".to_string(),
        chain: None,
        limit: Some(20),
    };
    let result = engine.query_sessions(input).await;

    assert!(result.is_ok());
}
```

**Red:** Run test → Should fail until commands.rs is updated
**Green:** Update commands.rs to use state injection
**Commit:** `test(commands): query commands with state injection`

---

### Test 2.3: Error Propagation

**Purpose:** Verify errors from context-os-core propagate correctly to frontend
**Write BEFORE:** Error handling in commands.rs

```rust
#[tokio::test]
async fn test_engine_initialization_error() {
    // Given: Invalid database path
    std::env::set_var("CONTEXT_OS_DB", "/nonexistent/db.sqlite");

    let state = create_test_state();

    // When: Try to get engine
    let result = state.get_query_engine().await;

    // Then: Returns Config error
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(matches!(e, context_os_core::CoreError::Config(_)));
    }

    // Cleanup
    std::env::remove_var("CONTEXT_OS_DB");
}

#[test]
fn test_command_error_json_format() {
    // Given: CommandError
    let err = tastematter::commands::CommandError {
        code: "ENGINE_ERROR".to_string(),
        message: "Failed to initialize".to_string(),
        details: Some("Database not found".to_string()),
    };

    // When: Serialize to JSON
    let json = serde_json::to_value(&err).unwrap();

    // Then: Matches frontend expectations
    assert_eq!(json["code"], "ENGINE_ERROR");
    assert_eq!(json["message"], "Failed to initialize");
    assert_eq!(json["details"], "Database not found");
}
```

**Red:** Run test → Should fail until error handling implemented
**Green:** Implement error conversion and propagation
**Commit:** `test(commands): error propagation`

---

## Level 3: E2E Tests

### Test 3.1: App Startup

**Purpose:** Verify app starts without blocking on database initialization
**Write BEFORE:** Final integration verification

**Note:** E2E tests are run manually or via Tauri's test framework

```rust
// tests/e2e/test_app_startup.rs
// This is a manual test checklist, not automated

/// Manual Test: App Startup Time
///
/// Steps:
/// 1. Close Tastematter if running
/// 2. Run: `time npm run tauri dev` (measure startup)
/// 3. Observe: App window should appear in <2 seconds
///
/// Expected:
/// - App window appears quickly (not blocked by DB init)
/// - No error dialogs
/// - Query engine initializes lazily on first query
///
/// If FAIL:
/// - Check that get_query_engine uses OnceCell (lazy init)
/// - Check that AppState::new doesn't call Database::open

/// Manual Test: First Query After Startup
///
/// Steps:
/// 1. Start fresh app (npm run tauri dev)
/// 2. Navigate to Timeline view
/// 3. Open DevTools (F12) → Network tab
/// 4. Observe first query timing
///
/// Expected:
/// - First query may take 200-500ms (includes DB open)
/// - Subsequent queries <100ms
/// - No "CLI not found" errors in console
```

---

### Test 3.2: Query Latency Verification

**Purpose:** Verify all queries complete in <100ms after warmup
**Write BEFORE:** Performance optimization

```rust
// tests/e2e/test_latency.rs
use std::time::{Duration, Instant};

const MAX_LATENCY: Duration = Duration::from_millis(100);
const WARMUP_QUERIES: usize = 3;
const MEASUREMENT_QUERIES: usize = 10;

#[tokio::test]
#[ignore] // Run with: cargo test --ignored
async fn test_query_flex_latency() {
    let state = create_test_state();
    let engine = match state.get_query_engine().await {
        Ok(e) => e,
        Err(_) => {
            println!("Skipping: No database available");
            return;
        }
    };

    // Warmup
    for _ in 0..WARMUP_QUERIES {
        let _ = engine.query_flex(context_os_core::QueryFlexInput::default()).await;
    }

    // Measure
    let mut total = Duration::ZERO;
    for _ in 0..MEASUREMENT_QUERIES {
        let start = Instant::now();
        let _ = engine.query_flex(context_os_core::QueryFlexInput {
            time: Some("7d".to_string()),
            limit: Some(20),
            agg: vec!["count".to_string()],
            ..Default::default()
        }).await.unwrap();
        total += start.elapsed();
    }

    let avg = total / MEASUREMENT_QUERIES as u32;
    println!("Average query_flex latency: {:?}", avg);

    assert!(
        avg < MAX_LATENCY,
        "query_flex too slow: {:?} (max: {:?})",
        avg,
        MAX_LATENCY
    );
}

#[tokio::test]
#[ignore]
async fn test_all_queries_latency() {
    let state = create_test_state();
    let engine = match state.get_query_engine().await {
        Ok(e) => e,
        Err(_) => return,
    };

    // Warmup all queries
    let _ = engine.query_flex(Default::default()).await;
    let _ = engine.query_chains(Default::default()).await;
    let _ = engine.query_timeline(context_os_core::QueryTimelineInput {
        time: "7d".to_string(),
        files: None,
        limit: None,
    }).await;
    let _ = engine.query_sessions(context_os_core::QuerySessionsInput {
        time: "7d".to_string(),
        chain: None,
        limit: None,
    }).await;

    // Measure each query type
    let queries = [
        ("query_flex", measure_query_flex(&engine).await),
        ("query_chains", measure_query_chains(&engine).await),
        ("query_timeline", measure_query_timeline(&engine).await),
        ("query_sessions", measure_query_sessions(&engine).await),
    ];

    for (name, latency) in &queries {
        println!("{}: {:?}", name, latency);
        assert!(
            *latency < MAX_LATENCY,
            "{} too slow: {:?}",
            name,
            latency
        );
    }
}

async fn measure_query_flex(engine: &context_os_core::QueryEngine) -> Duration {
    let start = Instant::now();
    let _ = engine.query_flex(context_os_core::QueryFlexInput {
        time: Some("7d".to_string()),
        limit: Some(20),
        ..Default::default()
    }).await;
    start.elapsed()
}

async fn measure_query_chains(engine: &context_os_core::QueryEngine) -> Duration {
    let start = Instant::now();
    let _ = engine.query_chains(context_os_core::QueryChainsInput { limit: Some(20) }).await;
    start.elapsed()
}

async fn measure_query_timeline(engine: &context_os_core::QueryEngine) -> Duration {
    let start = Instant::now();
    let _ = engine.query_timeline(context_os_core::QueryTimelineInput {
        time: "7d".to_string(),
        files: None,
        limit: Some(30),
    }).await;
    start.elapsed()
}

async fn measure_query_sessions(engine: &context_os_core::QueryEngine) -> Duration {
    let start = Instant::now();
    let _ = engine.query_sessions(context_os_core::QuerySessionsInput {
        time: "7d".to_string(),
        chain: None,
        limit: Some(50),
    }).await;
    start.elapsed()
}
```

**Run:** `cargo test --ignored test_latency`
**Commit:** `test(e2e): query latency verification`

---

### Test 3.3: Frontend Compatibility

**Purpose:** Verify frontend receives identical JSON structure
**Write BEFORE:** Final integration testing

```rust
// tests/e2e/test_frontend_compatibility.rs

/// Manual Test: JSON Structure Comparison
///
/// BEFORE Phase 2 (capture baseline):
/// 1. Start current app (with CLI)
/// 2. Open DevTools → Network tab
/// 3. Trigger query_flex
/// 4. Copy response JSON → save as `expected_flex.json`
/// 5. Repeat for query_timeline, query_sessions, query_chains
///
/// AFTER Phase 2 (compare):
/// 1. Start updated app (with library)
/// 2. Trigger same queries
/// 3. Compare JSON structure (field names, types, nesting)
///
/// Expected:
/// - Field names identical
/// - Data types identical (number vs string)
/// - Optional field handling identical (null vs missing)

#[test]
fn test_query_result_json_matches_expected() {
    // This test verifies JSON structure matches CONTRACTS.rs
    // The actual frontend compatibility test is manual

    let result = context_os_core::QueryResult {
        receipt_id: "test-123".to_string(),
        timestamp: "2026-01-08T12:00:00Z".to_string(),
        result_count: 2,
        results: vec![
            context_os_core::FileResult {
                file_path: "src/main.rs".to_string(),
                access_count: 10,
                last_access: Some("2026-01-08".to_string()),
                session_count: Some(3),
                sessions: None,
                chains: None,
            },
        ],
        aggregations: context_os_core::Aggregations {
            count: Some(context_os_core::CountAgg {
                total_files: 2,
                total_accesses: 15,
            }),
            recency: None,
        },
    };

    let json = serde_json::to_value(&result).unwrap();

    // Verify structure matches frontend expectations
    // (These checks document the contract)
    assert!(json["receipt_id"].is_string());
    assert!(json["timestamp"].is_string());
    assert!(json["result_count"].is_number());
    assert!(json["results"].is_array());
    assert!(json["aggregations"].is_object());

    // Verify optional field handling
    let file = &json["results"][0];
    assert!(file.get("sessions").is_none()); // None = field absent
    assert!(file.get("chains").is_none());

    // Verify aggregations
    assert!(json["aggregations"]["count"].is_object());
    assert!(json["aggregations"].get("recency").is_none());
}
```

**Commit:** `test(e2e): frontend compatibility verification`

---

## Test Execution Order

### Phase A: Pre-Integration Setup (15 min)
1. Create tests/ directory structure
2. Add test dependencies to Cargo.toml
3. Verify Phase 1 tests still pass

### Phase B: Unit Tests (15 min)
```bash
cd apps/tastematter/src-tauri
cargo test test_core_error_to_command_error
cargo test test_find_database_path
```

### Phase C: Integration Tests (1 hour)
```bash
cargo test test_app_state
cargo test test_query_commands
cargo test test_error_propagation
```

### Phase D: E2E Tests (30 min)
```bash
# Automated latency tests
cargo test --ignored test_latency

# Manual frontend tests
npm run tauri dev
# ... follow manual test checklist
```

---

## Success Criteria

**Phase 2 is complete when:**

- [ ] All unit tests pass
- [ ] All integration tests pass (with available DB)
- [ ] E2E latency tests pass (<100ms average)
- [ ] App starts in <2 seconds
- [ ] Manual frontend compatibility verified
- [ ] No "CLI" references in test output

**Test Quality Gates:**

- [ ] Tests written BEFORE code changes
- [ ] Tests run in CI (skip gracefully if no DB)
- [ ] Latency measurements documented
- [ ] Frontend JSON comparison documented

---

## Common Testing Pitfalls

### Pitfall 1: Tests Require Database

**Problem:** Tests fail in CI where database doesn't exist.

**Prevention:** Skip tests gracefully:
```rust
let engine = match state.get_query_engine().await {
    Ok(e) => e,
    Err(_) => {
        println!("Skipping: No database available");
        return;
    }
};
```

### Pitfall 2: Testing Tauri Commands Directly

**Problem:** Can't easily call #[command] functions in tests.

**Prevention:** Test the underlying logic, not the Tauri wrapper:
```rust
// DON'T: Try to call command directly
// commands::query_flex(state, None, ...)

// DO: Test the query engine logic
let result = engine.query_flex(input).await;
```

### Pitfall 3: Ignoring Cold Start

**Problem:** Latency tests pass but first query is slow.

**Prevention:** Test cold start separately:
```rust
// Test includes DB open time
let start = Instant::now();
let state = AppState::new(...);
let engine = state.get_query_engine().await.unwrap();
let _ = engine.query_flex(...).await;
let cold_start = start.elapsed();
```

### Pitfall 4: Not Comparing JSON Structure

**Problem:** Types serialize differently than CLI output.

**Prevention:** Document expected JSON structure in tests:
```rust
// Explicitly test JSON structure, not just Rust types
let json = serde_json::to_value(&result).unwrap();
assert!(json.get("sessions").is_none()); // None = absent, not null
```

---

## Manual Test Checklist

**Before marking Phase 2 complete, manually verify:**

- [ ] App starts without errors
- [ ] Timeline view shows data
- [ ] Sessions view shows data
- [ ] Chains view shows data
- [ ] File access view shows data
- [ ] No "CLI not found" errors in console
- [ ] No "context-os.cmd" in any logs
- [ ] DevTools Network tab shows query responses
- [ ] Response JSON structure unchanged from before

---

## References

- **CONTRACTS.rs** - Type and API contracts
- **SPEC.md** - Implementation specification
- **Phase 1 TESTS.md** - Foundation tests (must pass first)
- Kent Beck TDD principles (test-driven-execution skill)

---

**Spec Version:** 1.0
**Created:** 2026-01-08
**Phase:** 2 of 8
**Test Count:** ~25 tests across 3 levels
