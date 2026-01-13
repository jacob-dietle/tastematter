# Transport-Agnostic Architecture Specification

**Version:** 1.0
**Date:** 2026-01-09
**Status:** APPROVED
**Author:** Architecture Agent
**Skill Reference:** technical-architecture-engineering, specification-driven-development

---

## Executive Summary

This specification defines a **transport-agnostic architecture** for context-os-core that enables:
1. **Production:** Tauri IPC for native desktop app (no permission friction)
2. **Development:** HTTP API for browser-based testing with Claude Code automation
3. **Future:** Web deployment via PWA with user-granted filesystem access

**Core Insight:** The QueryEngine already exists. We just need to expose it over multiple transports.

---

## Problem Statement

### Current Pain Points

1. **Data issues hard to debug** - Arbitrary caps (100 sessions, 50 files) require manual testing
2. **No automation** - Tauri app can't be controlled by Claude Code browser automation
3. **Slow dev iteration** - Must rebuild Tauri to test frontend changes
4. **No E2E testing** - Playwright/WebDriver for Tauri requires complex setup

### Root Cause

The frontend is tightly coupled to Tauri IPC transport, not to the QueryEngine capabilities.

### Solution

Abstract the transport layer. Same QueryEngine, multiple access methods:

```
                    ┌─────────────────────────────────────┐
                    │        context-os-core              │
                    │  ┌────────────────────────────────┐ │
                    │  │      QueryEngine (Rust)        │ │
                    │  │  • query_flex()                │ │
                    │  │  • query_chains()              │ │
                    │  │  • query_timeline()            │ │
                    │  │  • query_sessions()            │ │
                    │  └────────────────────────────────┘ │
                    │              ▲                      │
                    │    ┌─────────┴─────────┐           │
                    │    │                   │           │
                    │  ┌─┴─┐              ┌──┴──┐        │
                    │  │CLI│              │HTTP │        │
                    │  └─┬─┘              └──┬──┘        │
                    └────┼──────────────────┼───────────┘
                         │                  │
              ┌──────────┴──────┐  ┌────────┴────────┐
              │ tastematter CLI │  │ localhost:3001  │
              └─────────────────┘  └─────────────────┘
                                          ▲
                    ┌─────────────────────┼─────────────────────┐
                    │                     │                     │
              ┌─────▼─────┐        ┌──────▼──────┐       ┌──────▼──────┐
              │  Tauri    │        │  Browser    │       │   Claude    │
              │  Desktop  │        │  Dev Mode   │       │   Code      │
              │  (IPC)    │        │  (HTTP)     │       │  Automation │
              └───────────┘        └─────────────┘       └─────────────┘
```

---

## Latency Budget Analysis

**Target:** <100ms for any view switch (from [[01_PRINCIPLES.md]])

| Transport | Latency | Overhead | Use Case |
|-----------|---------|----------|----------|
| Direct function call (Rust→Rust) | <1μs | - | QueryEngine internal |
| Tauri IPC (JS→Rust→JS) | ~1-5ms | Serialization | Production desktop |
| HTTP localhost (JS→HTTP→Rust→HTTP→JS) | ~5-15ms | Network + serialization | Development |
| CLI spawn (shell→process→stdout) | ~100-200ms | Process creation | Scripting |

**Budget allocation for HTTP mode (15ms total):**
```
HTTP overhead:         5ms (33%)  ← Acceptable for dev
DB query:              8ms (53%)  ← Same as Tauri
JSON serialization:    2ms (14%)  ← Same as Tauri
```

**Verdict:** HTTP adds ~10ms overhead. Still under 100ms budget. Acceptable for development.

---

## IPC Pattern Selection (Technical Architecture Skill)

**Decision Tree Applied:**

```
Same process?
├─ QueryEngine → Database: YES → Direct call (<1μs) ✅
│
└─ Frontend → QueryEngine: NO
   └─ Same machine?
      └─ YES → Two options:
         ├─ Tauri IPC (production) → ~1-5ms ✅
         └─ HTTP localhost (dev) → ~5-15ms ✅
```

