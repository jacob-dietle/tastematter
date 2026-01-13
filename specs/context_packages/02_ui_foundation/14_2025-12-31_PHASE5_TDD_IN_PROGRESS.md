---
title: "Tastematter Context Package 14"
package_number: 14

migrated_from: "apps/context-os/specs/tastematter/context_packages/14_2025-12-31_PHASE5_TDD_IN_PROGRESS.md"
status: current
previous_package: "[[13_2025-12-30_DESIGN_SYSTEM_COMPLETE]]"
related:
  - "[[task_specs/PHASE_5_SESSION_VIEW.md]]"
  - "[[src/lib/types/index.ts]]"
  - "[[tests/unit/stores/session.test.ts]]"
tags:
  - context-package
  - tastematter
  - phase-5
  - tdd
---

# Tastematter - Context Package 14

## Executive Summary

Phase 5 (Session View) implementation started using TDD methodology. **Cycle 1 RED complete** - 15 store tests written. **Cycle 1 GREEN in progress** - types added, API stub added, session store NOT YET CREATED. 79 original tests passing, 1 test file failing (expected - store doesn't exist yet).

## Global Context

**Project:** Tastematter - Desktop GUI for context-os CLI
**Tech Stack:** Tauri 2.9.5 + Svelte 5.46.x + Vite 7.3.0
**Architecture:** Svelte 5 Frontend → Tauri IPC → Rust Backend → context-os CLI subprocess

### Phase Status

| Phase | Status |
|-------|--------|
| 0-4 | COMPLETE (Scaffold, IPC, HeatMap, Git, Timeline) |
| 5 | **IN PROGRESS** (Session View - TDD Cycle 1) |

### TDD Methodology (Kent Beck)

Following RED → GREEN → REFACTOR → COMMIT cycle:
```
Cycle 1: Store Tests + Types + Store     ← IN PROGRESS
Cycle 2: Rust Backend
Cycle 3: Small Component Tests + Components
Cycle 4: SessionCard Tests + SessionCard/View
Cycle 5: Integration
```

## Local Problem Set

### Completed This Session

1. **Context Foundation Loaded** [VERIFIED: read package 13]
2. **TDD Plan Created** [VERIFIED: [[~/.claude/plans/shimmering-yawning-thunder.md]]]
3. **Cycle 1 RED - Tests Written** [VERIFIED: [[tests/unit/stores/session.test.ts]]]
   - 15 tests for session store (initial state, fetch, expand/collapse, chain filter, derived)
   - Tests fail as expected (store doesn't exist)
4. **Cycle 1 GREEN Partial:**
   - Session types added [VERIFIED: [[src/lib/types/index.ts]]:161-233]
   - API stub added [VERIFIED: [[src/lib/api/tauri.ts]]:72-75]

### In Progress

- **Cycle 1 GREEN: Create session.svelte.ts store**
  - Current state: Types and API stub done, store file NOT created
  - Next step: Create `src/lib/stores/session.svelte.ts` following timeline.svelte.ts pattern
  - Reference: [[src/lib/stores/timeline.svelte.ts]] (read this for pattern)

### Jobs To Be Done (Next Session)

| Priority | Task | Success Criteria |
|----------|------|------------------|
| 1 | Create session.svelte.ts | 15 tests pass |
| 2 | Commit Cycle 1 | Tests green, committed |
| 3 | Cycle 2: Rust command | query_sessions works |
| 4 | Cycle 3: Component tests + components | ChainBadge, FilePreview, FileTree |
| 5 | Cycle 4: SessionCard + SessionView | Progressive disclosure works |
| 6 | Cycle 5: Integration | Sessions tab in App.svelte |

## File Locations

| File | Purpose | Status |
|------|---------|--------|
| [[tests/unit/stores/session.test.ts]] | Session store tests (15) | CREATED |
| [[src/lib/types/index.ts]] | Session types added | MODIFIED |
| [[src/lib/api/tauri.ts]] | querySessions stub | MODIFIED |
| [[src/lib/stores/session.svelte.ts]] | Session store | **NOT CREATED** |
| [[~/.claude/plans/shimmering-yawning-thunder.md]] | TDD implementation plan | REFERENCE |

## Test State

- **Original tests:** 79 passing [VERIFIED: pnpm test:unit]
- **New test file:** 1 failing (expected - store doesn't exist)
- **Command:** `pnpm test:unit`

### Test Commands for Next Agent

```bash
# Run all tests (expect 1 file failing until store created)
cd apps/tastematter && pnpm test:unit

# Run only session store tests
cd apps/tastematter && pnpm test:unit tests/unit/stores/session.test.ts

# After creating store, all should pass
cd apps/tastematter && pnpm test:unit  # Expect 94 tests (79 + 15)
```

## Git State

```
Modified (not committed):
M src/lib/api/tauri.ts
M src/lib/types/index.ts
?? tests/unit/stores/session.test.ts

Latest commits:
2cdf10e refactor(tastematter): Complete design system cleanup  ← HEAD
91749f5 feat(tastematter): Add design tokens + visual improvements
433c5a1 feat(tastematter): Phase 4 - Timeline View complete
```

## For Next Agent

**Context Chain:**
- Previous: [[13_2025-12-30_DESIGN_SYSTEM_COMPLETE]] (Phase 4 done, design system done)
- This package: Phase 5 TDD started, Cycle 1 GREEN in progress
- Next action: Create session store to make tests pass

**Start here:**
1. Read this context package
2. Read [[~/.claude/plans/shimmering-yawning-thunder.md]] for full TDD plan
3. Read [[src/lib/stores/timeline.svelte.ts]] for store pattern
4. Create `src/lib/stores/session.svelte.ts` following the pattern
5. Run `pnpm test:unit tests/unit/stores/session.test.ts` → expect 15 pass

**Session Store Implementation (copy from timeline pattern):**

```typescript
// src/lib/stores/session.svelte.ts
import { querySessions } from '$lib/api/tauri';
import type { SessionQueryResult, CommandError } from '$lib/types';

export function createSessionStore() {
  // State
  let loading = $state(false);
  let data = $state<SessionQueryResult | null>(null);
  let error = $state<CommandError | null>(null);
  let selectedRange = $state<'7d' | '14d' | '30d'>('7d');
  let expandedSessions = $state<Set<string>>(new Set());
  let selectedChain = $state<string | null>(null);

  // Actions
  async function fetch() {
    loading = true;
    error = null;
    try {
      data = await querySessions({ time: selectedRange, chain: selectedChain ?? undefined, limit: 50 });
    } catch (e) {
      error = e as CommandError;
      data = null;
    } finally {
      loading = false;
    }
  }

  async function setRange(range: '7d' | '14d' | '30d') {
    selectedRange = range;
    await fetch();
  }

  async function setChainFilter(chainId: string | null) {
    selectedChain = chainId;
    await fetch();
  }

  function toggleSessionExpanded(sessionId: string) {
    const newSet = new Set(expandedSessions);
    if (newSet.has(sessionId)) {
      newSet.delete(sessionId);
    } else {
      newSet.add(sessionId);
    }
    expandedSessions = newSet;
  }

  function isExpanded(sessionId: string): boolean {
    return expandedSessions.has(sessionId);
  }

  function collapseAll() {
    expandedSessions = new Set();
  }

  // Derived
  function getMaxAccessCount(): number {
    if (!data?.sessions?.length) return 0;
    return Math.max(...data.sessions.flatMap(s => s.files.map(f => f.access_count)), 1);
  }

  function getFilteredSessions() {
    if (!data?.sessions) return [];
    if (!selectedChain) return data.sessions;
    return data.sessions.filter(s => s.chain_id === selectedChain);
  }

  return {
    get loading() { return loading; },
    get data() { return data; },
    get error() { return error; },
    get selectedRange() { return selectedRange; },
    get selectedChain() { return selectedChain; },
    get expandedSessions() { return expandedSessions; },
    get maxAccessCount() { return getMaxAccessCount(); },
    get filteredSessions() { return getFilteredSessions(); },

    fetch,
    setRange,
    setChainFilter,
    toggleSessionExpanded,
    isExpanded,
    collapseAll,
  };
}
```

**Do NOT:**
- Skip writing tests first (TDD discipline)
- Edit existing packages (append-only)
- Commit before tests pass (GREEN must be verified)

**Key Insight:**
The store pattern uses Svelte 5 `$state` runes (not old stores API). Follow [[timeline.svelte.ts]] exactly. Tests are already written and mock the API layer.
