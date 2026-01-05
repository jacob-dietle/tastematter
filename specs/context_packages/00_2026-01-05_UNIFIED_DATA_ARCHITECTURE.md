---
title: "Tastematter Context Package 00"
package_number: 0
date: 2026-01-05
status: current
previous_package: null
related:
  - "[[specs/08_UNIFIED_DATA_ARCHITECTURE.md]]"
  - "[[specs/07_CHAIN_INTEGRATION_SPEC.md]]"
  - "[[src/lib/stores/context.svelte.ts]]"
tags:
  - context-package
  - tastematter
  - unified-data-architecture
---

# Tastematter - Context Package 00

## Executive Summary

Implemented unified data architecture for Tastematter based on hypercube model (Spec 08). TDD cycle complete: 92 new tests written, all 246 tests passing, build succeeds. ContextProvider now manages global state (timeRange, selectedChain, chains) with view-specific stores subscribing to it.

## Global Context

### Architecture Overview

Tastematter is a Tauri desktop app (Rust backend + Svelte 5 frontend) that visualizes Claude Code context data. It provides three views (Files, Timeline, Sessions/Workstreams) that are projections of a 5-dimensional hypercube:

```
┌─────────────────────────────────────────────────────────────────┐
│                    CONTEXT HYPERCUBE                            │
├─────────────────────────────────────────────────────────────────┤
│  Dimension 1: FILES      - All file paths ever touched          │
│  Dimension 2: SESSIONS   - All Claude Code sessions (UUIDs)     │
│  Dimension 3: TIME       - Temporal axis (days, weeks, ranges)  │
│  Dimension 4: CHAINS     - Conversation chains (work streams)   │
│  Dimension 5: ACCESS_TYPE - read | write | create               │
└─────────────────────────────────────────────────────────────────┘
```

### Key Design Decisions

1. **Shared ContextProvider** - Single source of truth for global filters [VERIFIED: [[context.svelte.ts]]:1-90]
2. **View stores subscribe to context** - FilesStore, TimelineStore, WorkstreamStore read from ctx [VERIFIED: [[files.svelte.ts]], [[workstream.svelte.ts]]]
3. **Backwards compatibility** - TimelineStore accepts optional context parameter [VERIFIED: [[timeline.svelte.ts]]:12]
4. **ChainNav always visible** - Chain sidebar shows in ALL views, filters ALL views [VERIFIED: [[App.svelte]]:116-119]
5. **Lazy loading** - WorkstreamStore loads sessions per-chain when expanded [VERIFIED: [[workstream.svelte.ts]]:63-80]

### Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                     ContextProvider                              │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │ Global State (single source of truth)                       ││
│  │ • timeRange: '7d' | '14d' | '30d'                          ││
│  │ • selectedChain: string | null                              ││
│  │ • chains: ChainData[]  (always loaded)                     ││
│  └─────────────────────────────────────────────────────────────┘│
│              ┌───────────────┼───────────────┐                  │
│              ▼               ▼               ▼                  │
│  ┌───────────────┐ ┌───────────────┐ ┌───────────────────────┐ │
│  │  FilesStore   │ │ TimelineStore │ │   WorkstreamStore     │ │
│  │  (ctx.time,   │ │ (ctx.time)    │ │ (ctx.chains,          │ │
│  │   ctx.chain)  │ │               │ │  lazy-load sessions)  │ │
│  └───────────────┘ └───────────────┘ └───────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

## Local Problem Set

### Completed This Session

- [X] Wrote spec 08_UNIFIED_DATA_ARCHITECTURE.md [VERIFIED: [[specs/08_UNIFIED_DATA_ARCHITECTURE.md]]]
- [X] TDD RED: Wrote 92 tests for new stores [VERIFIED: test files in tests/unit/stores/]
  - context.test.ts (22 tests)
  - files.test.ts (23 tests)
  - timeline-refactored.test.ts (20 tests)
  - workstream.test.ts (27 tests)
- [X] TDD GREEN: Implemented all stores [VERIFIED: store files in src/lib/stores/]
  - context.svelte.ts (~90 lines)
  - files.svelte.ts (~85 lines)
  - timeline.svelte.ts (refactored, ~108 lines)
  - workstream.svelte.ts (~175 lines)
