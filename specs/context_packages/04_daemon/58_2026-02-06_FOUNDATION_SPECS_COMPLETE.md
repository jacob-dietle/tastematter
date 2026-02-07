---
title: "Tastematter Context Package 58"
package_number: 58
date: 2026-02-06
status: current
previous_package: "[[57_2026-02-06_FULL_CODEBASE_AUDIT_COMPLETE]]"
related:
  - "[[specs/canonical/14_CODEBASE_AUDIT_2026-02-06.md]]"
  - "[[specs/audits/2026-02-06/audit_data_pipeline.md]]"
  - "[[specs/audits/2026-02-06/cross_check_data_pipeline.md]]"
  - "[[specs/implementation/phase_04_core_improvements/02_PATH_NORMALIZATION_SPEC.md]]"
  - "[[specs/implementation/phase_04_core_improvements/03_CHAIN_NAMES_CLI_SPEC.md]]"
  - "[[specs/implementation/phase_04_core_improvements/04_NON_DESTRUCTIVE_CHAINS_SPEC.md]]"
  - "[[specs/implementation/phase_04_core_improvements/05_SCHEMA_UNIFICATION_SPEC.md]]"
  - "[[specs/implementation/phase_04_core_improvements/06_FILES_WRITTEN_QUERIES_SPEC.md]]"
tags:
  - context-package
  - tastematter
  - foundation-fixes
  - spec-writing
---

# Tastematter - Context Package 58

## Executive Summary

Wrote all 5 implementation specs for the Foundation Fixes (Fork 1) using a 5-agent parallel team. Specs are complete and ready for implementation. Implementation Wave 2 was attempted with 2 parallel agents but **failed — zero code changes made**. Agents consumed all turns reading files and never got to editing. A linker lock also blocked `cargo test`. Next session should implement directly (single agent, no delegation).

## Session Activity

### 1. Wave 1: Spec Writing (5 agents, ~10 min, SUCCESS)

Spawned 5 general-purpose agents in parallel, each writing one spec. All completed successfully with no conflicts.

| Agent | Spec | Bug ID | Lines | Status |
|-------|------|--------|-------|--------|
| spec-path-norm | 02_PATH_NORMALIZATION_SPEC.md | LIVE-01 | 340 | COMPLETE |
| spec-chain-names | 03_CHAIN_NAMES_CLI_SPEC.md | LIVE-02 | 520 | COMPLETE |
| spec-persist-chains | 04_NON_DESTRUCTIVE_CHAINS_SPEC.md | BUG-07 | 480 | COMPLETE |
| spec-schema | 05_SCHEMA_UNIFICATION_SPEC.md | XCHECK-1, BUG-09 | 510 | COMPLETE |
| spec-files-written | 06_FILES_WRITTEN_QUERIES_SPEC.md | BUG-05/06/10 | 530 | COMPLETE |

All specs at: `specs/implementation/phase_04_core_improvements/`

### 2. Wave 2: Implementation (2 agents, ~30 min, FAILED)

Spawned 2 implementation agents to work in parallel on different files:

| Agent | Task | Files | Result |
|-------|------|-------|--------|
| impl-parser | Spec 02 (paths) + Spec 05 (schema) | jsonl_parser.rs, storage.rs, cache.rs | **ZERO code changes** |
| impl-chains | Spec 04 (non-destructive chains) | query.rs | **ZERO code changes** |

**Why they failed:**
- Both agents spent all their turns reading specs and source files
- The codebase is large enough that reading + understanding consumed the full turn budget
- `cargo test` was also blocked by linker error LNK1104 (debug exe locked by running daemon or prior cargo process)
- Lesson: Implementation agents for this codebase need to be given pre-read context, not told to read from scratch

### 3. Linker Lock Issue

```
LINK : fatal error LNK1104: cannot open file 'target\debug\deps\tastematter-cea67b929a91f870.exe'
```

Likely cause: daemon process or prior `cargo test` holding the file lock. Fixed by `taskkill /F /IM tastematter.exe`. Next agent should verify `cargo test` passes before starting implementation.

## Specs Summary

### Spec 02: Path Normalization (LIVE-01)
- **Problem:** Same file stored as relative AND absolute path, double-counted everywhere
- **Fix:** Add `normalize_file_path(raw, project_path)` in jsonl_parser.rs, called during `aggregate_session()`
- **Key detail:** Normalize at aggregation time (not extraction), because `parse_jsonl_line()` has no project_path
- **Tests:** 8 (7 unit + 1 integration)

### Spec 03: Chain Names CLI (LIVE-02)
- **Problem:** CLI shows hex chain IDs, no human-readable names
- **Fix:** Add `display_name` to ChainData, `chain_name` to SessionData, 3-level fallback (generated_name → first_user_message → chain_id[:12])
- **Depends on:** Spec 05 (schema) — but can work independently since generated_name exists in storage.rs schema
- **Tests:** 5

### Spec 04: Non-Destructive Chains (BUG-07)
- **Problem:** `persist_chains()` DROP+recreates tables every sync, queries return empty during window
- **Fix:** Remove DROP/CREATE, use INSERT OR REPLACE + DELETE stale entries, wrap in transaction
- **Key detail:** Also needs chain_graph schema updated to include parent_session_id, is_root, indexed_at
- **Tests:** 5

### Spec 05: Schema Unification (XCHECK-1)
- **Problem:** chain_metadata has 2 incompatible definitions (storage.rs vs cache.rs)
- **Fix:** Merge all columns into storage.rs, remove from cache.rs, add ALTER TABLE migration
- **Canonical schema:** chain_id, generated_name, summary, key_topics, category, confidence, generated_at, model_used, created_at, updated_at
- **Tests:** 5

