# Agent Task Spec: HTTP Server Implementation

**Phase:** 3.1 - HTTP Server Foundation
**Estimated Time:** 2-3 hours
**Agent Type:** Implementation Agent
**Skill Reference:** specification-driven-development

---

## Mission Statement

Implement an HTTP API server mode for context-os-core that exposes the existing QueryEngine over HTTP. This enables browser-based development and testing without modifying any existing Tauri functionality.

**You are NOT:**
- Modifying existing Tauri IPC code
- Changing the QueryEngine implementation
- Adding authentication (not needed for localhost dev)
- Adding caching (latency is already <2ms)

**You ARE:**
- Adding axum HTTP server
- Adding `serve` subcommand to CLI
- Exposing existing QueryEngine functions over HTTP
- Adding CORS for browser access

---

## Read-First Checklist

Read these files IN ORDER before writing any code:

1. **[[04_TRANSPORT_ARCHITECTURE.md]]** - Full architecture spec (you're implementing Phase 1)
   - Focus: Architecture decisions, latency budget, type contracts
   - Location: `apps/tastematter/specs/canonical/04_TRANSPORT_ARCHITECTURE.md`

2. **[[types.rs]]** - Existing type contracts (DO NOT MODIFY)
   - Focus: Input/Output types you'll serialize over HTTP
   - Location: `apps/context-os/core/src/types.rs`

3. **[[query.rs]]** - QueryEngine implementation
   - Focus: The 4 functions you're exposing: query_flex, query_timeline, query_sessions, query_chains
   - Location: `apps/context-os/core/src/query.rs`

4. **[[main.rs]]** - Existing CLI structure
   - Focus: Understand subcommand pattern, add `serve` subcommand
   - Location: `apps/context-os/core/src/main.rs`

5. **[[Cargo.toml]]** - Current dependencies
   - Focus: What's already included (tokio, serde, etc.)
   - Location: `apps/context-os/core/Cargo.toml`

---

## Implementation Steps

### Step 1: Add Dependencies (5 min)

**File:** `apps/context-os/core/Cargo.toml`

Add to `[dependencies]`:
```toml
axum = "0.7"
tower-http = { version = "0.5", features = ["cors"] }
```

**Verification:**
```bash
cd apps/context-os/core && cargo check
```

### Step 2: Create HTTP Module (30 min)

**File:** `apps/context-os/core/src/http.rs` (NEW)

```rust
//! HTTP API server for context-os-core
//!
//! Exposes QueryEngine over HTTP for browser-based development.
//! NOT for production use - binds to localhost only.

use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use tower_http::cors::{Any, CorsLayer};

use crate::{
    QueryEngine,
    QueryFlexInput, QueryResult,
    QueryTimelineInput, TimelineData,
    QuerySessionsInput, SessionQueryResult,
    QueryChainsInput, ChainQueryResult,
    CoreError,
};

/// Application state shared across handlers
pub struct AppState {
    pub engine: QueryEngine,
    pub start_time: Instant,
}

/// Health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: String,
    pub version: String,
    pub database: String,
    pub uptime_seconds: u64,
}

/// Error response format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub error: String,
    pub message: String,
}

impl From<CoreError> for (StatusCode, Json<ApiError>) {
    fn from(err: CoreError) -> Self {
        let api_error = ApiError {
            error: "QueryError".to_string(),
            message: err.to_string(),
        };
        (StatusCode::BAD_REQUEST, Json(api_error))
    }
}

/// Create the HTTP router
pub fn create_router(state: Arc<AppState>, enable_cors: bool) -> Router {
    let mut router = Router::new()
        .route("/api/health", get(health_handler))
        .route("/api/query/flex", post(query_flex_handler))
        .route("/api/query/timeline", post(query_timeline_handler))
        .route("/api/query/sessions", post(query_sessions_handler))
        .route("/api/query/chains", post(query_chains_handler))
        .with_state(state);

    if enable_cors {
        router = router.layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any)
        );
    }

    router
}

// Handler implementations
async fn health_handler(
    State(state): State<Arc<AppState>>,
) -> Json<HealthStatus> {
    Json(HealthStatus {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        database: "connected".to_string(),
        uptime_seconds: state.start_time.elapsed().as_secs(),
    })
}

async fn query_flex_handler(
    State(state): State<Arc<AppState>>,
    Json(input): Json<QueryFlexInput>,
) -> Result<Json<QueryResult>, (StatusCode, Json<ApiError>)> {
    state.engine.query_flex(input).await
        .map(Json)
        .map_err(Into::into)
}

async fn query_timeline_handler(
    State(state): State<Arc<AppState>>,
    Json(input): Json<QueryTimelineInput>,
) -> Result<Json<TimelineData>, (StatusCode, Json<ApiError>)> {
    state.engine.query_timeline(input).await
        .map(Json)
        .map_err(Into::into)
}

async fn query_sessions_handler(
    State(state): State<Arc<AppState>>,
    Json(input): Json<QuerySessionsInput>,
) -> Result<Json<SessionQueryResult>, (StatusCode, Json<ApiError>)> {
    state.engine.query_sessions(input).await
        .map(Json)
        .map_err(Into::into)
}

async fn query_chains_handler(
    State(state): State<Arc<AppState>>,
    Json(input): Json<QueryChainsInput>,
) -> Result<Json<ChainQueryResult>, (StatusCode, Json<ApiError>)> {
    state.engine.query_chains(input).await
        .map(Json)
        .map_err(Into::into)
}
```

**Verification:**
```bash
cd apps/context-os/core && cargo check
```

### Step 3: Add to lib.rs (2 min)

**File:** `apps/context-os/core/src/lib.rs`

Add module declaration:
```rust
pub mod http;
```

### Step 4: Add Serve Subcommand (20 min)

**File:** `apps/context-os/core/src/main.rs`

Add to the existing Commands enum:
```rust
#[derive(Subcommand)]
enum Commands {
    Query {
        #[command(subcommand)]
        command: QueryCommands,
    },
    /// Start HTTP API server for development
    Serve {
        /// Port to listen on (default: 3001)
        #[arg(long, default_value = "3001")]
        port: u16,

        /// Host to bind to (default: 127.0.0.1)
        #[arg(long, default_value = "127.0.0.1")]
        host: String,

        /// Enable CORS for browser access
        #[arg(long)]
        cors: bool,
    },
}
```

Add handler in main():
```rust
Commands::Serve { port, host, cors } => {
    use crate::http::{create_router, AppState};
    use std::sync::Arc;
    use std::time::Instant;

    let db = Database::open_default().await?;
    let engine = QueryEngine::new(db);

    let state = Arc::new(AppState {
        engine,
        start_time: Instant::now(),
    });

    let router = create_router(state, cors);
    let addr = format!("{}:{}", host, port);

    println!("Starting HTTP API server on http://{}", addr);
    println!("Press Ctrl+C to stop");

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, router).await?;
}
```

**Verification:**
```bash
cd apps/context-os/core && cargo build --bin context-os
./target/debug/context-os serve --help
```

### Step 5: Test HTTP Server (15 min)

**Manual Testing:**

Terminal 1:
```bash
cd apps/context-os/core
cargo run --bin context-os -- serve --port 3001 --cors
```

Terminal 2:
```bash
# Health check
curl http://localhost:3001/api/health

# Query flex
curl -X POST http://localhost:3001/api/query/flex \
  -H "Content-Type: application/json" \
  -d '{"time": "7d", "limit": 5}'

# Query chains
curl -X POST http://localhost:3001/api/query/chains \
  -H "Content-Type: application/json" \
  -d '{"limit": 10}'

# Query timeline
curl -X POST http://localhost:3001/api/query/timeline \
  -H "Content-Type: application/json" \
  -d '{"time": "7d"}'

# Query sessions
curl -X POST http://localhost:3001/api/query/sessions \
  -H "Content-Type: application/json" \
  -d '{"time": "7d", "limit": 20}'
```

### Step 6: Write Integration Tests (30 min)

**File:** `apps/context-os/core/tests/http_integration_test.rs` (NEW)

```rust
//! HTTP API integration tests

use context_os_core::{
    Database, QueryEngine,
    QueryFlexInput, QueryChainsInput,
    http::{create_router, AppState},
};
use std::sync::Arc;
use std::time::Instant;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

fn create_test_router() -> axum::Router {
    // Uses test database - adjust path as needed
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (engine, state) = rt.block_on(async {
        let db = Database::open_default().await.unwrap();
        let engine = QueryEngine::new(db);
        let state = Arc::new(AppState {
            engine,
            start_time: Instant::now(),
        });
        (engine, state)
    });

    create_router(state, true)
}

#[tokio::test]
async fn test_health_endpoint() {
    let app = create_test_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/health")
                .body(Body::empty())
                .unwrap()
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_query_flex_endpoint() {
    let app = create_test_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/query/flex")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"time": "7d", "limit": 5}"#))
                .unwrap()
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_query_chains_endpoint() {
    let app = create_test_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/query/chains")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"limit": 10}"#))
                .unwrap()
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
```

**Run tests:**
```bash
cd apps/context-os/core && cargo test http
```

---

## Type Contracts

### Input Types (FROM types.rs - DO NOT MODIFY)

These types are already defined and MUST match HTTP request bodies:

```rust
// QueryFlexInput
{
  "files": null,           // Optional<String>
  "time": "7d",            // Optional<String>
  "chain": null,           // Optional<String>
  "session": null,         // Optional<String>
  "agg": ["count"],        // Vec<String>
  "limit": 50,             // Optional<u32>
  "sort": "count"          // Optional<String>
}

// QueryTimelineInput
{
  "time": "7d",            // String (required)
  "files": null,           // Optional<String>
  "chain": null,           // Optional<String>
  "limit": 30              // Optional<u32>
}

// QuerySessionsInput
{
  "time": "7d",            // String (required)
  "chain": null,           // Optional<String>
  "limit": 100             // Optional<u32>
}

// QueryChainsInput
{
  "limit": 20              // Optional<u32>
}
```

### Output Types (FROM types.rs - DO NOT MODIFY)

HTTP responses MUST serialize to the same JSON as existing CLI output.

### New Types (http.rs)

```rust
// HealthStatus
{
  "status": "ok",
  "version": "0.1.0",
  "database": "connected",
  "uptime_seconds": 3600
}

// ApiError
{
  "error": "QueryError",
  "message": "Invalid time range: abc"
}
```

---

## Success Criteria

**MUST pass before marking complete:**

- [ ] `cargo build` succeeds with no warnings
- [ ] `context-os serve --help` shows correct options
- [ ] `curl localhost:3001/api/health` returns 200
- [ ] `curl -X POST localhost:3001/api/query/flex -d '{"time":"7d"}'` returns data
- [ ] All 4 query endpoints return same data as CLI equivalents
- [ ] CORS enabled: Browser can fetch from different origin
- [ ] Integration tests pass: `cargo test http`
- [ ] Existing tests still pass: `cargo test` (15 tests)

**Performance check:**
```bash
# Should be <100ms
time curl -X POST localhost:3001/api/query/flex \
  -H "Content-Type: application/json" \
  -d '{"time": "7d", "limit": 50}'
```

---

## Common Pitfalls

### Pitfall 1: Forgetting to Update lib.rs

**Symptom:** `unresolved import http`
**Fix:** Add `pub mod http;` to lib.rs

### Pitfall 2: Wrong Error Type

**Symptom:** Type mismatch in handler return
**Fix:** Implement `From<CoreError> for (StatusCode, Json<ApiError>)`

### Pitfall 3: Missing Content-Type

**Symptom:** HTTP 415 Unsupported Media Type
**Fix:** Ensure curl/fetch includes `Content-Type: application/json`

### Pitfall 4: CORS Not Working

**Symptom:** Browser shows CORS error
**Fix:** Ensure `--cors` flag is passed to serve command

### Pitfall 5: Port Already in Use

**Symptom:** `Address already in use`
**Fix:** Kill existing process or use different port `--port 3002`

---

## Files Created/Modified Summary

| File | Action | Lines |
|------|--------|-------|
| `Cargo.toml` | MODIFY | +2 deps |
| `src/lib.rs` | MODIFY | +1 line |
| `src/http.rs` | CREATE | ~120 lines |
| `src/main.rs` | MODIFY | ~30 lines |
| `tests/http_integration_test.rs` | CREATE | ~80 lines |

**Total new code:** ~230 lines

---

## Completion Report Template

After completing, write to `PHASE_3_1_COMPLETION_REPORT.md`:

```markdown
# Phase 3.1 Completion Report

**Status:** ✅ COMPLETE | ⚠️ INCOMPLETE

## What Was Implemented
- [ ] HTTP module (src/http.rs)
- [ ] Serve subcommand
- [ ] Integration tests

## Test Results
- Unit tests: X passing
- Integration tests: X passing
- Manual curl tests: ✅ All endpoints working

## Performance
- Health endpoint: Xms
- Query flex: Xms
- Query timeline: Xms

## Known Issues
[List any issues discovered]

## Next Agent
Proceed to Phase 3.2: Frontend Transport Abstraction
```

---

**Spec Version:** 1.0
**Last Updated:** 2026-01-09