- [X] Created ChainNav component using context [VERIFIED: [[ChainNav.svelte]]]
- [X] Updated App.svelte to use ContextProvider [VERIFIED: [[App.svelte]]:15-17]
- [X] All 246 tests passing [VERIFIED: pnpm test:unit 2026-01-05]
- [X] Build succeeds [VERIFIED: pnpm build 2026-01-05]

### In Progress

- [ ] Uncommitted changes need to be committed
  - 2 modified files, 9 new files
  - All tests pass, build works

### Jobs To Be Done (Next Session)

1. [ ] Commit the unified data architecture implementation
   - Success criteria: Clean commit with descriptive message

2. [ ] Test the app manually in Tauri
   - Success criteria: Chain selection filters all views, time range is global

3. [ ] Create WorkstreamView component (optional enhancement)
   - Currently using SessionView with chainFilter prop
   - Could create dedicated WorkstreamView using WorkstreamStore
   - Success criteria: Chain → Session → Files hierarchy display

4. [ ] Fix svelte-check configuration issues (low priority)
   - Pre-existing module resolution errors in svelte-check
   - Build and tests work fine, just check command fails
   - Success criteria: `pnpm check` passes

## File Locations

| File | Purpose | Status |
|------|---------|--------|
| [[src/lib/stores/context.svelte.ts]] | Global context provider | New |
| [[src/lib/stores/files.svelte.ts]] | Files view store (uses context) | New |
| [[src/lib/stores/timeline.svelte.ts]] | Timeline store (optional context) | Modified |
| [[src/lib/stores/workstream.svelte.ts]] | Workstream hierarchy store | New |
| [[src/lib/components/ChainNav.svelte]] | Chain navigation using context | New |
| [[src/App.svelte]] | Main app with ContextProvider | Modified |
| [[specs/08_UNIFIED_DATA_ARCHITECTURE.md]] | Architecture specification | New |
| [[tests/unit/stores/context.test.ts]] | Context store tests | New |
| [[tests/unit/stores/files.test.ts]] | Files store tests | New |
| [[tests/unit/stores/timeline-refactored.test.ts]] | Timeline refactored tests | New |
| [[tests/unit/stores/workstream.test.ts]] | Workstream store tests | New |

## Test State

- Tests: **246 passing**, 0 failing
- Command: `pnpm test:unit`
- Last run: 2026-01-05 23:44
- Evidence: [VERIFIED: vitest output showing 19 test files, 246 tests passed]

### Test Commands for Next Agent

```bash
# Verify all tests pass
cd apps/tastematter && pnpm test:unit

# Run specific new store tests
pnpm test:unit -- tests/unit/stores/context.test.ts
pnpm test:unit -- tests/unit/stores/files.test.ts
pnpm test:unit -- tests/unit/stores/workstream.test.ts

# Build check
pnpm build

# Run Tauri dev (if testing UI)
pnpm tauri dev
```

## For Next Agent

**Context Chain:**
- Previous: None (first package for Tastematter)
- This package: Unified data architecture implementation complete
- Next action: Commit changes, test manually in Tauri

**Start here:**
1. Read this context package (you're doing it now)
2. Read [[specs/08_UNIFIED_DATA_ARCHITECTURE.md]] for full architecture spec
3. Run `pnpm test:unit` to verify 246 tests pass
4. Run `git status` to see uncommitted changes

**Do NOT:**
- Edit existing old stores (query.svelte.ts, session.svelte.ts, chain.svelte.ts) - they're kept for backwards compatibility
- Try to fix svelte-check errors - they're pre-existing config issues, tests and build work fine

**Key insight:**
The hypercube model from CLI spec 12 was the foundation for this architecture. Every view is a projection: slice by dimensions (time, chain) then aggregate and render. The ContextProvider owns the slice parameters, view stores handle aggregation/rendering.
[VERIFIED: [[specs/08_UNIFIED_DATA_ARCHITECTURE.md]] and [[12_CLI_HYPERCUBE_SPEC.md]]]
