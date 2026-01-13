---
title: "Tastematter Context Package 12"
package_number: 12

migrated_from: "apps/context-os/specs/tastematter/context_packages/12_2025-12-30_DESIGN_SYSTEM_CLEANUP.md"
status: current
previous_package: "[[11_2025-12-30_VISUAL_DESIGN_AUDIT]]"
related:
  - "[[src/app.css]]"
  - "[[src/lib/components/GitPanel.svelte]]"
  - "[[.claude/plans/snazzy-meandering-babbage.md]]"
tags:
  - context-package
  - tastematter
  - design-system
---

# Tastematter - Context Package 12

## Executive Summary

Continued design system cleanup from previous session. Committed visual improvements (91749f5), then ran comprehensive 4-agent frontend audit identifying 70+ remaining issues. Started full cleanup - GitPanel.svelte complete, 11 more components pending. **79 tests passing.**

## Global Context

**Project:** Tastematter - Desktop GUI for context-os CLI
**Tech Stack:** Tauri 2.9.5 + Svelte 5.46.x + Vite 7.3.0
**Architecture:** Svelte 5 Frontend → Tauri IPC → Rust Backend → context-os CLI subprocess

### Phase Status

| Phase | Status |
|-------|--------|
| 0-3 | COMPLETE (Scaffold, IPC, HeatMap, Git) |
| 4 | **COMPLETE** (Timeline View) |
| 5 | SPEC READY (Session View) |

## Local Problem Set

### Completed This Session

1. **First Commit** (91749f5) [VERIFIED: git log]
   - Added spacing scale (--space-1 through --space-8)
   - Added typography scale (--font-xs through --font-lg)
   - Added border-radius tokens (--radius-sm/md/lg/full)
   - Added shadow tokens (--shadow-sm/md/lg)
   - Added heat map color tokens (--heat-empty/low/high)
   - Added button padding tokens (--button-padding-sm/md/lg)
   - Added global focus-visible on buttons
   - Fixed LoadingSpinner, HeatMapRow, TimelineView, TimelineRow to use tokens

2. **Frontend Audit** (4 parallel agents) [VERIFIED: audit output]
   - Identified 18 hardcoded border-radius values
   - Identified 45 redundant var() fallbacks
   - Identified 8 button padding variations
   - Identified 0 responsive breakpoints in major components

3. **GitPanel.svelte Cleanup** [VERIFIED: file modified]
   - 6 border-radius → var(--radius-sm/md)
   - 12 fallback removals
   - 1 button padding → var(--button-padding-sm)

### In Progress (Uncommitted)

**Files Modified:**
- `src/app.css` - Button padding tokens added (already in 91749f5)
- `src/lib/components/GitPanel.svelte` - Design tokens applied

**Cleanup Plan Written:** `.claude/plans/snazzy-meandering-babbage.md`

### Jobs To Be Done (Next Session)

**Remaining Components to Fix:**

| Component | Radii | Fallbacks | Padding |
|-----------|-------|-----------|---------|
| GranularityToggle.svelte | 1 | 5 | 1 |
| ViewModeToggle.svelte | 1 | 5 | 1 |
| TimeRangeToggle.svelte | 1 | 0 | 1 |
| GitActions.svelte | 1 | 6 | 1 |
| GitFileList.svelte | 1 | 6 | 1 |
| GitStatusBadge.svelte | 0 | 3 | 0 |
| ErrorDisplay.svelte | 3 | 0 | 0 |
| HeatMap.svelte | 1 | 8 | 1 |
| TimelineAxis.svelte | 1 | 0 | 0 |
| TimelineLegend.svelte | 2 | 0 | 0 |
| **Remaining Total** | **12** | **33** | **6** |

**Edit Pattern for Each Component:**
1. Replace `border-radius: 4px` → `var(--radius-sm)`
2. Replace `border-radius: 8px` → `var(--radius-md)`
3. Remove fallback values from `var(--name, #hex)` → `var(--name)`
4. Replace `padding: 0.5rem 1rem` → `var(--button-padding-md)`

## File Locations

| File | Purpose | Status |
|------|---------|--------|
| [[src/app.css]] | Design tokens, global styles | Modified (committed) |
| [[src/lib/components/GitPanel.svelte]] | Git panel component | Modified (uncommitted) |
| [[.claude/plans/snazzy-meandering-babbage.md]] | Full cleanup plan | Active |

## Test State

- **Tests:** 79 passing, 0 failing [VERIFIED: pnpm test:unit 21:03]
- **Command:** `pnpm test:unit`

## Git State

```
Commits:
91749f5 feat(tastematter): Add design tokens + visual improvements  ← Latest commit
433c5a1 feat(tastematter): Phase 4 - Timeline View complete
8b12014 feat(tastematter): Phase 3 complete - Git Panel

Uncommitted:
M src/app.css                        (minor - already mostly in 91749f5)
M src/lib/components/GitPanel.svelte (design token cleanup)
```

## For Next Agent

**Context Chain:**
- Previous: [[11_2025-12-30_VISUAL_DESIGN_AUDIT]] (visual improvements + audit)
- This package: Design system cleanup in progress
- Next action: Continue component fixes from plan

**Start here:**
1. Read plan file: `.claude/plans/snazzy-meandering-babbage.md`
2. GitPanel.svelte is DONE - continue with toggle components
3. Run `pnpm test:unit` to verify 79 tests pass
4. Apply same pattern to remaining 11 components

**Edit Commands for Toggle Components:**

For GranularityToggle.svelte, ViewModeToggle.svelte:
```
Replace style block:
- padding: 0.5rem 1rem → var(--button-padding-md)
- border-radius: 4px → var(--radius-sm)
- var(--border-color, #ccc) → var(--border-color)
- var(--bg-secondary, #f5f5f5) → var(--bg-secondary)
- var(--bg-hover, #e8e8e8) → var(--bg-hover)
- var(--bg-active, #1a1a2e) → var(--bg-active)
- var(--text-inverse, white) → var(--text-inverse)
```

For TimeRangeToggle.svelte:
```
- padding: 0.5rem 1rem → var(--button-padding-md)
- border-radius: 4px → var(--radius-sm)
(no fallbacks to remove)
```

**After all components fixed:**
```bash
pnpm test:unit  # Verify 79 pass
git add .
git commit -m "refactor(tastematter): Complete design system cleanup

- Standardized border-radius across 15 components
- Removed 45 redundant CSS variable fallbacks
- Unified button padding to token variants (sm/md/lg)
- All components now use design tokens consistently

79 tests passing.

🤖 Generated with Claude Code

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

**Do NOT:**
- Edit existing context packages (append-only)
- Skip running tests after changes
- Create new design tokens (use existing ones)

**Key Insight:**
The cleanup is mechanical - same edit pattern for all components. Just work through the list systematically. [VERIFIED: GitPanel.svelte completed with same pattern]
