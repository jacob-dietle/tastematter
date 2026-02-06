---
title: "Tastematter Context Package 54"
package_number: 54
date: 2026-02-05
status: superseded
previous_package: "[[53_2026-02-04_CORE_AUDIT_AND_DATA_QUALITY_RCA]]"
related:
  - "[[core/src/capture/jsonl_parser.rs]]"
  - "[[core/src/query.rs]]"
  - "[[core/src/daemon/sync.rs]]"
  - "[[core/src/types.rs]]"
  - "[[core/src/storage.rs]]"
  - "[[specs/canonical/07_CLAUDE_CODE_DATA_MODEL.md]]"
tags:
  - context-package
  - tastematter
  - data-quality
  - heat-score
  - dq-002
---

# Tastematter - Context Package 54

## Executive Summary

Implemented 3-fix compound fix for DQ-002 (phantom empty sessions in query output), then went to ground on e2e pipeline analysis. Fixes work — phantom sessions no longer created, incremental sync operational, 72 stale phantom records cleaned from DB. **Major new discovery:** heat scores are fundamentally broken due to two data source problems: file-history-snapshot pollution (79% of file entries) and Skill tool blindness (129 invocations/week invisible to index).

## What Was Fixed (DQ-002)

### Root Cause (Verified)

Summary-only JSONL files (29 files) and system-only files (8 files) produced empty `messages` vectors from `parse_session_file()` because `parse_jsonl_line` returns `None` for `summary` and `system` record types. `aggregate_session()` was called anyway, creating DB records with `started_at = Utc::now()` (fallback at line 595), `duration_seconds = 0`, `files_read = []`. [VERIFIED: [[jsonl_parser.rs]]:485, line 595]

These 72 phantom records all had `started_at = 2026-02-05T03:28:28` (the last pre-fix sync time) and `file_size_bytes = NULL`, filling the top of every `ORDER BY started_at DESC LIMIT 50` query. [VERIFIED: DB query showing 45 of 50 results were phantom]

### Fix 1: Skip Empty-Message Sessions
**File:** [[jsonl_parser.rs]]:838-842
After `parse_session_file()`, skip if `messages.is_empty()`. Prevents phantom records from summary-only, system-only, and empty JSONL files (~39 files affected). [VERIFIED: cargo test passing]

### Fix 2: Incremental Sync
**File:** [[sync.rs]]:150-156
Replaced `HashMap::new()` TODO with `engine.get_session_file_sizes().await`. Sessions with unchanged JSONL file size are skipped. [VERIFIED: `daemon once` parses 4 sessions instead of 900+]

**File:** [[query.rs]]:1372-1391
New method `get_session_file_sizes()` — queries `session_id, file_size_bytes FROM claude_sessions WHERE file_size_bytes IS NOT NULL`. [VERIFIED: cargo test passing]

