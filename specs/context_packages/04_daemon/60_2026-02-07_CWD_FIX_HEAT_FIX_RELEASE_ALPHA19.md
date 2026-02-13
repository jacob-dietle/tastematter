---
title: "Tastematter Context Package 60"
package_number: 60
date: 2026-02-07
status: current
previous_package: "[[59_2026-02-06_ALL_FOUNDATION_FIXES_IMPLEMENTED]]"
related:
  - "[[specs/implementation/phase_04_core_improvements/01_HEAT_COMMAND_SPEC.md]]"
  - "[[specs/implementation/phase_04_core_improvements/02_PATH_NORMALIZATION_SPEC.md]]"
  - "[[core/src/capture/jsonl_parser.rs]]"
  - "[[core/src/query.rs]]"
tags:
  - context-package
  - tastematter
  - bugfix
  - release
---

# Tastematter - Context Package 60

## Executive Summary

Two critical bugs discovered and fixed via epistemic grounding + UX smoke testing. Released as v0.1.0-alpha.18 (cwd fix) and v0.1.0-alpha.19 (heat fix). 307 tests passing, all CI/staging/production pipelines green on 3 platforms.

## Session Activity

### 1. Path Normalization Was a No-Op (CRITICAL BUG)

**Discovery:** Epistemic grounding skill flagged assumption that `decode_project_path` returns the correct project path. Investigation revealed it's lossy — replaces ALL dashes with backslashes, making the decoded path wrong for any project with spaces/underscores.

**Root cause:** `extract_project_path_from_file()` decodes the JSONL directory slug `C--Users-dietl-VSCode-Projects-taste-systems-gtm-operating-system` into `C:\Users\dietl\VSCode\Projects\taste\systems\gtm\operating\system` — wrong path. Real path: `C:\Users\dietl\VSCode Projects\taste_systems\gtm_operating_system`.

Since `normalize_file_path()` compares the file path prefix against this wrong project path, the prefix never matches, and all paths stay absolute. **Path normalization from Spec 02 was silently a no-op.**

**Fix:** Extract `cwd` field from the first JSONL record in `parse_session_file()`. JSONL records have accurate `cwd` field per [[07_CLAUDE_CODE_DATA_MODEL_V2.md]]. Use `cwd` for `project_path`, fall back to lossy `decode_project_path` only if `cwd` is absent.

**Changes:**
- `parse_session_file()` now returns `(Vec<ParsedMessage>, i64, Option<String>)` — third element is `cwd`
- `sync_sessions()` uses `cwd` over `extract_project_path_from_file()`

**Verification:** After DB delete + full resync:
- Before: `C:/Users/dietl/VSCode Projects/.../query.rs` (absolute)
- After: `apps/tastematter/core/src/query.rs` (relative)
- 1,126 sessions, 453 chains, 4,401 files indexed in 36s

### 2. Heat Command Returned Wrong Files (HIGH BUG)

**Discovery:** UX smoke testing of every CLI command. Heat output showed ONLY `.claude/` files — no project source files, no specs, no scripts.

**Root cause:** SQL query had `GROUP BY af.file_path LIMIT 50` with no ORDER BY. SQLite returned arbitrary rows (alphabetically first: `.claude/*`). Heat score computation happened in Rust AFTER the SQL LIMIT had already truncated the result set.

**Architecture analysis (devops-architecture-perspectives):** Heat scoring requires `velocity` and `rcr` which are computed in Rust, not SQL. SQL cannot know which rows will have highest heat score. Measured 1,670 unique files at 162ms — removing SQL LIMIT is safe.

**Fix:** Removed `LIMIT {limit}` from SQL, added `items.truncate(limit as usize)` after Rust-side sort. 1 file, 4 insertions, 3 deletions.

**Verification:**
- Before: Top 15 = all `.claude/skills/*.md` (score 0.06-0.76)
- After: Top 15 = nickel worker files, linkedin provider, audit reports (score 0.90-0.94)

