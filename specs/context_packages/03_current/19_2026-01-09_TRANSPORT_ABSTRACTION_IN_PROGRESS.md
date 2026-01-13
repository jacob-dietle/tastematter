---
title: "Tastematter Context Package 19"
package_number: 19

migrated_from: "apps/tastematter/specs/context_packages/19_2026-01-09_TRANSPORT_ABSTRACTION_IN_PROGRESS.md"
status: current
previous_package: "[[18_2026-01-09_HTTP_SERVER_COMPLETE]]"
related:
  - "[[apps/tastematter/src/lib/api/transport.ts]]"
  - "[[apps/tastematter/src/lib/api/http-transport.ts]]"
  - "[[apps/tastematter/src/lib/api/tauri-transport.ts]]"
  - "[[apps/tastematter/tests/unit/api/transport.test.ts]]"
  - "[[apps/tastematter/vite.config.ts]]"
tags:
  - context-package
  - tastematter
  - transport-abstraction
  - tdd
  - in-progress
---

# Tastematter - Context Package 19

## Executive Summary

**Phase 3.2 Frontend Transport: IN PROGRESS.** Implemented transport abstraction using TDD - created interface, HTTP transport, Tauri transport, and 10 passing transport tests. Configured Vite proxy. Started updating stores but **test mocks need updating** from `$lib/api/tauri` to `$lib/api`.

## Global Context

### Architecture Achievement (from Package 18)

Transport-agnostic QueryEngine accessible via CLI, Tauri IPC, and HTTP:

```
                    ┌─────────────────────────────────────┐
                    │        context-os-core              │
                    │  ┌────────────────────────────────┐ │
                    │  │      QueryEngine (Rust)        │ │
                    │  └────────────────────────────────┘ │
                    │              ▲                      │
                    │    ┌─────────┼─────────┐           │
                    │    │         │         │           │
                    │  ┌─┴─┐   ┌───┴───┐  ┌──┴──┐       │
                    │  │CLI│   │Tauri  │  │HTTP │       │
                    │  └───┘   │IPC    │  └─────┘       │
                    │          └───────┘                 │
                    └─────────────────────────────────────┘
                                   │
                    ┌──────────────┴──────────────┐
                    │   Frontend Transport Layer   │  ← THIS SESSION
                    │  ┌────────────────────────┐ │
                    │  │    Transport Interface  │ │
                    │  │  ┌──────┐  ┌─────────┐ │ │
                    │  │  │Tauri │  │  HTTP   │ │ │
                    │  │  │Trans │  │ Trans   │ │ │
                    │  │  └──────┘  └─────────┘ │ │
                    │  └────────────────────────┘ │
                    └─────────────────────────────┘
```

## Local Problem Set

### Completed This Session

- [X] Created transport interface (`Transport` type) [VERIFIED: transport.ts:20-27]
- [X] Created `isTauriEnvironment()` detection [VERIFIED: transport.ts:35-41]
- [X] Created `createTransport()` factory with caching [VERIFIED: transport.ts:55-72]
- [X] Created `initializeTransport()` async for Tauri [VERIFIED: transport.ts:78-103]
- [X] Created `http-transport.ts` with fetch calls [VERIFIED: http-transport.ts created]
- [X] Created `tauri-transport.ts` with invoke calls [VERIFIED: tauri-transport.ts created]
- [X] Created `api/index.ts` re-exporting convenience functions [VERIFIED: index.ts created]
- [X] Wrote 10 transport tests using TDD [VERIFIED: transport.test.ts, all 10 pass]
- [X] Configured Vite proxy `/api` → `localhost:3001` [VERIFIED: vite.config.ts:13-20]
- [X] Updated `query.svelte.ts` import to `$lib/api` [VERIFIED: query.svelte.ts:1]
- [X] Updated `files.test.ts` mock to `$lib/api` [VERIFIED: files.test.ts:14]
- [X] Updated `workstream.test.ts` mock to `$lib/api` [VERIFIED: workstream.test.ts:15]

### In Progress

- [ ] Update remaining test mocks from `$lib/api/tauri` to `$lib/api`
  - **Current state:** 2 test files updated, ~6 remaining
  - **Files needing update:**
    - `tests/unit/stores/context.test.ts` - mock line 14
    - `tests/unit/stores/git.test.ts` - mock line 4
    - `tests/unit/stores/query.test.ts` - mock line 4
    - `tests/unit/stores/timeline.test.ts` - mock line 4
    - `tests/unit/stores/timeline-refactored.test.ts` - mock line 14
    - `tests/unit/components/WorkstreamView.test.ts` - mock line 64
  - **Pattern to apply:**
    ```typescript
    // Change FROM:
    vi.mock('$lib/api/tauri', () => ({
    // Change TO:
    vi.mock('$lib/api', () => ({

    // Also update import:
    // FROM: import { queryFlex } from '$lib/api/tauri';
    // TO:   import { queryFlex } from '$lib/api';
    ```

- [ ] Update remaining store imports (files, timeline, workstream, context)
  - Only `query.svelte.ts` updated so far
  - Pattern: change `from '$lib/api/tauri'` to `from '$lib/api'`

### Jobs To Be Done (Next Session)

**IMMEDIATE - Fix Test Mocks (~15 min):**
1. [ ] Update test mocks in remaining 6 test files
2. [ ] Update store imports in remaining 4 store files
3. [ ] Run `npm run test:unit` - should see 244+ tests pass

