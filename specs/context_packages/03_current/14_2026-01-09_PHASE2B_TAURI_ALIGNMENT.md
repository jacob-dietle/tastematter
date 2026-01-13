---
title: "Tastematter Context Package 14"
package_number: 14

migrated_from: "apps/tastematter/specs/context_packages/14_2026-01-09_PHASE2B_TAURI_ALIGNMENT.md"
status: current
previous_package: "[[13_2026-01-09_PHASE2_DATA_SOURCE_FIX]]"
related:
  - "[[apps/context-os/core/src/query.rs]]"
  - "[[apps/context-os/core/src/types.rs]]"
  - "[[apps/context-os/core/src/main.rs]]"
  - "[[apps/tastematter/src-tauri/src/commands.rs]]"
tags:
  - context-package
  - tastematter
  - phase-2b-alignment
  - cli-binary
---

# Tastematter - Context Package 14

## Executive Summary

Phase 2B Tauri alignment in progress. Fixed SQL injection vulnerabilities in query_sessions. Added chain filter to query_timeline (types.rs + query.rs). CLI binary created by background agent (main.rs + clap). All 15 tests passing. **Next agent must complete Tauri command + frontend chain integration for timeline.**

## Global Context

### Architecture Overview

```
apps/context-os/core/              # Rust library + NEW CLI binary
├── src/
│   ├── lib.rs                     # Library exports
│   ├── main.rs                    # NEW: CLI binary with clap
│   ├── query.rs                   # MODIFIED: SQL injection fixes + chain filter
│   ├── types.rs                   # MODIFIED: Added chain to QueryTimelineInput
│   └── storage.rs                 # Database path resolution
│
apps/tastematter/                   # Tauri desktop app
├── src-tauri/
│   └── src/commands.rs            # NEEDS UPDATE: Add chain to query_timeline
└── src/
    └── lib/
        ├── api/tauri.ts           # NEEDS UPDATE: Add chain to queryTimeline
        └── stores/timeline.svelte.ts  # NEEDS UPDATE: Pass ctx.selectedChain
```

### Key Design Decisions

1. **Parameterized SQL queries** - Fixed SQL injection by using bind parameters [VERIFIED: [[query.rs]]:347-363]
2. **Chain filter on timeline** - Added chain param to filter timeline by workstream [VERIFIED: [[types.rs]]:51-52]
3. **CLI binary via clap** - Standalone Rust CLI for <100ms queries [VERIFIED: [[main.rs]] exists]

## Local Problem Set

### Completed This Session

- [X] Fixed SQL injection in query_sessions (chain filter) [VERIFIED: [[query.rs]]:347-363]
- [X] Fixed SQL injection in query_sessions (session_id subquery) [VERIFIED: [[query.rs]]:382-394]
- [X] Added `chain: Option<String>` to QueryTimelineInput [VERIFIED: [[types.rs]]:51-52]
- [X] Added chain filtering to query_timeline bucket query [VERIFIED: [[query.rs]]:196-222]
- [X] Added chain filtering to query_timeline file query [VERIFIED: [[query.rs]]:250-287]
- [X] Fixed integration test for new chain field [VERIFIED: [[integration_test.rs]]:141-146]
- [X] Fixed main.rs CLI for new chain field [VERIFIED: [[main.rs]]:107-109, 181-189]
- [X] All 15 tests passing [VERIFIED: cargo test output]
- [X] CLI binary created (background agent) [VERIFIED: [[main.rs]] with clap]

### In Progress

- [ ] Add chain param to Tauri query_timeline command
  - Current state: types.rs updated, commands.rs not yet
  - File: [[apps/tastematter/src-tauri/src/commands.rs]]
  - Search for: `query_timeline` function

- [ ] Add chain to frontend queryTimeline API
  - File: [[apps/tastematter/src/lib/api/tauri.ts]]
  - Search for: `queryTimeline` function

- [ ] Update timeline store to pass selectedChain
  - File: [[apps/tastematter/src/lib/stores/timeline.svelte.ts]]
  - Search for: `queryTimeline` call

### Jobs To Be Done (Next Session)

1. [ ] Update Tauri commands.rs - Add `chain: Option<String>` param to query_timeline
   - Success criteria: Command accepts chain parameter
   - File: [[apps/tastematter/src-tauri/src/commands.rs]]

2. [ ] Update frontend tauri.ts - Add chain to queryTimeline interface
   - Success criteria: TypeScript type includes chain
   - File: [[apps/tastematter/src/lib/api/tauri.ts]]