**Why HTTP over Unix socket:**
- Cross-platform (Windows + Mac + Linux)
- Browser-native (no plugins needed)
- Standard tooling (curl, fetch, Playwright)
- Claude Code browser automation compatibility

---

## Type Contracts

### Existing Contracts (No Changes)

All types in `apps/context-os/core/src/types.rs` remain unchanged:

**Input Types:**
- `QueryFlexInput` - Main query parameters
- `QueryTimelineInput` - Timeline-specific parameters
- `QuerySessionsInput` - Session-specific parameters
- `QueryChainsInput` - Chain-specific parameters

**Output Types:**
- `QueryResult` - Flex query response
- `TimelineData` - Timeline response
- `SessionQueryResult` - Sessions response
- `ChainQueryResult` - Chains response

### HTTP API Contract

```rust
// New file: apps/context-os/core/src/http.rs

/// HTTP API routes - all POST for consistency
///
/// POST /api/query/flex     → QueryResult
/// POST /api/query/timeline → TimelineData
/// POST /api/query/sessions → SessionQueryResult
/// POST /api/query/chains   → ChainQueryResult
/// GET  /api/health         → HealthStatus

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: String,        // "ok" | "error"
    pub version: String,       // "0.1.0"
    pub database: String,      // "connected" | "disconnected"
    pub uptime_seconds: u64,
}

/// Error response format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub error: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
}
```

### Frontend Transport Abstraction

```typescript
// New file: apps/tastematter/src/lib/api/transport.ts

export interface QueryTransport {
  queryFlex(input: QueryFlexInput): Promise<QueryResult>;
  queryTimeline(input: QueryTimelineInput): Promise<TimelineData>;
  querySessions(input: QuerySessionsInput): Promise<SessionQueryResult>;
  queryChains(input: QueryChainsInput): Promise<ChainQueryResult>;
}

// Tauri implementation (production)
export const tauriTransport: QueryTransport = {
  queryFlex: (input) => invoke('query_flex', input),
  queryTimeline: (input) => invoke('query_timeline', input),
  querySessions: (input) => invoke('query_sessions', input),
  queryChains: (input) => invoke('query_chains', input),
};

// HTTP implementation (development)
export const httpTransport: QueryTransport = {
  queryFlex: (input) => fetch('/api/query/flex', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(input),
  }).then(r => r.json()),
  // ... same pattern for other methods
};

// Auto-select based on environment
export const transport: QueryTransport =
  window.__TAURI__ ? tauriTransport : httpTransport;
```

---

## Architecture Decisions

### Decision 1: Axum for HTTP Server

**Options Considered:**
| Framework | Pros | Cons |
|-----------|------|------|
| Axum | Type-safe, async, Tokio-native | Newer |
| Actix-Web | Battle-tested, fast | Actor model complexity |
| Warp | Composable filters | Learning curve |
| Rocket | Familiar syntax | Requires nightly (less now) |

**Decision:** Axum

**Rationale:**
1. Already using Tokio runtime (sqlx dependency)
2. Type-safe extractors match our types.rs pattern
3. Minimal boilerplate (~100 lines for all routes)
4. Same async model as existing code

### Decision 2: Single Binary, Multiple Modes

**Options Considered:**
1. Separate binaries: `context-os-cli`, `context-os-server`
2. Single binary with subcommands: `context-os serve`, `context-os query`
3. Library + separate server crate

**Decision:** Single binary with subcommands (Option 2)

**Rationale:**
1. Already have CLI subcommands (query flex, query chains, etc.)
2. Adding `serve` subcommand is natural extension
3. Single binary simplifies distribution
4. Shared QueryEngine initialization code

