---
title: "Tastematter Context Package 17"
package_number: 17

migrated_from: "apps/tastematter/specs/context_packages/17_2026-01-09_TRANSPORT_ARCHITECTURE_SPEC.md"
status: current
previous_package: "[[16_2026-01-09_ARCHITECTURE_DOC_UPDATE]]"
related:
  - "[[apps/tastematter/specs/canonical/04_TRANSPORT_ARCHITECTURE.md]]"
  - "[[apps/tastematter/specs/implementation/phase_03_http_api/01_HTTP_SERVER_AGENT_TASK.md]]"
  - "[[apps/tastematter/specs/implementation/phase_03_http_api/02_FRONTEND_TRANSPORT_AGENT_TASK.md]]"
  - "[[apps/context-os/core/src/types.rs]]"
tags:
  - context-package
  - tastematter
  - transport-architecture
  - http-api
  - specification
---

# Tastematter - Context Package 17

## Executive Summary

Created comprehensive transport-agnostic architecture specification. Enables browser-based development with HTTP API while keeping Tauri for production. Full agent task specs written for Phase 3 implementation (~4-6 hours total work).

## Global Context

### Problem Solved

**Pain points identified:**
1. Tauri app has arbitrary data caps (100 sessions, 50 files)
2. No automation - Claude Code browser automation doesn't work with Tauri
3. Slow dev iteration - must rebuild Tauri to test frontend changes
4. No E2E testing - Playwright requires complex Tauri-specific setup

**Solution:** Transport-agnostic architecture

```
                    ┌─────────────────────────────────────┐
                    │        context-os-core              │
                    │  ┌────────────────────────────────┐ │
                    │  │      QueryEngine (Rust)        │ │
                    │  └────────────────────────────────┘ │
                    │              ▲                      │
                    │    ┌─────────┴─────────┐           │
                    │    │                   │           │
                    │  ┌─┴─┐              ┌──┴──┐        │
                    │  │CLI│              │HTTP │  ← NEW │
                    │  └───┘              └─────┘        │
                    └─────────────────────────────────────┘
                              ▲                ▲
              ┌───────────────┤                │
     ┌────────▼────────┐ ┌────▼─────┐   ┌──────▼──────┐
     │  Tauri Desktop  │ │ Browser  │   │ Claude Code │
     │  (Production)   │ │ Dev Mode │   │ Automation  │
     └─────────────────┘ └──────────┘   └─────────────┘
```

### Key Design Decisions

1. **Axum for HTTP server** - Tokio-native, matches existing async runtime [VERIFIED: [[04_TRANSPORT_ARCHITECTURE.md]]:Decision 1]
2. **Single binary, multiple modes** - `context-os serve` vs `context-os query` [VERIFIED: [[04_TRANSPORT_ARCHITECTURE.md]]:Decision 2]
3. **No caching needed** - Query latency already <2ms [VERIFIED: [[04_TRANSPORT_ARCHITECTURE.md]]:Five-Minute Rule Analysis]
4. **HTTP adds ~10ms overhead** - Still under 100ms budget [VERIFIED: [[04_TRANSPORT_ARCHITECTURE.md]]:Latency Budget]

## Local Problem Set

### Completed This Session

- [X] Created canonical spec: 04_TRANSPORT_ARCHITECTURE.md [VERIFIED: file exists, ~400 lines]
- [X] Created agent task spec: 01_HTTP_SERVER_AGENT_TASK.md [VERIFIED: file exists, ~350 lines]
- [X] Created agent task spec: 02_FRONTEND_TRANSPORT_AGENT_TASK.md [VERIFIED: file exists, ~300 lines]
- [X] Updated implementation README with Phase 3 [VERIFIED: [[README.md]] updated]
- [X] Identified hardcoded limits in frontend stores [VERIFIED: grep output]

### Hardcoded Limits Found

| File | Line | Limit | Issue |
|------|------|-------|-------|
| `files.svelte.ts` | 34 | 50 | Files capped |
| `workstream.svelte.ts` | 75 | 50 | Workstream capped |
| `context.svelte.ts` | 33 | 50 | Chains capped |
| `WorkstreamView.svelte` | 30 | 100 | Sessions capped |

### In Progress

None - specification complete. Ready for implementation.

### Jobs To Be Done (Next Session)

**Phase 3.1: HTTP Server** (2-3 hours)
1. [ ] Add axum + tower-http to Cargo.toml
   - File: `apps/context-os/core/Cargo.toml`
   - Deps: `axum = "0.7"`, `tower-http = { version = "0.5", features = ["cors"] }`

2. [ ] Create http.rs module
   - File: `apps/context-os/core/src/http.rs` (NEW)
   - ~120 lines: routes, handlers, types
   - Success: All 4 query endpoints over HTTP