3. [ ] Update timeline.svelte.ts - Pass ctx.selectedChain to query
   - Success criteria: Timeline filters by selected chain
   - File: [[apps/tastematter/src/lib/stores/timeline.svelte.ts]]

4. [ ] Verify WorkstreamView passes chain to querySessions
   - Success criteria: Sessions filter by chain at backend (not client-side)
   - File: [[apps/tastematter/src/lib/components/WorkstreamView.svelte]]

5. [ ] Test end-to-end in Tauri app
   - Success criteria: Select chain in sidebar, Timeline + Sessions filter correctly

6. [ ] Verify CLI binary works
   - Command: `./target/debug/context-os query flex --time 7d --format json`
   - Success criteria: Returns real file data in <100ms

## File Locations

| File | Purpose | Status |
|------|---------|--------|
| [[apps/context-os/core/src/query.rs]] | Query engine with SQL | Modified - SQL injection fixed, chain filter added |
| [[apps/context-os/core/src/types.rs]] | Type definitions | Modified - chain added to QueryTimelineInput |
| [[apps/context-os/core/src/main.rs]] | CLI binary | Created by background agent |
| [[apps/context-os/core/Cargo.toml]] | Dependencies | Modified - clap added |
| [[apps/context-os/core/tests/integration_test.rs]] | Integration tests | Modified - chain field added |
| [[apps/tastematter/src-tauri/src/commands.rs]] | Tauri commands | NEEDS UPDATE |
| [[apps/tastematter/src/lib/api/tauri.ts]] | Frontend API | NEEDS UPDATE |
| [[apps/tastematter/src/lib/stores/timeline.svelte.ts]] | Timeline store | NEEDS UPDATE |

## Test State

- **Core tests:** 15 passing (7 unit + 8 integration)
- **Command:** `cd apps/context-os/core && cargo test`
- **Last run:** 2026-01-09
- **Evidence:** [VERIFIED: cargo test output all green]

### Test Commands for Next Agent

```bash
# Verify core tests still pass
cd apps/context-os/core && cargo test

# Build CLI binary
cd apps/context-os/core && cargo build --bin context-os

# Test CLI query
./target/debug/context-os query flex --time 7d --format json

# Build Tauri backend (after commands.rs update)
cd apps/tastematter/src-tauri && cargo build

# Run Tauri app
cd apps/tastematter && npm run tauri dev
```

## Background Agent Status

A background agent was spawned to build the CLI binary (agent ID: a6bbefa).

**Agent task:** Create Rust CLI binary with clap
**Files created:**
- `apps/context-os/core/src/main.rs` - CLI entry point with 4 query commands
- Modified `apps/context-os/core/Cargo.toml` - Added clap dependency + [[bin]] target

**Agent completion status:** Likely complete (main.rs exists with full implementation)

## For Next Agent

**Context Chain:**
- Previous: [[13_2026-01-09_PHASE2_DATA_SOURCE_FIX]] (json_each rewrite)
- This package: SQL injection fixes + chain filter for timeline + CLI binary
- Next action: Complete Tauri + frontend chain integration

**Start here:**
1. Read this context package
2. Run `cargo test` to verify state
3. Update `commands.rs` to add chain param to query_timeline
4. Update `tauri.ts` to add chain to queryTimeline interface
5. Update `timeline.svelte.ts` to pass ctx.selectedChain
6. Test end-to-end in Tauri app

**Do NOT:**
- Edit query.rs SQL (already fixed with parameterized queries)
- Edit types.rs (chain field already added)
- Recreate CLI main.rs (already exists)

**Key insight:**
The Tauri app now shows real data (1,037 files tracked). The remaining 10% is connecting the chain selector to Timeline view. Sessions view already passes chain but may be filtering client-side instead of at backend. Timeline view currently ignores chain selection completely.

[VERIFIED: Explore agent analysis of frontend components]

## SQL Pattern Reference

**Parameterized query pattern (use this):**
```rust
let mut bindings: Vec<String> = Vec::new();
if let Some(ref chain) = input.chain {
    sql.push_str(" AND cg.chain_id = ?");
    bindings.push(chain.clone());
}

let mut query = sqlx::query(&sql);
for binding in &bindings {
    query = query.bind(binding);
}
let rows = query.fetch_all(self.db.pool()).await?;
```

**Do NOT use string interpolation (SQL injection vulnerable):**
```rust
// WRONG - don't do this
sql.push_str(&format!(" AND cg.chain_id = '{}'", chain));
```
