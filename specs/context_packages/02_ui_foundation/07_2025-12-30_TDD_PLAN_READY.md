---
title: "Tastematter Context Package 07"
package_number: 7

migrated_from: "apps/context-os/specs/tastematter/context_packages/07_2025-12-30_TDD_PLAN_READY.md"
status: current
previous_package: "[[06_2025-12-30_SHARED_ARCHITECTURE]]"
related:
  - "[[../02_ARCHITECTURE_SPEC.md]]"
  - "[[task_specs/PHASE_4_TIMELINE.md]]"
  - "[[task_specs/PHASE_5_SESSION_VIEW.md]]"
  - "[[plans/smooth-yawning-fiddle.md]]"
tags:
  - context-package
  - tastematter
  - tdd-planning
---

# Tastematter - Context Package 07

## Executive Summary

Created comprehensive TDD implementation plan for Phase 4 Timeline View. Applied test-driven-execution skill (Kent Beck RED→GREEN→REFACTOR). Plan saved to `plans/smooth-yawning-fiddle.md`. Ready for implementation: ~4 hours, 15+ new tests, 5 components. No code changes this session - planning only.

## Global Context

**Project:** Tastematter - Desktop GUI for context-os CLI
**Purpose:** Visual file access patterns, git visibility, temporal analysis
**Tech Stack:** Tauri 2.9.5 + Svelte 5.46.x + Vite 7.3.0

### Architecture
```
Svelte 5 Frontend → Tauri IPC → Rust Backend → context-os CLI subprocess
                                            → git subprocess
```

### Phase Status
| Phase | Name | Status |
|-------|------|--------|
| 0 | Scaffold | COMPLETE (498fee7) |
| 1 | IPC Foundation | COMPLETE (e11c123) |
| 2 | Heat Map View | COMPLETE (a593aa6) |
| 3 | Git Panel | COMPLETE (8b12014) |
| 4 | Timeline View | **TDD PLAN READY** |
| 5 | Session View | SPEC READY |

## Local Problem Set

### Completed This Session

- [X] Loaded context via context-foundation skill [VERIFIED: read packages 04, 05, 06]
- [X] Verified test state: 42 tests passing [VERIFIED: `pnpm test:unit` output]
- [X] Applied feature-planning-and-decomposition skill [VERIFIED: session transcript]
- [X] Analyzed shared components across Phase 4-5 [VERIFIED: codebase exploration]
- [X] Updated 02_ARCHITECTURE_SPEC.md with "Shared Component Layer" section [VERIFIED: file edit]
- [X] Created context package 06 documenting architecture decisions [VERIFIED: file created]
- [X] Invoked test-driven-execution skill (Kent Beck TDD) [VERIFIED: session transcript]
- [X] Created comprehensive TDD implementation plan [VERIFIED: [[plans/smooth-yawning-fiddle.md]]]
- [X] Confirmed TimeSelector → TimeRangeToggle rename in place [VERIFIED: user answer]

### In Progress

- Nothing in progress (clean handoff - planning complete)

### Jobs To Be Done (Next Session)

**IMPLEMENTATION READY - Follow TDD Plan**

1. [ ] **Step 0A: Create colors.ts** (5 min)
   - File: `src/lib/utils/colors.ts`
   - Content: COLORS constant + getHeatColor() + calculateIntensity()
   - No tests needed (pure constants)

2. [ ] **Step 0B: Rename TimeSelector → TimeRangeToggle** (10 min)
   - Rename file in place
   - Add `options` prop (default: ['7d', '14d', '30d'])
   - Update imports in App.svelte

3. [ ] **Step 1: Add TypeScript types** (15 min)
   - File: `src/lib/types/index.ts`
   - Add: TimeBucket, FileTimeline, TimelineData, TimelineQueryArgs, TimelineState

4. [ ] **Step 2-3: Rust command** (45 min)
   - File: `src-tauri/src/commands.rs`
   - Add: Rust structs + query_timeline command
   - Register in lib.rs

5. [ ] **Step 4: TypeScript API wrapper** (10 min)
   - File: `src/lib/api/tauri.ts`
   - Add: queryTimeline function

6. [ ] **Step 5-6: Timeline Store (RED→GREEN)** (30 min)
   - RED: Write 15 tests first in `tests/unit/stores/timeline.test.ts`
   - GREEN: Implement `src/lib/stores/timeline.svelte.ts`

7. [ ] **Step 7-16: Components (RED→GREEN each)** (90 min)
   - TimeRangeToggle tests + generalization
   - TimelineAxis tests + component
   - TimelineRow tests + component
   - TimelineLegend tests + component
   - TimelineView tests + component

8. [ ] **Step 17-19: Integration + Refactor** (30 min)
   - Wire to App.svelte
   - E2E test (optional)
   - Clean up

## Key Decisions This Session

### Decision 1: TDD Implementation Order

**Source:** [VERIFIED: [[plans/smooth-yawning-fiddle.md]]]

