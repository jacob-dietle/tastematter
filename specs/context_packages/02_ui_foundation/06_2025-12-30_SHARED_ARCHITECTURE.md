---
title: "Tastematter Context Package 06"
package_number: 6

migrated_from: "apps/context-os/specs/tastematter/context_packages/06_2025-12-30_SHARED_ARCHITECTURE.md"
status: current
previous_package: "[[05_2025-12-30_PHASE5_SPEC_CREATED]]"
related:
  - "[[../02_ARCHITECTURE_SPEC.md]]"
  - "[[task_specs/PHASE_4_TIMELINE.md]]"
  - "[[task_specs/PHASE_5_SESSION_VIEW.md]]"
  - "[[../../context_os_intelligence/specs/12_CLI_HYPERCUBE_SPEC.md]]"
tags:
  - context-package
  - tastematter
  - architecture
---

# Tastematter - Context Package 06

## Executive Summary

Applied feature-planning-and-decomposition skill with Jeff Dean/Carmack simplicity principles to identify minimal shared layer between Phase 4 (Timeline) and Phase 5 (Session View). Updated `02_ARCHITECTURE_SPEC.md` with new "Shared Component Layer" section. No code changes - architecture documentation only.

## Global Context

**Project:** Tastematter - Desktop GUI for context-os CLI
**Purpose:** Visual file access patterns, git visibility, temporal analysis
**Tech Stack:** Tauri 2.9.5 + Svelte 5.46.x + Vite 7.3.0

### Architecture (60-Second Rule)

```
┌──────────────────────────────────────────────────────────────────────┐
│                     VIEWS (Hypercube Projections)                     │
├──────────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐      ┌─────────────┐      ┌─────────────┐          │
│  │  HeatMap    │      │  Timeline   │      │  Session    │          │
│  │  (Phase 2)  │      │  (Phase 4)  │      │  (Phase 5)  │          │
│  │  Files×Count│      │  Files×Time │      │  Files×Sess │          │
│  └─────────────┘      └─────────────┘      └─────────────┘          │
│         │                    │                    │                  │
│         └────────────────────┼────────────────────┘                  │
│                              │                                       │
│                    ┌─────────▼─────────┐                            │
│                    │  SHARED LAYER     │                            │
│                    │  - colors.ts      │  ← Extract now             │
│                    │  - TimeRangeToggle│  ← Generalize existing     │
│                    │  - LoadingSpinner │  ← Already exists          │
│                    │  - ErrorDisplay   │  ← Already exists          │
│                    └───────────────────┘                            │
└──────────────────────────────────────────────────────────────────────┘
```

### Phase Status
| Phase | Name | Status |
|-------|------|--------|
| 0 | Scaffold | COMPLETE (498fee7) |
| 1 | IPC Foundation | COMPLETE (e11c123) |
| 2 | Heat Map View | COMPLETE (a593aa6) |
| 3 | Git Panel | COMPLETE (8b12014) |
| 4 | Timeline View | SPEC READY |
| 5 | Session View | SPEC READY |

## Local Problem Set

### Completed This Session

- [X] Loaded context via context-foundation skill [VERIFIED: read packages 04, 05]
- [X] Verified test state: 42 tests passing [VERIFIED: `pnpm test:unit` output]
- [X] Applied feature-planning-and-decomposition skill [VERIFIED: session transcript]
- [X] Analyzed existing codebase for reusable components [VERIFIED: file reads]
- [X] Identified shared vs unique components for Phase 4-5 [VERIFIED: analysis below]
- [X] Updated 02_ARCHITECTURE_SPEC.md with "Shared Component Layer" section [VERIFIED: file edit]

### In Progress

- Nothing in progress (clean handoff)

### Jobs To Be Done (Next Session)

1. [ ] Create `src/lib/utils/colors.ts` (shared palette)
   - Extract Ink & Paper colors: #f6f8fa, #d4c9b8, #8b4513, #1a1a2e
   - Add `getHeatColor(intensity: number)` function
   - Success criteria: Importable by HeatMap, Timeline, Session

2. [ ] Generalize TimeSelector → TimeRangeToggle
   - Current: hardcoded ['7d', '30d', '90d']
   - Target: accept `options` prop, default ['7d', '14d', '30d']
   - Success criteria: Both Timeline and Session can use it

3. [ ] Phase 4 Implementation: Timeline View
   - Read [[task_specs/PHASE_4_TIMELINE.md]] for full spec
   - Follow TDD workflow: RED → GREEN → REFACTOR
   - Success criteria: Timeline shows file access over time with day columns

## Key Architecture Decisions This Session

### Decision 1: Minimal Shared Layer (Dean/Carmack Principle)

**Principle:** "Don't build what you don't need yet. Extract only when duplication is proven."

**EXTRACT NOW (proven duplication):**