3. [ ] Add `serve` subcommand to CLI
   - File: `apps/context-os/core/src/main.rs`
   - Command: `context-os serve --port 3001 --cors`

4. [ ] Write integration tests
   - File: `apps/context-os/core/tests/http_integration_test.rs` (NEW)
   - Test all endpoints return correct data

**Phase 3.2: Frontend Transport** (1-2 hours)
5. [ ] Create transport abstraction
   - Files: `src/lib/api/transport.ts`, `tauri-transport.ts`, `http-transport.ts`
   - Auto-detect Tauri vs browser

6. [ ] Configure Vite proxy
   - File: `vite.config.ts`
   - Proxy `/api/*` to `localhost:3001`

7. [ ] Update stores to use transport
   - Files: `files.svelte.ts`, `timeline.svelte.ts`, `workstream.svelte.ts`, `context.svelte.ts`
   - Just import path changes

**Phase 3.3: Fix Limits** (30 min)
8. [ ] Remove hardcoded 50/100 limits from stores

## File Locations

| File | Purpose | Status |
|------|---------|--------|
| [[apps/tastematter/specs/canonical/04_TRANSPORT_ARCHITECTURE.md]] | Canonical architecture spec | CREATED |
| [[apps/tastematter/specs/implementation/phase_03_http_api/01_HTTP_SERVER_AGENT_TASK.md]] | Backend agent task | CREATED |
| [[apps/tastematter/specs/implementation/phase_03_http_api/02_FRONTEND_TRANSPORT_AGENT_TASK.md]] | Frontend agent task | CREATED |
| [[apps/tastematter/specs/implementation/README.md]] | Implementation index | UPDATED |
| [[apps/context-os/core/src/types.rs]] | Type contracts (unchanged) | REFERENCE |

## Test State

- **Core tests:** 8 integration passing [VERIFIED: cargo test 2026-01-09]
- **CLI:** Working [VERIFIED: `tastematter query flex --time 7d` returns data]
- **Tauri:** Not tested this session (specs only)

### Test Commands for Next Agent

```bash
# Verify core tests pass before starting
cd apps/context-os/core && cargo test

# After Phase 3.1: Test HTTP server
context-os serve --port 3001 --cors &
curl http://localhost:3001/api/health
curl -X POST http://localhost:3001/api/query/flex \
  -H "Content-Type: application/json" \
  -d '{"time": "7d", "limit": 5}'

# After Phase 3.2: Test browser mode
npm run dev  # In apps/tastematter
# Open http://localhost:5173 in browser
# Check console for "[transport] Using HTTP API"
```

## Vision Roadmap Status

| Phase | Name | Status |
|-------|------|--------|
| 0 | Performance Foundation | ✅ COMPLETE |
| 1 | Stigmergic Display | NOT STARTED |
| 2 | Multi-Repo Dashboard | NOT STARTED |
| 3 | Agent UI Control | NOT STARTED |
| 4 | Intelligent GitOps | NOT STARTED |

**New: Phase 3 HTTP API** inserted before Phase 1 Stigmergic Display to unblock dev tooling.

## For Next Agent

**Context Chain:**
- Previous: [[16_2026-01-09_ARCHITECTURE_DOC_UPDATE]] (Phase 0 complete, docs updated)
- This package: Transport architecture specification complete
- Next action: Implement Phase 3.1 HTTP Server

**Start here:**
1. Read this context package (you're doing it now)
2. Read [[04_TRANSPORT_ARCHITECTURE.md]] for full architecture spec
3. Read [[01_HTTP_SERVER_AGENT_TASK.md]] for step-by-step implementation
4. Run: `cd apps/context-os/core && cargo test` to verify baseline

**Do NOT:**
- Modify existing Tauri IPC code (add HTTP alongside, not replace)
- Change types.rs (HTTP uses exact same types)
- Add caching (latency is already <2ms)
- Add authentication (localhost dev only)

**Key insight:**
The QueryEngine already exists and works. This is just adding a second transport layer (HTTP) alongside the existing CLI. Same functions, different access method. The frontend abstraction auto-detects environment and uses the right transport.

**Estimated implementation time:**
- Phase 3.1 (HTTP Server): 2-3 hours
- Phase 3.2 (Frontend): 1-2 hours
- Phase 3.3 (Fix Limits): 30 min
- **Total: 4-6 hours**

**Skills used this session:**
- `technical-architecture-engineering` - Latency budget, IPC pattern selection
- `specification-driven-development` - Agent task specs, type contracts
- `observability-engineering` - Thought through debugging approaches

[VERIFIED: All specs written to `apps/tastematter/specs/`]