**CLI Structure:**
```
context-os
├── query
│   ├── flex --time 7d --limit 50
│   ├── chains --limit 20
│   ├── timeline --time 7d
│   └── sessions --time 7d
└── serve                          ← NEW
    ├── --port 3001 (default)
    ├── --host 127.0.0.1 (default)
    └── --cors (enable for browser)
```

### Decision 3: CORS Configuration

**For Development:**
- Allow `http://localhost:5173` (Vite dev server)
- Allow `http://localhost:1420` (Tauri dev)

**For Production:**
- Disabled by default (Tauri doesn't need HTTP)
- Explicitly enabled via `--cors` flag

### Decision 4: No Authentication (Dev Mode Only)

**Rationale:**
1. HTTP API is for local development only
2. Binds to 127.0.0.1 (localhost only)
3. No network exposure
4. Production uses Tauri IPC (no HTTP)

**Future:** If remote access needed, add API key auth later.

---

## Five-Minute Rule Analysis (Caching)

**Question:** Should we add caching to the HTTP API?

**Analysis:**
```
Current query latency: 1.5ms (measured, from context package 12)
HTTP overhead: ~10ms
Total: ~11.5ms

Target: <100ms

Verdict: 11.5ms << 100ms. NO CACHING NEEDED.
```

**If latency becomes an issue later:**
- Add response caching with 5-second TTL
- Invalidate on write operations
- But this is premature optimization now

---

## Implementation Phases

### Phase 1: HTTP Server Foundation (2-3 hours)

**Files to Create/Modify:**
```
apps/context-os/core/
├── Cargo.toml                 # Add axum, tower-http
└── src/
    ├── http.rs                # NEW: HTTP types, routes
    └── main.rs                # ADD: serve subcommand
```

**Dependencies to Add:**
```toml
[dependencies]
axum = "0.7"
tower-http = { version = "0.5", features = ["cors"] }
```

**Success Criteria:**
- `context-os serve --port 3001` starts HTTP server
- `curl http://localhost:3001/api/health` returns 200
- All 4 query endpoints return same data as CLI

### Phase 2: Frontend Transport Abstraction (1-2 hours)

**Files to Create/Modify:**
```
apps/tastematter/src/lib/
├── api/
│   ├── transport.ts           # NEW: Transport interface
│   ├── tauri.ts               # REFACTOR: Extract from current
│   └── http.ts                # NEW: HTTP implementation
└── stores/
    ├── files.svelte.ts        # UPDATE: Use transport
    ├── timeline.svelte.ts     # UPDATE: Use transport
    ├── workstream.svelte.ts   # UPDATE: Use transport
    └── context.svelte.ts      # UPDATE: Use transport
```

**Success Criteria:**
- `npm run dev` works in browser (no Tauri)
- All views load data via HTTP API
- No code changes to components (only stores)

### Phase 3: Browser Dev Mode (1 hour)

**Configuration:**
```typescript
// vite.config.ts
export default defineConfig({
  server: {
    proxy: {
      '/api': 'http://localhost:3001'
    }
  }
});
```

**Success Criteria:**
- `npm run dev` + `context-os serve` = full app in Chrome
- Claude Code browser automation can control it
- DevTools work normally

### Phase 4: Fix Data Issues (30 min)

With HTTP API in place, fix the hardcoded limits:

| File | Line | Current | Fix |
|------|------|---------|-----|
| `files.svelte.ts` | 34 | `limit: 50` | Remove or increase to 500 |
| `workstream.svelte.ts` | 75 | `limit: 50` | Remove or increase to 500 |
| `context.svelte.ts` | 33 | `limit: 50` | Remove or increase to 200 |
| `WorkstreamView.svelte` | 30 | `limit: 100` | Remove or increase to 500 |

**Validation:** Use browser dev mode to verify all data loads.

---

## Testing Strategy

### Unit Tests (existing)
- QueryEngine tests remain unchanged
- Type serialization tests remain unchanged

### Integration Tests (add)
```rust
// tests/http_integration_test.rs

#[tokio::test]
async fn test_http_query_flex() {
    // Start server
    let server = spawn_test_server().await;

    // Make HTTP request
    let resp = reqwest::Client::new()
        .post(&format!("{}/api/query/flex", server.url()))
        .json(&QueryFlexInput { time: Some("7d".into()), ..Default::default() })
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let result: QueryResult = resp.json().await.unwrap();
    assert!(result.result_count > 0);
}
```

### E2E Tests (browser)
```typescript
// tests/e2e/browser.spec.ts (Playwright)

test('files view loads in browser', async ({ page }) => {
  await page.goto('http://localhost:5173');
  await expect(page.locator('[data-testid="files-count"]')).toBeVisible();
  const count = await page.locator('[data-testid="files-count"]').textContent();
  expect(parseInt(count)).toBeGreaterThan(0);
});
```

---

## Rollback Strategy

If HTTP API causes issues:

1. **Frontend:** Remove transport abstraction, revert to direct Tauri calls
2. **Backend:** Remove `serve` subcommand from main.rs
3. **Dependencies:** Remove axum, tower-http from Cargo.toml

**Risk Assessment:** Low. HTTP API is additive, doesn't modify existing Tauri paths.

---

## Success Metrics

| Metric | Target | How to Measure |
|--------|--------|----------------|
| HTTP latency | <100ms | `time curl localhost:3001/api/query/flex` |
| Dev startup | <5s | Time from `npm run dev` to first paint |
| Browser parity | 100% | Same data in browser vs Tauri |
| Test coverage | HTTP routes | All 4 endpoints tested |

---

## Future Extensions

### Web Deployment (PWA Mode)

With HTTP API in place, web deployment becomes possible:

1. User visits web app
2. Prompted to select database directory (File System Access API)
3. Permission granted, stored
4. App connects to remote/local context-os server
5. Full functionality in browser

**Not in scope for this spec.** Document for future reference.

### Remote Access

If needed later:
1. Add API key authentication
2. Add TLS (HTTPS)
3. Allow non-localhost binding
4. Rate limiting

**Not in scope for this spec.**

---

## References

- [[01_PRINCIPLES.md]] - Latency target <100ms
- [[03_CORE_ARCHITECTURE.md]] - Existing architecture
- [[types.rs]] - Type contracts
- [[main.rs]] - CLI structure

**External:**
- [Axum Documentation](https://docs.rs/axum)
- [Tauri IPC Documentation](https://v2.tauri.app/develop/calling-rust/)
- [File System Access API](https://developer.mozilla.org/en-US/docs/Web/API/File_System_Access_API)

---

## Appendix A: Full HTTP API Reference

### Health Check

```
GET /api/health

Response 200:
{
  "status": "ok",
  "version": "0.1.0",
  "database": "connected",
  "uptime_seconds": 3600
}
```

### Query Flex

```
POST /api/query/flex
Content-Type: application/json

Request:
{
  "time": "7d",
  "limit": 50,
  "chain": null,
  "files": null,
  "session": null,
  "agg": ["count"],
  "sort": "count"
}

Response 200: QueryResult (see types.rs)
Response 400: ApiError
Response 500: ApiError
```

### Query Timeline

```
POST /api/query/timeline
Content-Type: application/json

Request:
{
  "time": "7d",
  "chain": null,
  "files": null,
  "limit": 30
}

Response 200: TimelineData (see types.rs)
```

### Query Sessions

```
POST /api/query/sessions
Content-Type: application/json

Request:
{
  "time": "7d",
  "chain": null,
  "limit": 100
}

Response 200: SessionQueryResult (see types.rs)
```

### Query Chains

```
POST /api/query/chains
Content-Type: application/json

Request:
{
  "limit": 20
}

Response 200: ChainQueryResult (see types.rs)
```

---

**Spec Version:** 1.0
**Last Updated:** 2026-01-09
