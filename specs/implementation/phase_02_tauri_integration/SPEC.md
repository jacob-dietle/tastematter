# Phase 2: Tauri Integration - Agent Task Specification

## Executive Summary

**Mission:** Replace Command::new() CLI calls in Tastematter with direct context-os-core library calls, achieving <100ms query latency with zero frontend changes.

**Why This Matters:** This phase delivers visible user value. The 18-second query latency becomes <100ms. Users see instant results instead of spinning indicators.

**Success Definition:** App launches, queries return in <100ms, frontend works identically.

**Dependencies:** Phase 1 must be complete with all tests passing.

---

## Prerequisites

### Required Reading (In Order)

1. **Phase 1 Completion:** Verify `apps/context-os-core/` exists and `cargo test` passes
2. **This spec** (you're reading it)
3. **Current commands.rs:** `apps/tastematter/src-tauri/src/commands.rs`
   - Focus on: lines 88-155 (query_flex), 389-514 (query_timeline)
4. **Current lib.rs:** `apps/tastematter/src-tauri/src/lib.rs`
   - Focus on: AppState struct, invoke_handler registration
5. **Type contracts:** `phase_01_core_foundation/CONTRACTS.rs`
   - These types MUST match - no frontend changes allowed

### Environment Verification

```bash
# Verify Phase 1 complete
cd apps/context-os-core
cargo test

# Verify Tastematter builds
cd apps/tastematter/src-tauri
cargo build

# Verify database exists
ls -la ~/.context-os/context.db
```

---

## Architecture Context

### The Problem We're Solving

```
Current Flow (18 seconds):
  Tauri Command (commands.rs)
      ↓
  Command::new("context-os.cmd")     [500ms - process spawn]
      ↓
  Python CLI starts                   [2-3s - imports]
      ↓
  ContextIndex.load()                 [15s - loads ENTIRE DB]
      ↓
  QueryEngine.execute()               [50ms - actual query]
      ↓
  Return JSON → Parse → Return

After Phase 2 (<100ms):
  Tauri Command (commands.rs)
      ↓
  context_os_core::query_flex()      [<100ms - direct SQL]
      ↓
  Return QueryResult (same type)
```

### What Changes

**Files Modified:**
```
apps/tastematter/src-tauri/
├── Cargo.toml         # Add context-os-core dependency
├── src/
│   ├── lib.rs         # Add Database to AppState
│   └── commands.rs    # Replace Command::new() with library calls
```

**Files NOT Changed:**
- All frontend code (src/)
- Return types (same JSON structure)
- Command signatures (same parameters)

---

## Type Contract Verification

**CRITICAL:** Phase 2 must return the EXACT same JSON structure as current CLI.

### Query Result Contract

The types in `context-os-core/src/types.rs` MUST serialize identically to current `commands.rs` types:

```rust
// commands.rs (current)               // context-os-core (Phase 1)
pub struct QueryResult {               pub struct QueryResult {
    pub receipt_id: String,                pub receipt_id: String,
    pub timestamp: String,                 pub timestamp: String,
    pub result_count: usize,               pub result_count: usize,
    pub results: Vec<FileResult>,          pub results: Vec<FileResult>,
    pub aggregations: Aggregations,        pub aggregations: Aggregations,
}                                      }
```

**Verification Step:** Before modifying commands.rs, run Phase 1 contract tests to confirm JSON match.

---

## Implementation Steps

### Step 1: Add Dependency (5 min)

Modify `apps/tastematter/src-tauri/Cargo.toml`:

```toml
[dependencies]
# Existing dependencies...
tauri = { version = "2", features = [...] }
serde = { version = "1", features = ["derive"] }
# ... other existing deps ...

# ADD THIS: Link to context-os-core
context-os-core = { path = "../../context-os-core" }
```

**Verify:**
```bash
cd apps/tastematter/src-tauri
cargo check  # Should compile with new dependency
```

---

### Step 2: Update AppState (15 min)

Modify `apps/tastematter/src-tauri/src/lib.rs`:

**Before:**
```rust
mod commands;
mod logging;

use logging::LogService;
use std::sync::Arc;

pub struct AppState {
    pub log_service: Arc<LogService>,
}
```

**After:**
```rust
mod commands;
mod logging;

use context_os_core::{Database, QueryEngine};
use logging::LogService;
use std::sync::Arc;
use tokio::sync::OnceCell;

pub struct AppState {
    pub log_service: Arc<LogService>,
    pub query_engine: Arc<OnceCell<QueryEngine>>,
}

impl AppState {
    /// Initialize query engine lazily on first use
    /// This avoids blocking app startup if database isn't ready
    pub async fn get_query_engine(&self) -> Result<&QueryEngine, context_os_core::CoreError> {
        self.query_engine.get_or_try_init(|| async {
            let db_path = find_database_path()?;
            let db = Database::open(&db_path).await?;
            Ok(QueryEngine::new(db))
        }).await
    }
}

fn find_database_path() -> Result<std::path::PathBuf, context_os_core::CoreError> {
    let candidates = [
        dirs::home_dir().map(|h| h.join(".context-os/context.db")),
        // Add other standard locations if needed
    ];

    for path in candidates.into_iter().flatten() {
        if path.exists() {
            return Ok(path);
        }
    }

    Err(context_os_core::CoreError::Config(
        "Database not found. Is context-os daemon running?".into()
    ))
}
```

**Update run() function:**
```rust
pub fn run() {
    let log_service = Arc::new(LogService::new());

    tauri::Builder::default()
        .manage(AppState {
            log_service: log_service.clone(),
            query_engine: Arc::new(OnceCell::new()),
        })
        // ... rest unchanged ...
}
```

**Key Design Decisions:**
- `OnceCell` for lazy initialization (don't block startup)
- `Arc` for thread-safe sharing across commands
- Error handling if database not found

---

### Step 3: Replace query_flex Command (30 min)

This is the main transformation. Replace the Command::new() pattern with direct library call.

**Before (commands.rs:88-155):**
```rust
#[command]
pub async fn query_flex(
    files: Option<String>,
    time: Option<String>,
    chain: Option<String>,
    session: Option<String>,
    agg: Vec<String>,
    limit: Option<u32>,
    sort: Option<String>,
) -> Result<QueryResult, CommandError> {
    info!("[query_flex] time={:?}, chain={:?}, limit={:?}", time, chain, limit);

    // Build command with context-os CLI path
    let cli_path = std::env::var("CONTEXT_OS_CLI")
        .unwrap_or_else(|_| "C:/Users/dietl/.context-os/bin/context-os.cmd".to_string());

    let mut cmd = Command::new(&cli_path);
    cmd.current_dir("../../..");
    cmd.args(["query", "flex", "--format", "json"]);
    // ... 40 more lines of CLI building ...
}
```

**After:**
```rust
use context_os_core::{QueryFlexInput, QueryResult as CoreQueryResult};
use tauri::State;

#[command]
pub async fn query_flex(
    state: State<'_, crate::AppState>,
    files: Option<String>,
    time: Option<String>,
    chain: Option<String>,
    session: Option<String>,
    agg: Vec<String>,
    limit: Option<u32>,
    sort: Option<String>,
) -> Result<QueryResult, CommandError> {
    info!("[query_flex] time={:?}, chain={:?}, limit={:?}", time, chain, limit);

    // Get query engine from app state
    let engine = state.get_query_engine().await.map_err(|e| CommandError {
        code: "ENGINE_ERROR".to_string(),
        message: "Failed to initialize query engine".to_string(),
        details: Some(e.to_string()),
    })?;

    // Build input struct
    let input = QueryFlexInput {
        files,
        time,
        chain,
        session,
        agg,
        limit,
        sort,
    };

    // Execute query directly (<100ms!)
    let result = engine.query_flex(input).await.map_err(|e| {
        let cmd_err: CommandError = e.into();
        cmd_err
    })?;

    info!("[query_flex] success: {} results", result.result_count);

    // Convert if needed (should be identical types)
    Ok(result.into())
}
```

**Key Changes:**
1. Add `state: State<'_, crate::AppState>` parameter
2. Remove all Command::new() code (~40 lines)
3. Call `engine.query_flex(input).await` directly (~3 lines)
4. Total: -40 lines, +10 lines = 30 lines removed

---

### Step 4: Replace query_timeline Command (30 min)

**Before (commands.rs:389-514):** Complex transformation of CLI output to TimelineData

**After:** Direct call plus same transformation logic

```rust
#[command]
pub async fn query_timeline(
    state: State<'_, crate::AppState>,
    time: String,
    files: Option<String>,
    limit: Option<u32>,
) -> Result<TimelineData, CommandError> {
    info!("[query_timeline] time={}, limit={:?}", time, limit);

    let engine = state.get_query_engine().await.map_err(|e| CommandError {
        code: "ENGINE_ERROR".to_string(),
        message: "Failed to initialize query engine".to_string(),
        details: Some(e.to_string()),
    })?;

    // Use context-os-core's timeline query
    let input = QueryTimelineInput {
        time: time.clone(),
        files,
        limit,
    };

    let result = engine.query_timeline(input).await.map_err(|e| {
        let cmd_err: CommandError = e.into();
        cmd_err
    })?;

    info!("[query_timeline] success: {} buckets", result.buckets.len());
    Ok(result.into())
}
```

**Note:** The timeline transformation logic (buckets, day_of_week, etc.) should be in context-os-core, not commands.rs.

---

### Step 5: Replace query_sessions Command (20 min)

Similar pattern:

```rust
#[command]
pub async fn query_sessions(
    state: State<'_, crate::AppState>,
    time: String,
    chain: Option<String>,
    limit: Option<u32>,
) -> Result<SessionQueryResult, CommandError> {
    info!("[query_sessions] time={}, chain={:?}", time, chain);

    let engine = state.get_query_engine().await.map_err(|e| CommandError {
        code: "ENGINE_ERROR".to_string(),
        message: "Failed to initialize query engine".to_string(),
        details: Some(e.to_string()),
    })?;

    let input = QuerySessionsInput { time, chain, limit };

    let result = engine.query_sessions(input).await.map_err(|e| {
        let cmd_err: CommandError = e.into();
        cmd_err
    })?;

    info!("[query_sessions] success: {} sessions", result.sessions.len());
    Ok(result.into())
}
```

---

### Step 6: Replace query_chains Command (15 min)

```rust
#[command]
pub async fn query_chains(
    state: State<'_, crate::AppState>,
    limit: Option<u32>,
) -> Result<ChainQueryResult, CommandError> {
    info!("[query_chains] limit={:?}", limit);

    let engine = state.get_query_engine().await.map_err(|e| CommandError {
        code: "ENGINE_ERROR".to_string(),
        message: "Failed to initialize query engine".to_string(),
        details: Some(e.to_string()),
    })?;

    let input = QueryChainsInput { limit };

    let result = engine.query_chains(input).await.map_err(|e| {
        let cmd_err: CommandError = e.into();
        cmd_err
    })?;

    info!("[query_chains] success: {} chains", result.chains.len());
    Ok(result.into())
}
```

---

### Step 7: Remove Dead Code (10 min)

After replacing all commands, remove:

1. **CLI path logic:**
   ```rust
   // DELETE: No longer needed
   let cli_path = std::env::var("CONTEXT_OS_CLI")
       .unwrap_or_else(|_| "C:/Users/dietl/.context-os/bin/context-os.cmd".to_string());
   ```

2. **Command building code:**
   ```rust
   // DELETE: All Command::new() blocks
   let mut cmd = Command::new(&cli_path);
   cmd.current_dir("../../..");
   cmd.args([...]);
   ```

3. **CLI error handling:**
   ```rust
   // DELETE: CLI_NOT_FOUND error variant (keep ENGINE_ERROR)
   ```

4. **Unused imports:**
   ```rust
   // DELETE if no longer used:
   use std::process::Command;
   ```

---

### Step 8: Update Cargo.toml Dependencies (5 min)

Add required dependencies for async state:

```toml
[dependencies]
# ... existing deps ...

# ADD: For OnceCell
tokio = { version = "1.40", features = ["sync"] }

# ADD: For home directory detection
dirs = "5.0"
```

---

### Step 9: Verify Build (10 min)

```bash
cd apps/tastematter/src-tauri

# Check compilation
cargo check

# Build debug
cargo build

# Run tests if any
cargo test
```

**Expected warnings to fix:**
- Unused imports (remove them)
- Dead code (remove it)

---

### Step 10: Integration Test (30 min)

```bash
# Start the app
cd apps/tastematter
npm run tauri dev

# In the app:
# 1. Navigate to a view that queries data
# 2. Open DevTools (F12)
# 3. Check console for timing logs
# 4. Verify: "[query_flex] success" appears quickly (<1 second visible)
```

**Manual verification checklist:**
- [ ] App starts without errors
- [ ] Timeline view loads data
- [ ] Sessions view loads data
- [ ] Chains view loads data
- [ ] No "CLI not found" errors
- [ ] Query results look identical to before

---

## Error Handling Matrix

| Error Scenario | Before (CLI) | After (Library) |
|----------------|--------------|-----------------|
| Database not found | CLI_NOT_FOUND | ENGINE_ERROR: "Database not found" |
| Database locked | CLI_ERROR | DATABASE_ERROR: "Database locked" |
| Invalid query | CLI_ERROR (generic) | QUERY_ERROR: specific message |
| Timeout | Process killed | Never (direct SQL is fast) |

---

## Success Criteria

**Phase 2 is complete when:**

- [ ] `cargo build` succeeds with no warnings
- [ ] All 4 query commands work (query_flex, query_timeline, query_sessions, query_chains)
- [ ] No frontend changes required (same JSON structure)
- [ ] App startup time < 2 seconds
- [ ] First query < 100ms (measured in DevTools)
- [ ] No "context-os.cmd" or CLI references in commands.rs
- [ ] Git commands still work (unchanged)
- [ ] Log events still work (unchanged)

---

## Common Pitfalls

### Pitfall 1: Type Mismatch Breaking Frontend

**Problem:** Rust types serialize differently than old CLI output.

**Prevention:**
1. Use EXACT same type definitions from CONTRACTS.rs
2. Run Phase 1 contract tests before integration
3. Compare JSON output with browser DevTools

**Verification:**
```bash
# Before Phase 2 - capture expected output
curl http://localhost:1420/api/query_flex?time=7d > expected.json

# After Phase 2 - compare
curl http://localhost:1420/api/query_flex?time=7d > actual.json
diff expected.json actual.json  # Should be empty
```

### Pitfall 2: Blocking App Startup

**Problem:** Database::open() blocks main thread, app hangs on startup.

**Prevention:** Use lazy initialization with OnceCell:
```rust
// GOOD: Lazy init
pub query_engine: Arc<OnceCell<QueryEngine>>

// BAD: Blocks startup
pub query_engine: Arc<QueryEngine>  // Must be created in run()
```

### Pitfall 3: Forgetting State Parameter

**Problem:** Commands don't have access to query engine.

**Prevention:** EVERY query command needs:
```rust
#[command]
pub async fn query_xxx(
    state: State<'_, crate::AppState>,  // REQUIRED
    // ... other params
)
```

### Pitfall 4: Wrong Working Directory

**Problem:** Database path is relative and fails.

**Prevention:** Use absolute paths:
```rust
// GOOD
dirs::home_dir().map(|h| h.join(".context-os/context.db"))

// BAD
PathBuf::from(".context-os/context.db")
```

### Pitfall 5: Not Removing Old Code

**Problem:** Dead CLI code left in codebase, confusing future developers.

**Prevention:** Delete ALL:
- `Command::new()` calls
- CLI path environment variable logic
- `current_dir("../../..")` hacks
- Unused imports

---

## Code Removal Checklist

After Phase 2, these should NOT exist in commands.rs:

- [ ] `Command::new()` - replaced with engine.query_xxx()
- [ ] `CONTEXT_OS_CLI` env var - no longer needed
- [ ] `current_dir("../../..")` - no longer needed
- [ ] `CLI_NOT_FOUND` error code - replaced with ENGINE_ERROR
- [ ] `use std::process::Command` - no longer needed
- [ ] JSON parsing of CLI output - types are native now

---

## Integration Notes

### For Phase 3 (Cache Layer)

Phase 3 will add caching to QueryEngine. Phase 2's integration will automatically benefit:
```rust
// Phase 2 code (unchanged in Phase 3)
let result = engine.query_flex(input).await?;

// Phase 3: engine internally uses cache
// No changes needed to commands.rs
```

### For Phase 5 (IPC Socket)

Phase 5 will add socket server to QueryEngine. Commands.rs doesn't need to know about it.

---

## Handoff Checklist

Before completing Phase 2, verify:

- [ ] All tests pass (`cargo test` in both context-os-core AND src-tauri)
- [ ] App starts and queries work
- [ ] No CLI references remain in code
- [ ] DevTools shows query latency < 100ms
- [ ] Code compiles without warnings
- [ ] COMPLETION_REPORT.md written

---

## File Changes Summary

**Modified:**
```
apps/tastematter/src-tauri/
├── Cargo.toml              # +3 lines (dependency)
├── src/
│   ├── lib.rs              # +30 lines (AppState changes)
│   └── commands.rs         # -150 lines, +60 lines (net -90 lines)
```

**Net change:** ~60 lines removed, code significantly simpler

---

**Spec Version:** 1.0
**Created:** 2026-01-08
**Phase:** 2 of 8
**Dependencies:** Phase 1 complete
**Estimated Time:** 2-3 hours
