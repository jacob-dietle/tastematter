---
title: "Tastematter Context Package 03 - Phase 2 In Progress"
package_number: 03

migrated_from: "apps/tastematter/specs/context_packages/03_2026-01-06_PHASE2_IN_PROGRESS.md"
status: current
previous_package: "[[02_2026-01-05_PERF_OPTIMIZATION_HANDOFF]]"
related:
  - "[[10_PERF_OPTIMIZATION_SPEC]]"
  - "[[src/lib/stores/files.svelte.ts]]"
  - "[[src/lib/stores/timeline.svelte.ts]]"
  - "[[src/lib/stores/context.svelte.ts]]"
  - "[[src/lib/components/TimelineRow.svelte]]"
  - "[[src-tauri/src/commands.rs]]"
tags:
  - context-package
  - tastematter
  - performance
  - phase-2
---

# Tastematter - Context Package 03: Phase 2 In Progress

## Executive Summary

Phase 1 complete and committed (3 fixes). Phase 2 started - Fix 4 (request deduplication) partially implemented in files.svelte.ts only.
**Next agent should complete Fix 4 in 2 remaining files, then Fix 5 and Fix 6.**

## Session Accomplishments

### Phase 1 Complete ✅ [VERIFIED: commit 4a2fbfb]

| Fix | File | Change | Status |
|-----|------|--------|--------|
| 1 | App.svelte:47-48 | Remove `\|\| filesStore.error` from hasInitialData | ✅ Done |
| 2 | WorkstreamView.svelte:20 | Add `$derived` for maxAccessCount | ✅ Done |
| 3 | git.svelte.ts:36,57 | Remove `await` from fetchStatus | ✅ Done |

### Phase 2 In Progress

| Fix | File | Status | Notes |
|-----|------|--------|-------|
| 4 | files.svelte.ts | ✅ Done | Request deduplication added |
| 4 | timeline.svelte.ts | ❌ Pending | Same pattern needed |
| 4 | context.svelte.ts | ❌ Pending | Same pattern for refreshChains() |
| 5 | TimelineRow.svelte | ❌ Pending | Date pre-computation |
| 6 | commands.rs | ❌ Pending | Git command consolidation |

### Current Test State

```
Test Files: 18 passed (18)
Tests: 236 passed (236)
```

Command to verify: `cd apps/tastematter && npm test`

## Global Context

### Architecture Overview

```
Tastematter App (Svelte 5 + Tauri)
├── Frontend (Svelte 5 runes)
│   ├── App.svelte - Main orchestrator
│   ├── stores/
│   │   ├── context.svelte.ts - Global state
│   │   ├── files.svelte.ts - Files query (FIX 4 DONE)
│   │   ├── timeline.svelte.ts - Timeline query (FIX 4 NEEDED)
│   │   └── git.svelte.ts - Git operations
│   └── components/
│       ├── TimelineRow.svelte - Heat map row (FIX 5 NEEDED)
│       └── WorkstreamView.svelte - Sessions list
└── Backend (Rust/Tauri)
    └── src-tauri/src/commands.rs - CLI wrapper (FIX 6 NEEDED)
```

## Local Problem Set

### Completed This Session
- [x] Phase 1: All 3 fixes implemented and committed [VERIFIED: commit 4a2fbfb]
- [x] Wrote spec: [[10_PERF_OPTIMIZATION_SPEC]] [VERIFIED: file exists]
- [x] Fix 4 partial: files.svelte.ts request deduplication [VERIFIED: git status shows modified]

### In Progress
- [ ] Fix 4: Request deduplication in timeline.svelte.ts
  - Current state: Pattern established in files.svelte.ts, need to replicate
  - Pattern: Add `currentRequestId` counter, check before updating state

### Jobs To Be Done (Next Session)

**Fix 4: Complete Request Deduplication (~5 min)**

