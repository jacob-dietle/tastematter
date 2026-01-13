---
title: "Tastematter Context Package 00"
package_number: 0

migrated_from: "apps/context-os/specs/tastematter/context_packages/00_2025-12-28_PHASE0_COMPLETE.md"
status: current
previous_package: null
related:
  - "[[apps/context_os_events/specs/tastematter/AGENT_IMPLEMENTATION_PACKAGE.md]]"
  - "[[apps/context_os_events/specs/tastematter/task_specs/PHASE_0_SCAFFOLD.md]]"
  - "[[apps/context_os_events/specs/tastematter/task_specs/PHASE_1_IPC_FOUNDATION.md]]"
  - "[[apps/tastematter/src-tauri/tauri.conf.json]]"
tags:
  - context-package
  - tastematter
  - tauri
  - svelte
---

# Tastematter - Context Package 00

## Executive Summary

Phase 0 scaffold complete. Tauri 2.9.5 + Svelte 5 + Vite desktop app running successfully. Phase 1 (IPC foundation) in progress - test dependencies being installed when session paused.

## Global Context

Tastematter is the **Context OS Visibility Layer** - a desktop GUI for visualizing file access patterns from the context-os CLI.

### Architecture Overview

```
┌────────────────────────────────────────────────────────────┐
│                    Tastematter (Tauri)                      │
├─────────────────────┬──────────────────────────────────────┤
│   Svelte 5 Frontend │         Rust Backend                 │
│   ├── App.svelte    │         ├── commands.rs              │
│   ├── TimeSelector  │         └── invoke handlers          │
│   ├── HeatMap       │                  │                   │
│   └── GitPanel      │                  │ spawn             │
│         │           │                  ▼                   │
│         │ invoke()  │         ┌────────────────┐           │
│         └───────────┼────────▶│ context-os CLI │           │
│                     │         │ (Python)       │           │
│                     │         └────────────────┘           │
└─────────────────────┴──────────────────────────────────────┘
```

References: [[AGENT_IMPLEMENTATION_PACKAGE.md]], [[02_ARCHITECTURE_SPEC.md]]

### Key Design Decisions

- Tauri 2.x (not 1.x) for modern security model [VERIFIED: [[tauri.conf.json]]]
- Svelte 5 runes ($state, $derived) not legacy stores [VERIFIED: [[App.svelte]]:2]
- IPC via `invoke()` calling Rust commands that spawn context-os CLI
- Permissions model: core:default + log:default (shell plugin not needed for subprocess)

## Local Problem Set

### Completed This Session

- [X] Phase 0 scaffold complete [VERIFIED: git commit 498fee7]
  - Tauri 2.9.5 + Svelte 5.46.x + Vite 7.3.0
  - Window opens at 1200x800 with "Hello Tastematter"
  - Hot reload working

- [X] Fixed Rust version incompatibility [VERIFIED: rustup update output]
  - Problem: Rust 1.80.1 too old for Tauri 2.x dependencies
  - Fix: `rustup update stable` → Rust 1.92.0

- [X] Fixed Tauri 2.x permissions error [VERIFIED: successful build]
  - Problem: `shell:allow-open` not valid without shell plugin
  - Fix: Changed to `log:default` in capabilities/default.json (1-line config change)
  - Applied "Dumbest Possible Fix" from debugging skill

- [X] Git repo initialized for tastematter [VERIFIED: git log]
  - Location: `apps/tastematter/.git/`
  - Separate from main gtm_operating_system repo (apps/ is gitignored)

### In Progress

- [ ] Phase 1.1: Install test dependencies
  - State: pnpm add command was running (77 packages added)
  - Command: `pnpm add -D vitest @vitest/browser vitest-browser-svelte jsdom @playwright/test`
  - Interrupted at ~95% complete
  - Evidence: pnpm output showed "Packages: +77" before interrupt

### Jobs To Be Done (Next Session)

1. [ ] Complete Phase 1.1 - Verify test dependencies installed
   - Run: `pnpm list vitest @playwright/test` to confirm
   - If not: Re-run install command

2. [ ] Phase 1.2 - Create type definitions
   - Create: `src/lib/types/index.ts`
   - Types: QueryFlexArgs, QueryResult, FileResult, CommandError
   - Reference: [[PHASE_1_IPC_FOUNDATION.md]] Type Contracts section