### 3. CI Test Fix

`test_load_workstreams_from_real_yaml` depended on local `workstreams.yaml` not present in CI runners. Marked `#[ignore]`.

### 4. Releases

| Version | Commits | Key Changes |
|---------|---------|-------------|
| v0.1.0-alpha.18 | 621e5e8, 534fab1, 115cf35 | Foundation fixes + cwd extraction + CI fix |
| v0.1.0-alpha.19 | ef2741c | Heat LIMIT fix |

Both releases: CI green, staging smoke tests pass (Windows/macOS/Ubuntu), production verified via `latest.txt`.

## UX Evaluation (Full CLI Smoke Test)

| Command | Verdict | Notes |
|---------|---------|-------|
| `query flex --files` | Good | Relative paths, fast |
| `query flex --time` | Good | Time windowing correct |
| `query search` | Good | 20 hubspot results, cross-project |
| `query file` | Good | Session history with chain_id |
| `query co-access` | Excellent | PMI scores sensible |
| `query chains --format table` | Good | display_name working |
| `query chains --format json` | Good | Full data for agents |
| `query heat` | **Fixed** | Was broken, now correct |
| `query sessions` | OK | chain_name absent (needs Intel) |
| `query timeline` | Good | Daily buckets with session lists |

### Remaining UX Issues (Not Blocking)

| # | Severity | Issue |
|---|----------|-------|
| B2 | MEDIUM | `query verify` returns NOT_FOUND (receipt ledger not implemented) |
| B3 | LOW | Paths outside project still absolute (correct but confusing) |
| B4 | LOW | Chain names contain raw XML tags (`<command-message>`, etc.) |
| D1 | DOC | Skill doc says `query session <id>` but CLI is `query sessions --chain` |

## Test State

- **Unit tests:** 307 passing, 0 failed, 4 ignored
- **Integration tests:** 7 passing, 5 flaky (latency assertions under load, pre-existing)
- **Clippy:** Clean (`-D warnings`)
- **Cargo fmt:** Clean
- **CI:** All green on GitHub Actions

## File Locations

| File | Changes | Purpose |
|------|---------|---------|
| [[core/src/capture/jsonl_parser.rs]] | +195/-4 | `cwd` extraction from JSONL, `parse_session_file` returns 3-tuple |
| [[core/src/query.rs]] | +812/-215 | Heat LIMIT fix, files_written CTE, chain names |
| [[core/src/storage.rs]] | +232/-1 | Schema unification |
| [[core/src/types.rs]] | +18 | `display_name`, `summary`, `chain_name` fields |
| [[core/src/main.rs]] | +38/-3 | `output_chains_table()` |
| [[core/src/daemon/sync.rs]] | +3/-1 | Schema version 2.2, test ignore |
| [[core/src/intelligence/cache.rs]] | +30/-30 | Remove competing schema |

## For Next Agent

**Context Chain:**
- Previous: [[59_2026-02-06_ALL_FOUNDATION_FIXES_IMPLEMENTED]] (5 foundation specs)
- This package: cwd fix + heat fix + alpha.18/19 releases + full UX eval
- Current release: v0.1.0-alpha.19

**Carry-forward items:**
1. B2: Implement receipt verification ledger (receipts generated but not stored)
2. B4: Strip XML/system tags from `first_user_message` before using as chain display_name
3. Integration test latency assertions too tight (<100ms) — consider relaxing or removing
4. `query sessions` missing `chain_name` — needs Intel service running during sync

**Start here:**
1. Read this package for current state
2. Run `tastematter daemon once` to sync latest sessions
3. Run `tastematter query heat --limit 15` to verify heat is working
4. Pick a carry-forward item or new feature

**Key insight:** Epistemic grounding before implementation caught two bugs that would have shipped silently — path normalization was a no-op, and heat returned wrong files. The skill stack (epistemic-grounding → devops-architecture-perspectives → implementation) prevented wasted work.