```
Shared Layer (30 min)
    ↓
Types + Rust Command (45 min)
    ↓
Store: RED → GREEN (30 min)
    ↓
Components: RED → GREEN each (90 min)
    ↓
Integration + REFACTOR (30 min)
```

### Decision 2: TimeSelector Rename Strategy

**Decision:** Rename in place (user confirmed)
**Rationale:** Cleaner than creating duplicate, no backward compatibility needed

### Decision 3: Test-First for Store

**Pattern:** Write all 15 store tests BEFORE implementing store

```typescript
describe('timelineStore', () => {
  // Initial state (5 tests)
  // Fetch behavior (4 tests)
  // Range selection (2 tests)
  // Hover state (2 tests)
  // Derived values (2 tests)
});
```

## TDD Plan Location

**CRITICAL FILE:** `C:\Users\dietl\.claude\plans\smooth-yawning-fiddle.md`

Contains:
- Step-by-step implementation order
- All TypeScript type contracts
- All test specifications
- Mock data for tests
- File checklist
- Success criteria

## File Locations

| File | Purpose | Status |
|------|---------|--------|
| [[plans/smooth-yawning-fiddle.md]] | TDD implementation plan | CREATED |
| [[../02_ARCHITECTURE_SPEC.md]] | Shared layer documented | UPDATED |
| [[06_2025-12-30_SHARED_ARCHITECTURE]] | Architecture decisions | CREATED |
| [[task_specs/PHASE_4_TIMELINE.md]] | Timeline spec | Reference |

### Files to Create (Next Session)
```
src/lib/utils/colors.ts              # Shared palette
src/lib/stores/timeline.svelte.ts    # Timeline store
src/lib/components/TimelineView.svelte
src/lib/components/TimelineAxis.svelte
src/lib/components/TimelineRow.svelte
src/lib/components/TimelineLegend.svelte
tests/unit/stores/timeline.test.ts
tests/unit/components/*.test.ts      # 5 component test files
```

### Files to Modify (Next Session)
```
src/lib/components/TimeSelector.svelte → TimeRangeToggle.svelte (rename)
src/lib/types/index.ts               # Add timeline types
src/lib/api/tauri.ts                 # Add queryTimeline
src-tauri/src/commands.rs            # Add Rust command
src-tauri/src/lib.rs                 # Register command
src/App.svelte                       # Add timeline view
```

## Test State

- Tests: 42 passing (19 aggregation + 5 query + 18 git)
- Command: `pnpm test:unit`
- Last run: 2025-12-30 14:20
- Evidence: [VERIFIED: vitest output in session]

### Test Commands for Next Agent
```bash
# Verify current state before starting
cd apps/tastematter && pnpm test:unit

# After implementing store (Step 6)
cd apps/tastematter && pnpm test:unit -- --grep "timelineStore"

# After all implementation
cd apps/tastematter && pnpm test:unit
# Expected: 42 + 15+ = 57+ tests passing
```

## For Next Agent

**Context Chain:**
- Package 05: Phase 5 Session View spec created
- Package 06: Shared architecture analysis (Dean/Carmack)
- Package 07 (this): TDD implementation plan ready
- Next action: IMPLEMENT following the plan

**Start here:**
1. Read this context package (you're doing it now)
2. Read [[plans/smooth-yawning-fiddle.md]] for full TDD plan
3. Run: `cd apps/tastematter && pnpm test:unit` to verify 42 tests passing
4. Start with Step 0A: Create `colors.ts`
5. Follow RED→GREEN→REFACTOR for each component

**Do NOT:**
- Skip writing tests first (TDD is mandatory)
- Over-engineer shared abstractions (we decided YAGNI applies)
- Create new TimeSelector (rename existing in place)
- Implement without reading the plan file first

**Key insight:**
The TDD plan at `plans/smooth-yawning-fiddle.md` contains EVERYTHING needed:
- Exact type definitions (copy-paste ready)
- Test specifications (15+ tests outlined)
- Implementation order (optimized for RED→GREEN flow)
- Mock data for tests
- Success criteria checklist

[VERIFIED: plan file created this session]

## Estimated Implementation Time

| Phase | Tasks | Time |
|-------|-------|------|
| 0 | Shared layer (colors.ts, TimeRangeToggle) | 30 min |
| 1 | Types + Rust command | 45 min |
| 2 | Store (RED → GREEN) | 30 min |
| 3 | Components (RED → GREEN each) | 90 min |
| 4 | Integration + REFACTOR | 30 min |
| **Total** | | **~4 hours** |

## Commit History

```
8b12014 feat(tastematter): Phase 3 complete - Git Panel
a593aa6 feat(tastematter): Phase 2 complete - Heat Map View
e11c123 feat(tastematter): Phase 1 complete - IPC foundation
498fee7 feat(scaffold): Phase 0 complete - Tauri 2.0 + Svelte 5 + Vite
```

No new commits this session (planning only).
Architecture spec updated but not committed yet.