**Phase 3.2 Completion (~30 min):**
4. [ ] Verify HTTP transport works in browser dev mode
   - Start: `cargo run --bin context-os -- serve --port 3001 --cors`
   - Start: `npm run dev`
   - Open browser, verify data loads

**Phase 3.3: Fix Limits (~30 min):**
5. [ ] Remove hardcoded 50/100 limits from stores
   - See package 17 for exact file:line locations

## Files Created This Session

| File | Lines | Purpose |
|------|-------|---------|
| `src/lib/api/transport.ts` | ~150 | Transport interface, factory, env detection |
| `src/lib/api/http-transport.ts` | ~70 | HTTP fetch-based transport |
| `src/lib/api/tauri-transport.ts` | ~140 | Tauri invoke-based transport |
| `src/lib/api/index.ts` | ~50 | Re-exports for backwards compatibility |
| `tests/unit/api/transport.test.ts` | ~160 | 10 transport tests |

## Files Modified This Session

| File | Change |
|------|--------|
| `vite.config.ts` | Added proxy config for `/api` |
| `src/lib/stores/query.svelte.ts` | Changed import to `$lib/api` |
| `tests/unit/stores/files.test.ts` | Changed mock to `$lib/api` |
| `tests/unit/stores/workstream.test.ts` | Changed mock to `$lib/api` |

## Test State

**Transport tests:** 10 passing [VERIFIED: npm run test:unit -- transport.test.ts]

**Full suite:** 195 passing, 51 failing
- **Cause:** Test mocks still reference `$lib/api/tauri` but stores now import from `$lib/api`
- **Fix:** Update remaining test mocks (pattern shown above)

### Test Commands for Next Agent

```bash
# Run transport tests only (should pass)
cd apps/tastematter && npm run test:unit -- --run tests/unit/api/transport.test.ts

# Run full suite (will show failures until mocks fixed)
cd apps/tastematter && npm run test:unit -- --run

# After fixing mocks, verify all pass
cd apps/tastematter && npm run test:unit -- --run 2>&1 | tail -10
```

## Transport API Reference

### Interface

```typescript
interface Transport {
  queryFlex(args: QueryFlexArgs): Promise<QueryResult>;
  queryTimeline(args: TimelineQueryArgs): Promise<TimelineData>;
  querySessions(args: SessionQueryArgs): Promise<SessionQueryResult>;
  queryChains(args: ChainQueryArgs): Promise<ChainQueryResult>;
}
```

### Usage

```typescript
// Option 1: Use convenience functions (recommended)
import { queryFlex, queryTimeline } from '$lib/api';
const result = await queryFlex({ time: '7d', agg: ['count'] });

// Option 2: Use transport directly
import { createTransport } from '$lib/api';
const transport = createTransport();
const result = await transport.queryFlex({ time: '7d' });
```

### Environment Detection

```typescript
import { isTauriEnvironment } from '$lib/api';

if (isTauriEnvironment()) {
  // Running in Tauri desktop app - uses IPC
} else {
  // Running in browser - uses HTTP via Vite proxy
}
```

## For Next Agent

**Context Chain:**
- Previous: [[18_2026-01-09_HTTP_SERVER_COMPLETE]] (Phase 3.1 complete)
- This package: Phase 3.2 transport abstraction IN PROGRESS
- Next action: Fix remaining test mocks

**Start here:**
1. Read this context package (you're doing it now)
2. Fix test mocks in remaining 6 files (pattern above)
3. Fix store imports in remaining 4 files
4. Run `npm run test:unit` - verify 244+ tests pass
5. Continue with Phase 3.2 verification

**Files to update (mocks):**
```bash
# Each file needs vi.mock('$lib/api/tauri'...) changed to vi.mock('$lib/api'...)
tests/unit/stores/context.test.ts
tests/unit/stores/git.test.ts
tests/unit/stores/query.test.ts
tests/unit/stores/timeline.test.ts
tests/unit/stores/timeline-refactored.test.ts
tests/unit/components/WorkstreamView.test.ts
```

**Files to update (store imports):**
```bash
# Each file needs import from '$lib/api/tauri' changed to '$lib/api'
src/lib/stores/files.svelte.ts
src/lib/stores/timeline.svelte.ts
src/lib/stores/workstream.svelte.ts
src/lib/stores/context.svelte.ts
```

**Do NOT:**
- Modify transport.ts, http-transport.ts, tauri-transport.ts (already complete)
- Run tests without fixing mocks first (will show false failures)
- Change the git.svelte.ts import (git operations are Tauri-only, keep using tauri.ts)

**Key insight:**
The transport abstraction is COMPLETE. Only the test infrastructure needs updating because tests mock `$lib/api/tauri` but stores now import from `$lib/api`. This is a simple find-replace task.

## Vision Roadmap Status

| Phase | Name | Status |
|-------|------|--------|
| 0 | Performance Foundation | ✅ COMPLETE |
| 3.1 | HTTP Server | ✅ COMPLETE |
| 3.2 | Frontend Transport | 🔄 IN PROGRESS (mocks need updating) |
| 3.3 | Fix Limits | NOT STARTED |
| 1 | Stigmergic Display | NOT STARTED |
| 2 | Multi-Repo Dashboard | NOT STARTED |
| 3 | Agent UI Control | NOT STARTED |
| 4 | Intelligent GitOps | NOT STARTED |

[VERIFIED: All transport files created, 10 transport tests passing, test mock updates in progress]