### Fix 3: Persist file_size_bytes
**File:** [[types.rs]]:505-506 — Added `file_size_bytes: Option<i64>` to `SessionInput`
**File:** [[types.rs]]:536 — Added to `From<SessionSummary>` impl
**File:** [[query.rs]]:1215-1239, 1339-1362 — Added to both `insert_session()` and `upsert_session()` SQL
**File:** [[storage.rs]]:450-466,490 — Updated test schema and test data
[VERIFIED: Schema already had column at [[storage.rs]]:161, just wasn't being written]

### Cleanup
Deleted 72 phantom records: `DELETE FROM claude_sessions WHERE (total_messages = 0 OR total_messages IS NULL)`. [VERIFIED: query sessions --time 7d now returns real sessions]

### Regression Tests Added
- `test_summary_only_session_is_skipped` — [[jsonl_parser.rs]]:1545-1557
- `test_session_with_tools_retains_timestamp` — [[jsonl_parser.rs]]:1560-1588
- `cargo clippy -- -D warnings` passes clean [VERIFIED: 2026-02-05]

## What Was Discovered (Heat Score RCA)

### Problem 1: file-history-snapshot Pollution

**The numbers (7d window):**
- 18,911 file-history-snapshot entries vs 989 Read tool invocations
- 3,265 file entries from snapshot-only sessions vs 353 from real sessions
- **79% of heat signal is Claude Code's internal file versioning, not user work**

[VERIFIED: SQL aggregate query on claude_sessions, tools_used LIKE '%file-history-snapshot%']

**Mechanism:** `parse_jsonl_line` returns `Some(ParsedMessage)` for `file-history-snapshot` records (Source 3 in 3-source extraction at [[jsonl_parser.rs]]:479). `extract_from_snapshot` creates ToolUse entries with `is_read: true` for each `trackedFileBackups` key. These count identically to intentional Read tool_use in the heat computation.

**Example:** `context-analyzer/SKILL.md` shows as HOT (5 accesses, score 1.000). 5 of 6 sessions referencing it are snapshot-only (`tools: {'file-history-snapshot': 1047}`). User never intentionally read it. [VERIFIED: session query showing tools_used for each]

### Problem 2: Skill Tool Blindness

**The numbers:** 129 Skill tool invocations in 7 days produce zero file paths in the index.

**Mechanism:** When user runs `/context-package`, JSONL records `tool_use: {name: "Skill", input: {skill: "context-package"}}`. Extraction at [[jsonl_parser.rs]]:471-486 dispatches to `extract_from_assistant` which looks for `input.file_path`, `input.path`, `input.notebook_path`. The Skill tool has `input.skill` — none match. The SKILL.md loaded into context is invisible.

**Impact:** `context-package` — used every session — shows zero heat in 7d. `context-foundation` — used at session start — shows zero heat in 7d. The user's most-used files are completely absent from the heat map. [VERIFIED: query showing 0 sessions referencing context-package/SKILL.md in 7d]

### Disk Reality (JSONL Classification)

| Category | Files | `messages` empty? | Notes |
|----------|-------|-------------------|-------|
| Real sessions (user/assistant) | 633 | No | Core data, working correctly |
| Snapshot+summary (has backups) | 278 | No | Pollutes heat — treated as real reads |
| Snapshot+summary (empty backups) | 15 | No | Low impact, empty tool_uses |
| Summary-only | 29 | **Yes** | Fixed by DQ-002 Fix 1 |
| System/mixed | ~8 | **Yes** | Fixed by DQ-002 Fix 1 |
| Empty (0 bytes) | 2 | **Yes** | Fixed by DQ-002 Fix 1 |
| **Total** | **972** | | |

### What the Plan Got Wrong

| Plan Claim | Reality |
|------------|---------|
| "700+ phantom sessions" | 72 phantom records (40 on disk, 32 ghost DB records) |
| "JSONL compaction removes records" | Claude Code doesn't remove records during compaction — adds summaries |
| "Timestamp redistribution is primary cause" | Snapshot sessions get correct timestamps from `snapshot.timestamp`. Phantoms get `Utc::now()` because they have zero timestamps. |
| "All recent sessions empty" | 134 sessions in 7d window had file data. Phantoms filled LIMIT 50 first. |

## Current DB State (Post-Fix)

| Metric | Value |
|--------|-------|
| Total sessions | 1,112 |
| Sessions with files_read data | 670 |
| Sessions with messages but no files | 442 (legitimate — Bash/WebSearch/MCP sessions) |
| Phantom records | 0 (cleaned) |
| Sessions with file_size_bytes | 1,017 |
| Sessions with file_size_bytes NULL | 95 (pre-fix, will self-heal on next re-parse) |

## Jobs To Be Done (Next Session)

### HIGH: Fix Heat Score Data Source Quality
1. [ ] **Filter or weight-down file-history-snapshot entries** — These are metadata about backups, not user intent. Options: (a) exclude snapshot-only sessions from heat entirely, (b) weight snapshot accesses at 0.1x vs Read/Edit at 1.0x, (c) add `access_source` field and let query filter.
   - Success criteria: context-analyzer drops from HOT, real work files dominate
   - Files: [[jsonl_parser.rs]] (aggregate_session or new field), [[query.rs]] (heat SQL)

2. [ ] **Extract file paths from Skill tool invocations** — Map `input.skill` → `.claude/skills/{skill}/SKILL.md` plus any `references/` directory contents loaded.
   - Success criteria: context-package/SKILL.md shows in heat after skill invocations
   - Files: [[jsonl_parser.rs]] (extract_from_assistant, new tool handler)

### MEDIUM: Query Layer Improvements
3. [ ] **Populate `access_types`** — Currently always `[]` with TODO comment at [[query.rs]]:503. Need to surface Read vs Write vs Create distinction.
4. [ ] **Add co-access query** — "Given file X, what files co-occur with it?" The inverted index data exists (session→files, file→sessions), but no API exposes the join.

### LOW: Cleanup
5. [ ] Pre-existing test failures unrelated to this work: `test_load_workstreams_from_real_yaml` (expects 'tastematter-product' not in list), `test_batch_insert_commits_performance` (3725ms > 1000ms threshold on Windows)

## File Locations

| File | Purpose | Status |
|------|---------|--------|
| [[core/src/capture/jsonl_parser.rs]] | Fix 1 + regression tests | Modified |
| [[core/src/daemon/sync.rs]] | Fix 2 (incremental sync) | Modified |
| [[core/src/query.rs]] | Fix 3 (file_size_bytes) + get_session_file_sizes() | Modified |
| [[core/src/types.rs]] | Fix 3 (SessionInput field) | Modified |
| [[core/src/storage.rs]] | Test schema update | Modified |
| [[specs/canonical/07_CLAUDE_CODE_DATA_MODEL.md]] | Reference spec (unchanged) | Reference |

## Test State

- Tests: 280 passing, 3 failing (all 3 pre-existing, unrelated to this work)
- `cargo clippy -- -D warnings`: clean
- `cargo build --release`: success
- Command: `cd core && cargo test`
- Last run: 2026-02-05
- [VERIFIED: test output captured during session]

## For Next Agent

**Context Chain:**
- Previous: [[53_2026-02-04_CORE_AUDIT_AND_DATA_QUALITY_RCA]] (audit identified DQ-002)
- This package: DQ-002 fixed + heat score RCA complete
- Next action: Fix heat score data source quality (Job 1 + Job 2 above)

**Start here:**
1. Read this context package
2. Read [[specs/canonical/07_CLAUDE_CODE_DATA_MODEL.md]] for JSONL record types
3. Read [[core/src/capture/jsonl_parser.rs]] lines 358-384 (extract_from_snapshot) and 471-495 (parse_jsonl_line dispatch)
4. Read [[core/src/query.rs]] lines 853-932 (query_heat SQL)
5. Run: `cd core && cargo test` to confirm baseline

**Key insight:**
The 3-source extraction (assistant tool_use, user toolUseResult, file-history-snapshot) was designed for completeness but lacks quality weighting. Source 3 (snapshots) dominates the signal because Claude Code backs up hundreds of files per session. Heat computation treats all sources equally. Fix requires either filtering snapshot sources or weighting them differently. [VERIFIED: 79% snapshot ratio from DB analysis]