3. [ ] Phase 1.3 - Create Rust commands module
   - Create: `src-tauri/src/commands.rs`
   - Command: `query_flex` with error handling
   - Update: `src-tauri/src/lib.rs` to register command

4. [ ] Phase 1.4 - Create TS API wrapper + store
   - Create: `src/lib/api/tauri.ts` (invoke wrapper)
   - Create: `src/lib/stores/query.ts` (Svelte 5 store with $state)

5. [ ] Phase 1.5 - Create UI components
   - TimeSelector.svelte (7d/30d/90d buttons)
   - LoadingSpinner.svelte
   - ErrorDisplay.svelte
   - QueryResults.svelte (table view)

6. [ ] Phase 1.6 - Run tests and verify
   - Unit tests with Vitest
   - E2E tests with Playwright

## File Locations

| File | Purpose | Status |
|------|---------|--------|
| [[apps/tastematter/]] | Project root | Created |
| [[apps/tastematter/src/App.svelte]] | Main Svelte component | Modified (uses $state) |
| [[apps/tastematter/src-tauri/tauri.conf.json]] | Tauri config | Configured |
| [[apps/tastematter/src-tauri/capabilities/default.json]] | Permissions | Fixed (log:default) |
| [[apps/tastematter/src-tauri/src/lib.rs]] | Rust entry point | Default |
| [[apps/tastematter/src-tauri/Cargo.toml]] | Rust dependencies | Default |

### Spec Files (Read These)

| File | Purpose |
|------|---------|
| [[AGENT_IMPLEMENTATION_PACKAGE.md]] | Full context for all phases |
| [[PHASE_0_SCAFFOLD.md]] | Phase 0 details (DONE) |
| [[PHASE_1_IPC_FOUNDATION.md]] | Phase 1 details (IN PROGRESS) |
| [[PHASE_2_HEATMAP.md]] | Phase 2 details (PENDING) |
| [[PHASE_3_GIT_PANEL.md]] | Phase 3 details (PENDING) |

## Test State

- Tests: Not yet created (Phase 1 work)
- Build: Passing [VERIFIED: `pnpm tauri dev` completed successfully]
- App: Running [VERIFIED: app.exe launched]

### Verification Commands for Next Agent

```bash
# Navigate to project
cd apps/tastematter

# Check dependencies installed
pnpm list vitest @playwright/test

# Verify build still works
pnpm tauri dev

# Check git status
git status
git log --oneline -3
```

## For Next Agent

**Context Chain:**
- Previous: None (this is first package)
- This package: Phase 0 complete, Phase 1 in progress
- Next action: Verify test dependencies, continue Phase 1

**Start here:**
1. Read this context package (you're doing it now)
2. Read [[PHASE_1_IPC_FOUNDATION.md]] for detailed implementation steps
3. Run: `cd apps/tastematter && pnpm list vitest` to verify deps
4. If deps missing, run: `pnpm add -D vitest @vitest/browser vitest-browser-svelte jsdom @playwright/test`
5. Continue from Phase 1.2 (type definitions)

**Do NOT:**
- Use `shell:allow-open` permission (requires shell plugin we don't need)
- Use Svelte legacy `writable()` stores (use $state runes instead)
- Try to build with Rust < 1.81 (Tauri 2.x deps require newer)
- Edit files in main gtm_operating_system repo (apps/ is gitignored)

**Key Insights:**
- Tauri 2.x has strict capability-based security [VERIFIED: build error on invalid permission]
- IPC pattern: Rust commands spawn subprocess, parse JSON, return typed Result
- Svelte 5 requires `$props()` for component props, `$state()` for reactive variables
- First build takes ~3-4 min (399 Rust crates), subsequent builds much faster (cached)

**Node.js Warning:**
- Vite 7.3.0 warns about Node 20.15.1 (wants 20.19+) but still works
- Can be ignored or upgrade Node if causes issues

## Session Metrics

- Duration: ~2 hours estimated
- Context tokens at end: ~153k/200k (76%)
- Phases completed: 1 (Phase 0)
- Bugs fixed: 2 (Rust version, permissions)
- Lines of code written: ~50 (mostly config)
