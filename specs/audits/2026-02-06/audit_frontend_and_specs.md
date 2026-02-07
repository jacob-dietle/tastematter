# Tastematter Audit: Frontend & Spec Alignment

**Date:** 2026-02-06
**Auditor:** frontend-and-specs agent (Task #3)
**Scope:** All Tauri backend code, Svelte frontend (components, stores, API, types, config, logging, utils, tests), and all 15 canonical specs

---

## Part A: Frontend Audit

### A1. Tauri Command Inventory (Backend Bridge)

8 commands registered in `lib.rs:66-75`:

| Command | File:Line | Input | Output | Purpose |
|---------|-----------|-------|--------|---------|
| `query_flex` | commands.rs:56 | files, time, chain, session, agg, limit, sort | `QueryResult` | Hypercube flex query (files view) |
| `query_timeline` | commands.rs:87 | time, files, chain, limit | `TimelineData` | Per-day activity buckets (timeline view) |
| `query_sessions` | commands.rs:111 | time, chain, limit | `CoreSessionQueryResult` | Sessions with chain info |
| `query_chains` | commands.rs:134 | limit | `CoreChainQueryResult` | List conversation chains |
| `git_status` | commands.rs:179 | none | `GitStatus` | Git branch, ahead/behind, files |
| `git_pull` | commands.rs:242 | none | `GitOpResult` | Git pull --ff-only |
| `git_push` | commands.rs:290 | none | `GitOpResult` | Git push |
| `log_event` | commands.rs:18 | LogEvent | `()` | Frontend structured logging |

**Architecture:** All query commands go through `AppState.get_query_engine()` which lazily initializes a `QueryEngine` from `context_os_core` crate using the canonical database path (`~/.context-os/context_os_events.db`). Git commands use `std::process::Command` subprocess calls. The `log_event` command writes JSONL to `~/.tastematter/logs/dev-YYYY-MM-DD.jsonl`.

### A2. Frontend Query Map (Component -> Query -> Data)

```
App.svelte
  ├── onMount -> ctx.refreshChains() -> queryChains()     [sidebar ChainNav]
  ├── onMount -> filesStore.fetch() -> queryFlex()         [main content]
  ├── $effect(timeRange, selectedChain) -> filesStore.fetch()
  ├── $effect(activeView === 'timeline') -> timelineStore.fetch() -> queryTimeline()
  └── Views:
      ├── 'files'    -> QueryResults [uses filesStore.data]
      ├── 'timeline' -> TimelineView [uses timelineStore]
      └── 'sessions' -> WorkstreamView [calls querySessions() directly]

Sidebar:
  ├── GitPanel -> createGitStore() -> gitStatus(), gitPull(), gitPush()
  └── ChainNav -> ctx (chains from context store)
```

### A3. Component Inventory (23 components)

| Component | Purpose | Data Source | Status |
|-----------|---------|-------------|--------|
| **App.svelte** | Main shell: header, 3-view toggle, sidebar layout | context/files/timeline stores | Complete |
| **ChainBadge.svelte** | Colored pill showing truncated chain ID | chainId prop | Complete |
| **ChainNav.svelte** | Sidebar chain list with select/deselect filtering | context store (chains) | Complete |
| **ErrorDisplay.svelte** | Error card with code, message, details, retry | CommandError prop | Complete |
| **GitActions.svelte** | Pull/Push buttons with loading states | git store props | Complete |
| **GitFileList.svelte** | Collapsible file list (staged/modified/untracked) | files array prop | Complete |
| **GitPanel.svelte** | Full git panel: status badge, branch, file lists, actions | git store | Complete |
| **GitStatusBadge.svelte** | Shows ahead/behind/synced counts | ahead/behind props | Complete |
| **GranularityToggle.svelte** | Directory/File toggle for heatmap | bindable granularity | Complete |
| **HeatMap.svelte** | Heat map with directory drill-down | FileResult[] + granularity | Complete |
| **HeatMapRow.svelte** | Single heat map row with color interpolation | label, count, max props | Complete |
| **LoadingSpinner.svelte** | CSS spinner | none | Complete |
| **QueryResults.svelte** | Container: view mode toggle + granularity + table/heatmap | QueryResult data | Complete |
| **SessionCard.svelte** | Card showing session info, chain badge, files | SessionData + callbacks | Complete |
| **SessionFilePreview.svelte** | Compact file preview with color dots | SessionFile[] top_files | Complete |
| **SessionFileTree.svelte** | Expandable directory tree within session | SessionFile[] all files | Complete |
| **TableView.svelte** | Simple table: file path, access count, last access | FileResult[] | Complete |
| **TimelineAxis.svelte** | Day axis: month, day-of-week, date labels | TimeBucket[] | Complete |
| **TimelineLegend.svelte** | Color legend: Low -> High activity | COLORS constant | Complete |
| **TimelineRow.svelte** | Per-file row of heat cells across dates | filePath, dates, buckets | Complete |
| **TimelineView.svelte** | Full timeline: axis + rows + tooltip + legend | TimelineStore | Complete |
| **TimeRangeToggle.svelte** | 7d/14d/30d selector buttons | selected, options, onchange | Complete |
| **ViewModeToggle.svelte** | Table/Heat Map toggle | bindable mode | Complete |
| **WorkstreamView.svelte** | Sessions list with chain filtering, summary stats | querySessions() direct | Complete |

### A4. State Management Architecture

**6 stores**, factory-function pattern using Svelte 5 `$state` runes:

| Store | File | State | Depends On | Used By |
|-------|------|-------|------------|---------|
| **context** | context.svelte.ts | timeRange, selectedChain, chains[], chainsLoading/Error | queryChains API | App, ChainNav, WorkstreamView, files, timeline, workstream stores |
| **files** | files.svelte.ts | data (QueryResult), loading, error, sort, granularity | context store + queryFlex API | App -> QueryResults |
| **timeline** | timeline.svelte.ts | data (TimelineData), loading, error, hoveredCell | context store (optional) + queryTimeline API | App -> TimelineView |
| **git** | git.svelte.ts | data (GitStatus), loading, error, isPulling, isPushing, lastOperation | gitStatus/Pull/Push API | GitPanel |
| **query** | query.svelte.ts | data (QueryResult), loading, error, lastQuery | queryFlex API | **UNUSED** (legacy, superseded by files store) |
| **workstream** | workstream.svelte.ts | sessionsByChain, sessionsLoading, expandedChains/Sessions | context store + querySessions API | **UNUSED** (WorkstreamView uses inline state instead) |

**Data flow pattern:**
1. `App.svelte` creates context store + files/timeline stores
2. Context store is set via Svelte context API (`setContext`/`getContext`)
3. Stores use request deduplication (incrementing requestId, stale response ignored)
4. Reactivity: `$effect` in App.svelte re-fetches when `ctx.timeRange` or `ctx.selectedChain` changes

### A5. API / Transport Layer

**Dual transport architecture (Spec 04):**

| File | Purpose |
|------|---------|
| `api/index.ts` | Re-exports convenience functions, auto-selects transport |
| `api/transport.ts` | Transport interface + auto-detection (`__TAURI__` check) |
| `api/tauri-transport.ts` | Tauri IPC via `@tauri-apps/api/core` invoke, with logging |
| `api/tauri.ts` | Direct Tauri invoke (also used for git ops, has duplicate invokeLogged) |
| `api/http-transport.ts` | HTTP POST with timeout, for browser dev mode |

**Issue: Code duplication.** `tauri.ts` and `tauri-transport.ts` contain duplicate `invokeLogged`, `sanitizeArgs`, and `summarizeResult` functions. `tauri.ts` exists as a legacy path; `tauri-transport.ts` is the newer transport-pattern version. Git operations (`gitStatus`, `gitPull`, `gitPush`) are only exported from `tauri.ts` and re-exported by `api/index.ts`.

### A6. Config

| File | Contents |
|------|----------|
| `config/api.ts` | API_ENDPOINTS (5 endpoints), REQUEST_TIMEOUT_MS (30s) |
| `config/queries.ts` | QUERY_LIMITS: chains=50, files=50, timeline=30, sessions=50, default=100 |
| `config/index.ts` | Re-exports both |

### A7. Types

`types/index.ts` (294 lines) defines all IPC types. Clean, well-organized with phase comments (Phase 2: Heat Map, Phase 3: Git, Phase 4: Timeline, Phase 5: Sessions). Includes both data types and state interface types. Some state interfaces (`QueryState`, `GitState`, `TimelineState`, `SessionState`, `ChainState`) are defined but unused -- stores use factory functions returning inferred types instead.

### A8. Logging

**Frontend:** `logging/service.ts` - LogServiceImpl class, singleton export. Uses `invoke('log_event')` to send structured events to Tauri backend. Falls back to console.error if IPC fails. Features: correlation IDs, level filtering, argument sanitization.

**Backend (Tauri):** `src-tauri/src/logging/service.rs` - Writes JSONL to `~/.tastematter/logs/dev-YYYY-MM-DD.jsonl`. Daily rotation, append mode.

### A9. Utils

| File | Functions | Used By |
|------|-----------|---------|
| `utils/aggregation.ts` | `getParentDirectory`, `maxDate`, `calculateIntensity`, `aggregateByDirectory` | HeatMap, HeatMapRow |
| `utils/colors.ts` | `COLORS` (ink & paper palette), `getHeatColor`, `calculateIntensity` | TimelineLegend, TimelineRow |

**Issue: Duplicate `calculateIntensity`.** Defined in both `aggregation.ts` and `colors.ts` with identical logic.

### A10. Missing Features / TODOs / Stubs

1. **`handleFileClick` in WorkstreamView** (line 91): `console.log` only, no navigation
2. **`WorkstreamView.handleChainClick`** uses `ctx.setSelectedChain()` which doesn't exist on context store -- store exposes `selectChain()` instead. This is likely a bug (line 96).
3. **E2E test** checks for `90d` button but frontend only offers `7d/14d/30d` options
4. **`query.svelte.ts`** store is unused (superseded by `files.svelte.ts`)
5. **`workstream.svelte.ts`** store is unused (`WorkstreamView` manages its own state inline)
6. **No multi-repo support** (Roadmap Phase 2)
7. **No agent UI control protocol** (Roadmap Phase 3)
8. **No real-time file watcher / live updates** (Roadmap Phase 4)
9. **No commit history display** (Roadmap Phase 1: Stigmergic Display)

### A11. Test Coverage

**Test setup:** Vitest + happy-dom + `@testing-library/jest-dom`

| Category | Tests | Files |
|----------|-------|-------|
| Unit/Components | 10 files | ChainBadge, SessionCard, SessionFilePreview, SessionFileTree, TimelineAxis, TimelineLegend, TimelineRow, TimelineView, TimeRangeToggle, WorkstreamView |
| Unit/Stores | 7 files | context, files, git, query, timeline, timeline-refactored, workstream |
| Unit/API | 1 file | transport |
| Unit/Utils | 1 file | aggregation |
| E2E | 1 file | query.spec.ts (Playwright, 3 tests) |
| **Total** | **20 test files** | |

**Coverage gaps:**
- No tests for: ErrorDisplay, GitActions, GitFileList, GitPanel, GitStatusBadge, GranularityToggle, HeatMap, HeatMapRow, LoadingSpinner, QueryResults, TableView, ViewModeToggle
- No tests for: logging service, colors utility, config
- E2E tests reference `90d` button that doesn't exist in current UI (test is broken)

---

## Part B: Spec Alignment

### Per-Spec Alignment Table

| Spec | Title | Status | Key Drift |
|------|-------|--------|-----------|
| **00** | Vision | REFERENCE ONLY | Vision doc, not implementable. No code gaps. |
| **01** | Principles | PARTIAL | IMMEDIATE: achieved (<100ms via Rust core). STIGMERGIC: NOT STARTED (no commit timeline, no agent/human diff). MULTI-REPO: NOT STARTED. AGENT-CONTROLLABLE: NOT STARTED. INVESTMENT NOT RENT: achieved (local SQLite). |
| **02** | Roadmap | PARTIAL | Phase 0 COMPLETE. Phase 1-5 NOT STARTED. Roadmap accurately reflects current state. |
| **03** | Core Architecture | PARTIAL | Phase 0a COMPLETE (query engine, types, storage, Tauri integration). Phase 0b DEFERRED (IPC socket replaced by direct Rust CLI). Phase 0c DEFERRED (no event bus, no daemon integration). Cache layer, UI State Machine, Event Bus all deferred. |
| **04** | Transport Architecture | IMPLEMENTED | Transport abstraction works (Tauri IPC + HTTP). Frontend auto-detects environment. Vite proxy for dev mode. Matches spec intent. |
| **05** | Intelligence Layer | NOT STARTED | No Claude Agent SDK integration. No intelligent session naming. No agent commit analysis. No proactive insights. Spec defines TypeScript/Bun runtime -- nothing built. |
| **06** | Rust Port Specification | PARTIAL | Query engine ported (Rust reads). Daemon/indexer NOT ported (Python still writes). JSONL parsing ported to Rust in core (but daemon orchestration still Python). ~25% complete per spec's own estimate. |
| **07** | Claude Code Data Model (v1) | SUPERSEDED | Replaced by 07_V2. Contains inaccuracies (16+ types claim, actually 7). |
| **07_V2** | Claude Code Data Model (v2) | REFERENCE | Ground-truth data model spec. No direct code implementation -- serves as reference for parser/indexer code. Correctly identifies 7 record types, 7 linking mechanisms. |
| **08** | Python Port Inventory | REFERENCE | Inventory of Python CLI code for porting. ~5,887 lines need porting (Capture + Index + Daemon). Query Engine + Database already ported. |
| **09** | Rust Port Type Contracts | PARTIAL | Git sync types defined but NOT implemented in Rust. Python types catalogued for porting reference. |
| **10** | MCP Publishing Architecture | NOT STARTED | Phase 5 of roadmap. No MCP server generation, auth, or pay-walling. Draft status. |
| **11** | GitOps Decision Agent Spec | NOT STARTED | No signal collection, no TypeScript agent, no intelligent commit suggestions. Draft status. |
| **12** | Context Restoration API Spec | NOT STARTED | No context restoration API. No LLM synthesis integration. Draft status. |
| **13** | Heat Data Quality Spec | PARTIAL | Identifies noise problem (79% signal is snapshots). Fix requires changes in `jsonl_parser.rs` aggregate_session() to separate signal types. Code audit confirms the problem exists. |

### Unspecified Code (Code Not Described by Any Spec)

1. **Frontend logging system** (frontend `logging/` + Tauri `logging/` module) -- JSONL structured logging with correlation IDs, not described in any canonical spec (though Spec 03 Decision 6 mentions correlation ID logging as deferred)
2. **`api/tauri.ts` legacy direct-invoke path** -- exists alongside the transport-pattern `tauri-transport.ts`, creating duplication. No spec describes having two Tauri API layers.
3. **Git operations** (git_status, git_pull, git_push) are implemented in Tauri commands but their spec coverage is limited to the GitOps Decision Agent Spec (11) which is NOT STARTED. The current implementation is simpler than the spec envisions.
4. **`WorkstreamView` inline state management** -- diverges from the `workstream.svelte.ts` store pattern that exists but is unused.
5. **Ink & Paper color palette** (`utils/colors.ts`) -- the "aged document aesthetic" theming is not described in any spec.

---

## Summary of Gaps

### Critical Issues

1. **Bug: `WorkstreamView` calls `ctx.setSelectedChain()` but context store exposes `selectChain()`** -- This is likely a runtime error. The store also exposes a `setTimeRange()` method, creating naming inconsistency. (WorkstreamView.svelte:96)

2. **Duplicate code: `tauri.ts` vs `tauri-transport.ts`** -- `invokeLogged`, `sanitizeArgs`, `summarizeResult` are copy-pasted across both files. Only `tauri.ts` handles git operations.

3. **Duplicate function: `calculateIntensity`** exists in both `utils/aggregation.ts` and `utils/colors.ts` with identical logic.

4. **E2E test broken** -- `query.spec.ts:13` expects a `90d` button but the app only renders `7d/14d/30d`.

### Dead Code

1. **`query.svelte.ts`** -- Legacy store, superseded by `files.svelte.ts`. Has tests (`query.test.ts`) but nothing imports it at runtime.
2. **`workstream.svelte.ts`** -- Full-featured store with lazy loading, chain expand/collapse. Has tests (`workstream.test.ts`) but `WorkstreamView` component manages state inline instead.
3. **State interfaces** in `types/index.ts` (`QueryState`, `GitState`, `TimelineState`, `SessionState`, `ChainState`) -- defined but never imported.

### Dead Specs

1. **07_CLAUDE_CODE_DATA_MODEL.md** (v1) -- Superseded by V2. Should be archived or deleted.

### Spec Coverage Heat Map

```
Implemented:  00, 01(partial), 02(partial), 03(Phase0a only), 04
Not Started:  05, 10, 11, 12
Reference:    07_V2, 08
Partial:      01, 02, 03, 06, 09, 13
Dead:         07(v1)
```

### Architectural Observations

1. **Transport layer is well-designed.** The `Transport` interface + auto-detection pattern enables browser dev + Tauri production cleanly.

2. **Store architecture is clean but has orphans.** The context/files/timeline/git stores follow a consistent factory-function pattern with request deduplication. But `query` and `workstream` stores exist as unused code.

3. **Tauri backend is thin and correct.** Commands are pure wrappers around `context_os_core::QueryEngine`, with lazy initialization via `OnceCell`. No business logic in the Tauri layer.

4. **Missing the intelligence layer entirely.** Specs 05, 11, 12 define a TypeScript/Bun intelligence layer with Claude Agent SDK. Zero code exists for this. This is the biggest gap between specs and code.

5. **Frontend is production-quality for what it does** (data visualization) but missing the coordination/agent features that define the product vision (stigmergic display, multi-repo, agent control).