1. [ ] timeline.svelte.ts - Add same pattern as files.svelte.ts
2. [ ] context.svelte.ts - Add pattern to `refreshChains()` function

**Pattern to apply (from files.svelte.ts:21-50):**
```typescript
// Add after state declarations:
let currentRequestId = 0;

// In fetch function:
async function fetch() {
  const requestId = ++currentRequestId;
  loading = true;
  error = null;
  try {
    const result = await apiCall();
    if (requestId === currentRequestId) {
      data = result;
    }
  } catch (e) {
    if (requestId === currentRequestId) {
      error = e as CommandError;
    }
  } finally {
    if (requestId === currentRequestId) {
      loading = false;
    }
  }
}
```

**Fix 5: TimelineRow Date Pre-computation (~10 min)**

File: `src/lib/components/TimelineRow.svelte`
Lines: 29-38

Replace per-cell `isWeekend(date)` and `isToday(date)` calls with pre-computed `$derived`:

```typescript
const todayStr = new Date().toISOString().split('T')[0];

const dateClassifications = $derived(
  dates.reduce((acc, date) => {
    const d = new Date(date);
    const day = d.getDay();
    acc[date] = {
      isWeekend: day === 0 || day === 6,
      isToday: date === todayStr
    };
    return acc;
  }, {} as Record<string, { isWeekend: boolean; isToday: boolean }>)
);
```

Then update template:
- `class:weekend={isWeekend(date)}` → `class:weekend={dateClassifications[date]?.isWeekend}`
- `class:today={isToday(date)}` → `class:today={dateClassifications[date]?.isToday}`

**Fix 6: Rust Git Command Consolidation (~15 min)**

File: `src-tauri/src/commands.rs`
Lines: 181-255

Replace 4 git process spawns with 1 using `git status -sb --porcelain`.

See [[10_PERF_OPTIMIZATION_SPEC]] for full implementation details.

## File Locations

| File | Purpose | Status |
|------|---------|--------|
| `specs/10_PERF_OPTIMIZATION_SPEC.md` | Full spec for all 6 fixes | Reference |
| `specs/context_packages/02_*.md` | Previous context package | Reference |
| `src/lib/stores/files.svelte.ts` | Files store | Modified (Fix 4 done) |
| `src/lib/stores/timeline.svelte.ts` | Timeline store | Needs Fix 4 |
| `src/lib/stores/context.svelte.ts` | Context store | Needs Fix 4 |
| `src/lib/components/TimelineRow.svelte` | Timeline row | Needs Fix 5 |
| `src-tauri/src/commands.rs` | Rust backend | Needs Fix 6 |

## For Next Agent

### Context Chain
- Previous: [[02_2026-01-05_PERF_OPTIMIZATION_HANDOFF]] (Phase 1 handoff)
- This package: Phase 2 in progress
- Next action: Complete Fix 4 in 2 files, then Fix 5, then Fix 6

### Start Here

1. Read this context package (you're doing it now)
2. Read [[10_PERF_OPTIMIZATION_SPEC]] for exact code changes
3. Run `cd apps/tastematter && npm test` to verify 236 tests passing
4. Complete Fix 4 in timeline.svelte.ts (copy pattern from files.svelte.ts)
5. Complete Fix 4 in context.svelte.ts (same pattern for refreshChains)
6. Implement Fix 5 in TimelineRow.svelte
7. Implement Fix 6 in commands.rs (optional - Rust changes)
8. Run tests after each fix
9. Commit when all passing

### Do NOT

- Don't modify files.svelte.ts (Fix 4 already done there)
- Don't modify Phase 1 files (App.svelte, WorkstreamView, git.svelte.ts)
- Don't change the request deduplication pattern (it's correct)

### Key Insight

The request deduplication pattern uses a simple counter. Each new request increments the counter, and responses only update state if their request ID matches the current counter. This ensures stale responses from superseded requests are ignored.

[VERIFIED: files.svelte.ts:21-50 shows working implementation]