### Spec 06: Files Written Queries (BUG-05/06/10)
- **Problem:** All 7 query functions only use `json_each(s.files_read)`, never files_written
- **Fix:** CTE with UNION ALL across both JSON arrays, applied to all 7 query functions
- **Depends on:** Spec 02 (path normalization must be applied first)
- **Tests:** 8

## Dependency Graph (for implementation)

```
Spec 05 (schema)  ──must-before──►  Spec 03 (chain names)
  storage.rs, cache.rs                query.rs, main.rs, types.rs

Spec 02 (paths)   ──must-before──►  DB re-sync  ──must-before──►  Spec 06 (files_written)
  jsonl_parser.rs                                                    query.rs (7 queries)

Spec 04 (persist_chains)  ──independent──  Spec 02, Spec 05
  query.rs:1418-1507

CONFLICT ZONE: Spec 03, Spec 04, Spec 06 all edit query.rs
```

**Recommended implementation order:**
1. Spec 05 (schema) + Spec 02 (paths) — different files, parallel-safe
2. Spec 04 (non-destructive chains) — can be parallel with above IF careful about query.rs
3. Re-sync database with `cargo run --release -- daemon once`
4. Spec 03 (chain names) — query.rs + main.rs + types.rs
5. Spec 06 (files_written) — query.rs (7 functions)

## Local Problem Set

### Completed This Session
- [x] Write all 5 implementation specs [VERIFIED: 5 files in specs/implementation/phase_04_core_improvements/]

### NOT Completed (Carry Forward)
- [ ] **Implement Spec 02** (path normalization) — jsonl_parser.rs
- [ ] **Implement Spec 05** (schema unification) — storage.rs, cache.rs
- [ ] **Implement Spec 04** (non-destructive persist_chains) — query.rs
- [ ] **Re-sync database** — `cargo run --release -- daemon once`
- [ ] **Implement Spec 03** (chain names CLI) — query.rs, main.rs, types.rs
- [ ] **Implement Spec 06** (files_written queries) — query.rs (7 functions)
- [ ] **Release v0.1.0-alpha.18** — cargo test + clippy + tag

## Key Files

| File | Purpose | Status |
|------|---------|--------|
| [[specs/implementation/phase_04_core_improvements/02_PATH_NORMALIZATION_SPEC.md]] | Path normalization spec | Written |
| [[specs/implementation/phase_04_core_improvements/03_CHAIN_NAMES_CLI_SPEC.md]] | Chain names CLI spec | Written |
| [[specs/implementation/phase_04_core_improvements/04_NON_DESTRUCTIVE_CHAINS_SPEC.md]] | Non-destructive chains spec | Written |
| [[specs/implementation/phase_04_core_improvements/05_SCHEMA_UNIFICATION_SPEC.md]] | Schema unification spec | Written |
| [[specs/implementation/phase_04_core_improvements/06_FILES_WRITTEN_QUERIES_SPEC.md]] | Files written queries spec | Written |
| [[core/src/capture/jsonl_parser.rs]] | Parser (Spec 02 target) | Needs changes |
| [[core/src/storage.rs]] | DB schema (Spec 05 target) | Needs changes |
| [[core/src/intelligence/cache.rs]] | Intel cache (Spec 05 target) | Needs changes |
| [[core/src/query.rs]] | Queries (Spec 03, 04, 06 target) | Needs changes |
| [[core/src/main.rs]] | CLI (Spec 03 target) | Needs changes |

## Test State

- Rust tests: **BLOCKED by linker lock** last attempt. Kill tastematter.exe first, then `cd core && cargo test`
- Expected baseline: ~287 tests passing (from package 56)
- No source code was modified this session — baseline should be unchanged

## For Next Agent

**Context Chain:**
- Previous: [[57_2026-02-06_FULL_CODEBASE_AUDIT_COMPLETE]] (audit + strategic fork)
- This package: All 5 implementation specs written, zero code changes
- Next action: Implement all 5 specs

**Start here:**
1. Read this context package
2. Kill any running tastematter: `taskkill /F /IM tastematter.exe`
3. Run `cd core && cargo test` to verify baseline passes
4. Read each spec in order: 05 → 02 → 04 → 03 → 06
5. Implement each spec following its Implementation Steps section
6. Run `cargo test` and `cargo clippy -- -D warnings` after each spec

**Implementation strategy (single agent, NOT team):**
- Implement directly — don't delegate to sub-agents (they fail on this codebase size)
- Do Spec 05 first (schema) — it's the simplest (schema DDL changes) and unblocks Spec 03
- Do Spec 02 second (path normalization) — it's self-contained in jsonl_parser.rs
- Do Spec 04 third (non-destructive chains) — isolated to one function in query.rs
- Do Spec 03 fourth (chain names) — touches query.rs + main.rs + types.rs
- Do Spec 06 last (files_written) — touches 7 query functions in query.rs, biggest change

**Do NOT:**
- Use agent teams for implementation (they burn context reading without editing)
- Skip running `cargo test` between specs (catch regressions early)
- Edit query.rs for multiple specs simultaneously (merge conflicts)
- Expand scope beyond these 5 specs (no parser depth, no new features)

**Lesson learned this session:**
Implementation agents for large Rust codebases fail when they need to read specs + source code + compile + test all within their turn budget. The reading alone consumed all turns. Next time: either pre-digest the context into the prompt, or implement directly in the main session.
