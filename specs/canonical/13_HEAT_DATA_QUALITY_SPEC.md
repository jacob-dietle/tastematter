---
title: "Heat Score Data Quality Specification"
type: architecture-spec
created: 2026-02-05
status: approved
foundation:
  - "[[canonical/07_CLAUDE_CODE_DATA_MODEL.md]]"
  - "[[context_packages/04_daemon/54_2026-02-05_DQ002_FIX_AND_HEAT_SCORE_RCA]]"
related:
  - "[[core/src/capture/jsonl_parser.rs]]"
  - "[[core/src/query.rs]]"
  - "[[core/src/types.rs]]"
tags:
  - tastematter
  - heat-score
  - data-quality
  - canonical
---

# Heat Score Data Quality Specification

## Problem Statement

The heat score feature answers: "What files should I load into context right now?"

It currently answers a different question: "What files appeared in any session?" — because `files_read` conflates three distinct signals:

| Signal | Source | Intent Level | Volume (7d) | Current Treatment |
|--------|--------|-------------|-------------|-------------------|
| User-initiated reads | `Read`, `Edit`, `Grep`, `Glob` tool_use | HIGH | 989 entries | Counted in files_read |
| Snapshot backups | `file-history-snapshot` trackedFileBackups | ZERO | 18,911 entries | Counted in files_read |
| Skill file loads | `Skill` tool_use | HIGH | 129 invocations | NOT counted (invisible) |

**Result:** 79% of heat signal is noise. The hottest file (`context-analyzer/SKILL.md`) was never intentionally accessed. The most-used skill file (`context-package/SKILL.md`) is invisible.

[SOURCE: [[54_2026-02-05_DQ002_FIX_AND_HEAT_SCORE_RCA]], verified via DB aggregate queries]

## Root Cause

The data quality problem originates in `aggregate_session()` at `jsonl_parser.rs:508-637`. This function iterates all `ToolUse` entries from all three extraction sources and inserts file paths into a single `HashSet<String>`:

```rust
// Current: ALL sources treated identically
for tool_use in &msg.tool_uses {
    if let Some(ref path) = tool_use.file_path {
        if tool_use.is_read {
            files_read_set.insert(path.clone());  // snapshot + real → same bucket
        }
    }
}
```

The `file-history-snapshot` source creates `ToolUse` entries with `name: "file-history-snapshot"` and `is_read: true` at `jsonl_parser.rs:372-380`. These are indistinguishable from real reads once they enter `files_read_set`.

Separately, `extract_file_path()` at `jsonl_parser.rs:208-233` has no handler for the `Skill` tool. It checks `input.file_path`, `input.notebook_path`, `input.path` — but Skill tool uses `input.skill`. Result: `file_path: None`, so Skill invocations produce `ToolUse` entries with no file path.

## Design Decision

**Fix at the data source (aggregate_session), not at query time.**

Rationale:
- `files_read` should mean "files the user interacted with" — this is the correct semantic
- Every downstream consumer (heat, flex, sessions, co-access, future features) benefits from clean data
- No schema change required — `files_read` stays as `Vec<String>`
- JSONL files on disk are immutable — if we need snapshot data later, re-sync with different logic
- `tools_used` JSON column already preserves snapshot counts per session for any future use

Trade-off accepted: Requires full re-sync (delete sessions from DB, run `daemon once`). This is a one-time operation.

## Changes

### Change 1: Exclude snapshot-only paths from files_read

**File:** `core/src/capture/jsonl_parser.rs`
**Function:** `aggregate_session()`
**Lines:** ~562-592 (the tool_use processing loop)

**Logic change:**

Track snapshot file paths and real file paths in separate sets during iteration. At the end, `files_read` = paths that appeared from at least one non-snapshot source.

```
Before:
  for each tool_use:
    if is_read and has file_path → files_read_set.insert(path)

After:
  for each tool_use:
    if tool_use.name == "file-history-snapshot":
      snapshot_paths.insert(path)
    else if is_read and has file_path:
      files_read_set.insert(path)
```

Snapshot paths that ALSO appear from a real tool are already in `files_read_set` via the else branch. Snapshot-only paths stay in `snapshot_paths` and are discarded.

**What this preserves:**
- `tools_used` still counts `{"file-history-snapshot": N}` — unchanged
- `total_messages` still counts snapshot messages — unchanged
- Timestamps from snapshot records still contribute to `started_at`/`ended_at` — unchanged

**What this changes:**
- `files_read` no longer contains snapshot-only paths
- Sessions that only had snapshot activity will have `files_read: []`
- `access_count` in downstream queries drops (79% reduction in noise entries)
- Heat scores change completely (the point of this fix)

### Change 2: Extract file paths from Skill tool invocations

**File:** `core/src/capture/jsonl_parser.rs`
**Function:** `extract_file_path()`
**Lines:** ~208-233

**Logic change:**

Add a handler for the `Skill` tool name that maps `input.skill` to the SKILL.md file path.

```
Before:
  if tool_name == "Grep" → GREP:pattern
  if tool_name == "Glob" → GLOB:pattern
  try input.file_path, input.notebook_path, input.path

After:
  if tool_name == "Grep" → GREP:pattern
  if tool_name == "Glob" → GLOB:pattern
  if tool_name == "Skill" → .claude/skills/{input.skill}/SKILL.md
  try input.file_path, input.notebook_path, input.path
```