| Component | Why Extract |
|-----------|-------------|
| `colors.ts` | Used by HeatMap, Timeline, Session (3 views) |
| `TimeRangeToggle` | Used by Timeline, Session (2 views) |

**DON'T EXTRACT (YAGNI):**

| Tempting Abstraction | Why Skip |
|---------------------|----------|
| `createStore<T>()` factory | Only 4 stores, 20 lines each - copy is fine |
| `BaseViewContainer.svelte` | Views have different layouts |
| `DataFetcher` service | Tauri invoke is already simple |
| `TreeBuilder` class | Only Session needs trees (for now) |

### Decision 2: Store Pattern (Copy, Don't Abstract)

**Rationale:** [INFERRED: 4 stores total, each ~20-40 lines]

All stores follow same pattern but abstraction adds complexity without benefit:

```typescript
export function createXxxStore() {
  let loading = $state(false);
  let data = $state<XxxData | null>(null);
  let error = $state<CommandError | null>(null);

  async function fetch(args) { /* ... */ }
  function reset() { /* ... */ }

  return { get loading() {...}, get data() {...}, fetch, reset };
}
```

### Decision 3: Ink & Paper Color Palette

**Source:** [VERIFIED: [[00_ARCHITECTURE_GUIDE.md]] + existing HeatMapRow.svelte]

```typescript
export const COLORS = {
  empty: '#f6f8fa',    // Paper white (0 activity)
  low: '#d4c9b8',      // Faded ink (low activity)
  medium: '#8b4513',   // Aged ink (medium activity)
  high: '#1a1a2e',     // Deep ink (high activity)
} as const;
```

## Existing Code Analysis

### Already Built (Phase 0-3)

| File | Purpose | Reusable? |
|------|---------|-----------|
| `TimeSelector.svelte` | 7d/30d/90d toggle | ⚠️ Generalize to TimeRangeToggle |
| `query.svelte.ts` | Store pattern | ✅ Pattern to copy |
| `aggregation.ts` | calculateIntensity, getParentDirectory | ✅ Import directly |
| `LoadingSpinner.svelte` | Loading state | ✅ Reuse as-is |
| `ErrorDisplay.svelte` | Error state | ✅ Reuse as-is |

### File Locations

| File | Purpose | Status |
|------|---------|--------|
| [[../02_ARCHITECTURE_SPEC.md]] | Master architecture | UPDATED |
| [[task_specs/PHASE_4_TIMELINE.md]] | Timeline spec | Reference |
| [[task_specs/PHASE_5_SESSION_VIEW.md]] | Session spec | Reference |
| `src/lib/utils/colors.ts` | Shared palette | TO CREATE |
| `src/lib/components/TimeRangeToggle.svelte` | Shared toggle | TO CREATE |

## Test State

- Tests: 42 passing (19 aggregation + 5 query + 18 git)
- Command: `pnpm test:unit`
- Last run: 2025-12-30 13:26
- Evidence: [VERIFIED: vitest output in session]

### Test Commands for Next Agent
```bash
# Verify current state
cd apps/tastematter && pnpm test:unit

# Check git status
cd apps/tastematter && git log --oneline -5
```

## For Next Agent

**Context Chain:**
- Previous: [[05_2025-12-30_PHASE5_SPEC_CREATED]] (Session View spec)
- This package: Shared architecture analysis, 02_ARCHITECTURE_SPEC.md updated
- Next action: Create shared layer files, then implement Phase 4

**Start here:**
1. Read this context package (you're doing it now)
2. Read [[../02_ARCHITECTURE_SPEC.md#Shared Component Layer]] for full details
3. Create `src/lib/utils/colors.ts` (copy from spec)
4. Generalize `TimeSelector.svelte` → `TimeRangeToggle.svelte`
5. Read [[task_specs/PHASE_4_TIMELINE.md]] and implement Timeline View

**Do NOT:**
- Over-engineer shared abstractions (we analyzed this - YAGNI applies)
- Create store factory (copy-paste 20 lines is fine for 4 stores)
- Skip colors.ts (needed by all 3 views)

**Key insight:**
All views are projections of the 5D hypercube:
- HeatMap: Files × AccessCount (what's hot)
- Timeline: Files × Time (when was it active)
- Session: Files × Sessions (which conversations)

The shared layer is minimal by design - only extract what's proven to duplicate.
[VERIFIED: feature-planning skill analysis + existing codebase review]

## Commit History

```
8b12014 feat(tastematter): Phase 3 complete - Git Panel
a593aa6 feat(tastematter): Phase 2 complete - Heat Map View
e11c123 feat(tastematter): Phase 1 complete - IPC foundation
498fee7 feat(scaffold): Phase 0 complete - Tauri 2.0 + Svelte 5 + Vite
```

No new commits this session (architecture documentation only).