**JSONL evidence (verified):**
```json
{
  "type": "tool_use",
  "id": "toolu_01AAMfbUFeuvLh2ZfzBXixtS",
  "name": "Skill",
  "input": {
    "skill": "context-query",
    "args": "tastematter product strategy..."
  }
}
```

**Mapping:** `input.skill = "context-query"` → `.claude/skills/context-query/SKILL.md`

**Classification:** Skill loads files into context. This is a read operation. The `is_read_tool` check at `jsonl_parser.rs:24` needs `"Skill"` added to `READ_TOOLS`.

**What this does NOT handle (conscious scope limit):**
- `references/` directory contents loaded by skills — these vary per skill and aren't recorded in the JSONL. We can't know which reference files were loaded without parsing the skill itself. Defer to a future enhancement.
- `Task` tool (subagent spawning) — records `input.prompt` but no file path. The subagent's own session captures its file access. No change needed.
- `mcp__*` tools — MCP tools have diverse input schemas. Each would need its own handler. Defer.

## Test Plan

### Unit Tests

**Test 1: `test_snapshot_paths_excluded_from_files_read`**
- Create messages with both snapshot and Read tool_uses for the same file
- Create messages with snapshot-only tool_uses for a different file
- Call `aggregate_session()`
- Assert: shared file IS in `files_read`, snapshot-only file is NOT

**Test 2: `test_snapshot_only_session_has_empty_files_read`**
- Create messages with only `file-history-snapshot` tool_uses
- Call `aggregate_session()`
- Assert: `files_read` is empty, `tools_used` still has `file-history-snapshot` count

**Test 3: `test_skill_tool_extracts_file_path`**
- Call `extract_file_path("Skill", json!({"skill": "context-package"}))`
- Assert: returns `Some(".claude/skills/context-package/SKILL.md")`

**Test 4: `test_skill_tool_without_skill_field_returns_none`**
- Call `extract_file_path("Skill", json!({"args": "something"}))`
- Assert: returns `None`

**Test 5: `test_skill_in_read_tools`**
- Assert: `is_read_tool("Skill")` returns `true`

### Integration Verification (Post Re-sync)

After implementation, run:

```bash
# 1. Delete existing sessions
# (SQL: DELETE FROM claude_sessions)

# 2. Re-sync
tastematter daemon once

# 3. Verify heat
tastematter query heat --time 7d

# Expected: context-analyzer/SKILL.md NOT in top results
# Expected: real work files (jsonl_parser.rs, query.rs) dominate

# 4. Verify Skill visibility
# Expected: .claude/skills/context-package/SKILL.md appears in heat

# 5. Verify session counts
tastematter query sessions --time 7d
# Expected: sessions with files_read data, no phantoms
```

## Impact Assessment

| Dimension | Before | After |
|-----------|--------|-------|
| files_read signal quality | 21% real, 79% snapshot noise | ~100% real user intent |
| Skill tool visibility | 0 file paths from 129 invocations | 129 SKILL.md paths |
| Heat top results | context-analyzer (never used) | actual work files |
| Sessions with files_read | 670 | ~500 (snapshot-only sessions drop) |
| Parity with Python | Exact match | Diverges (intentional — Python had same bug) |
| tools_used accuracy | Unchanged | Unchanged |
| Schema | Unchanged | Unchanged |

## Parity Test Impact

The 27 Python-vs-Rust parity tests in `core/tests/parity/` compare output counts. These will fail because:
- Rust `files_read` will be smaller (no snapshot paths)
- Rust `files_read` will include Skill paths (Python doesn't extract these)

**Resolution:** The Python indexer is being replaced. Update parity tests to use Rust-only baselines, or mark snapshot/Skill parity tests as `#[ignore]` with a comment explaining the intentional divergence.

## Files to Modify

| File | Change | Lines (est.) |
|------|--------|-------------|
| `core/src/capture/jsonl_parser.rs` | Change 1: snapshot filtering in `aggregate_session` | ~10 |
| `core/src/capture/jsonl_parser.rs` | Change 2: Skill handler in `extract_file_path` | ~8 |
| `core/src/capture/jsonl_parser.rs` | Add "Skill" to `READ_TOOLS` constant | 1 |
| `core/src/capture/jsonl_parser.rs` | 5 new unit tests | ~80 |
| Parity tests (if they exist as separate files) | Update or ignore | ~5 |
| **Total** | | **~104 lines** |

## Verification Criteria

1. `cargo test` — all new tests pass, no regressions (except known parity divergence)
2. `cargo clippy -- -D warnings` — clean
3. After re-sync: `tastematter query heat --time 7d` returns files the user actually worked on
4. After re-sync: `.claude/skills/context-package/SKILL.md` appears in heat results
5. `tools_used` column still contains `file-history-snapshot` counts (data preserved)

## Sequence

1. Add `"Skill"` to `READ_TOOLS` (1 line)
2. Add Skill handler to `extract_file_path` (~8 lines)
3. Modify `aggregate_session` to exclude snapshot-only paths (~10 lines)
4. Write tests (~80 lines)
5. Run `cargo test` + `cargo clippy`
6. Delete existing sessions from DB
7. Run `tastematter daemon once`
8. Verify heat output
9. Write context package
