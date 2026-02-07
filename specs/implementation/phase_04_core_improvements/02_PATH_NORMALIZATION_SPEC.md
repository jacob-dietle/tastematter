# Path Normalization Specification

**Status:** Proposed
**Priority:** P0 (Critical)
**Bug ID:** LIVE-01
**Estimated Effort:** 1-2 hours

---

## Problem Statement

The same file is stored under both its relative path and absolute path, causing double-counting in every downstream query (flex, heat, co-access, PMI scores).

**Evidence from live testing:**

```
$ tastematter query flex --files "*audit*"

_system\temp\code_audit_report.md        5 accesses
C:\Users\dietl\VSCode Projects\taste_systems\gtm_operating_system\_system\temp\code_audit_report.md   2 accesses
```

These are the same file. Queries return it twice with split access counts. The true count is 7, but no single row reflects that. This corrupts:

- **flex:** File appears twice, counts fragmented
- **heat:** RCR and velocity computed on partial data
- **co-access / PMI:** Co-occurrence matrix inflated (file co-occurs with itself under two identities)
- **sessions:** `files_read` JSON array contains duplicate entries for the same physical file

---

## Root Cause Analysis

Three extraction functions in `core/src/capture/jsonl_parser.rs` return raw paths without normalization:

### 1. `extract_file_path()` (lines 208-238)

Returns the raw `file_path`, `notebook_path`, or `path` field from tool input JSON. Claude Code tools (Read, Write, Edit, NotebookEdit) emit absolute Windows paths like `C:\Users\dietl\VSCode Projects\taste_systems\gtm_operating_system\_system\temp\code_audit_report.md`. This function passes them through unchanged.

```rust
// Current code (line 228-230):
if let Some(path) = input.get("file_path").and_then(|v| v.as_str()) {
    return Some(path.to_string());  // Raw absolute path returned
}
```

### 2. `extract_from_tool_use_result()` (lines 308-354)

Returns the `filePath` from user message `toolUseResult` objects. Same absolute path problem:

```rust
// Current code (line 317-320):
let mut file_path = tool_use_result
    .get("filePath")
    .and_then(|v| v.as_str())
    .map(String::from);  // Raw absolute path
```

### 3. `extract_from_snapshot()` (lines 364-390)

Returns keys from the `trackedFileBackups` object. These keys can also be absolute paths:

```rust
// Current code (line 377-383):
for file_path in tracked_backups.keys() {
    tool_uses.push(ToolUse {
        file_path: Some(file_path.clone()),  // Raw key, may be absolute
        ..
    });
}
```

### Why it happens

The `project_path` is available during `sync_sessions()` (line 836-837) and passed to `aggregate_session()` (line 859). But the individual extraction functions run earlier during `parse_jsonl_line()` (line 439), which has no project path context. By the time aggregation happens, the raw absolute paths are already stored in `ToolUse.file_path` and get inserted into `files_read`/`files_written` HashSets without normalization.

The HashSet deduplication at line 592 (`files_read_set.insert(path.clone())`) correctly deduplicates identical strings, but `_system\temp\foo.md` and `C:\Users\...\foo.md` are different strings pointing to the same file.

---

## Implementation Plan

### Step 1: Add `normalize_file_path()` function

Add a new function in `jsonl_parser.rs` after the existing path encoding/decoding section (after line 183):

```rust
/// Normalize a file path to project-relative form.
///
/// Rules:
/// 1. If path starts with project_path (case-insensitive on Windows), strip it
/// 2. Convert backslashes to forward slashes for consistency
/// 3. Strip leading separator after prefix removal
/// 4. Leave pseudo-paths (GREP:, GLOB:) unchanged
/// 5. Leave already-relative paths unchanged
/// 6. Leave paths outside the project unchanged (cannot normalize)
pub fn normalize_file_path(raw_path: &str, project_path: &str) -> String {
    // Rule 4: Skip pseudo-paths
    if raw_path.starts_with("GREP:") || raw_path.starts_with("GLOB:") {
        return raw_path.to_string();
    }

    // Skip empty paths
    if raw_path.is_empty() {
        return raw_path.to_string();
    }

    // Normalize separators for comparison
    let normalized_raw = raw_path.replace('\\', "/");
    let normalized_project = project_path.replace('\\', "/");

    // Rule 1: Strip project path prefix (case-insensitive for Windows)
    let relative = if normalized_raw
        .to_lowercase()
        .starts_with(&normalized_project.to_lowercase())
    {
        let remainder = &normalized_raw[normalized_project.len()..];
        // Rule 3: Strip leading separator
        remainder.trim_start_matches('/')
    } else {
        // Rule 5/6: Already relative or outside project
        &normalized_raw
    };

    // Rule 2: Result already has forward slashes from normalization
    relative.to_string()
}
```

### Step 2: Apply normalization in `aggregate_session()`

The normalization point is `aggregate_session()` (line 514), which already receives `project_path` as a parameter. Normalize each `tool_use.file_path` before inserting into the deduplication sets.

Change the file collection loop (lines 581-601) to normalize before insertion:

```rust
// Current (line 582-593):
if let Some(ref path) = tool_use.file_path {
    if path.starts_with("GREP:") || path.starts_with("GLOB:") {
        continue;
    }
    if tool_use.name == "file-history-snapshot" {
        snapshot_paths.insert(path.clone());
    } else if tool_use.is_read {
        files_read_set.insert(path.clone());
    }
    // ...
}

// After fix:
if let Some(ref path) = tool_use.file_path {
    if path.starts_with("GREP:") || path.starts_with("GLOB:") {
        continue;
    }
    let normalized = normalize_file_path(path, project_path);
    if tool_use.name == "file-history-snapshot" {
        snapshot_paths.insert(normalized);
    } else if tool_use.is_read {
        files_read_set.insert(normalized.clone());
    }
    if tool_use.is_write {
        files_written_set.insert(normalized.clone());
        if tool_use.name == "Write" {
            files_created_set.insert(normalized);
        }
    }
}
```

### Step 3: Also normalize grep pattern extraction

The grep pattern extraction at line 575-578 should remain unchanged since pseudo-paths are not affected. But the `normalize_file_path` function already handles this via Rule 4 (returns GREP:/GLOB: paths unchanged).

### Why normalize in `aggregate_session()` and not in the extraction functions

1. **`parse_jsonl_line()` has no project_path context** -- it processes a single JSON line with no session metadata. Adding project_path would require threading it through all callers.
2. **`aggregate_session()` already receives `project_path`** -- no API change needed.
3. **Single normalization point** -- all three extraction sources flow through `aggregate_session()`, so normalizing there covers all paths with one code change.
4. **Preserves raw data** -- `ToolUse.file_path` retains the original value, which may be useful for debugging.

---

## Type Contract Changes

No new types required. The existing `normalize_file_path()` function is a pure `fn(&str, &str) -> String`. No struct changes needed.

The function signature is intentionally simple: takes raw path + project path, returns normalized path. This avoids coupling to `ToolUse` or `SessionSummary` types.

---

## TDD Test Plan

All tests go in the existing `#[cfg(test)] mod tests` block in `jsonl_parser.rs`.

### Unit Tests for `normalize_file_path()`

#### `test_normalize_absolute_to_relative`
**What:** Absolute Windows path with project prefix stripped to relative.
**Input:** `normalize_file_path("C:\\Users\\dietl\\VSCode Projects\\taste_systems\\gtm_operating_system\\_system\\temp\\foo.md", "C:\\Users\\dietl\\VSCode Projects\\taste_systems\\gtm_operating_system")`
**Expected:** `"_system/temp/foo.md"`
**Red:** Function doesn't exist yet.
**Green:** Returns project-relative path with forward slashes.

#### `test_normalize_already_relative_unchanged`
**What:** Already-relative path passes through with backslash normalization only.
**Input:** `normalize_file_path("_system\\temp\\foo.md", "C:\\Users\\dietl\\VSCode Projects\\taste_systems\\gtm_operating_system")`
**Expected:** `"_system/temp/foo.md"`
**Red:** Function doesn't exist yet.
**Green:** Returns same path with forward slashes.

#### `test_normalize_pseudo_paths_unchanged`
**What:** GREP: and GLOB: pseudo-paths are not modified.
**Input:** `normalize_file_path("GREP:some_pattern", "/any/project")`
**Expected:** `"GREP:some_pattern"`
**Input:** `normalize_file_path("GLOB:*.rs", "/any/project")`
**Expected:** `"GLOB:*.rs"`
**Red:** Function doesn't exist yet.
**Green:** Pseudo-paths returned unchanged.

#### `test_normalize_backslash_to_forward_slash`
**What:** Backslashes in relative paths converted to forward slashes.
**Input:** `normalize_file_path("src\\capture\\jsonl_parser.rs", "/some/project")`
**Expected:** `"src/capture/jsonl_parser.rs"`
**Red:** Function doesn't exist yet.
**Green:** All backslashes become forward slashes.

#### `test_normalize_outside_project_unchanged`
**What:** Absolute path outside the project is left as-is (with separator normalization).
**Input:** `normalize_file_path("D:\\Other\\Project\\file.rs", "C:\\Users\\dietl\\MyProject")`
**Expected:** `"D:/Other/Project/file.rs"`
**Red:** Function doesn't exist yet.
**Green:** Path outside project returned with forward slashes only.

#### `test_normalize_case_insensitive_windows`
**What:** Windows path comparison is case-insensitive (drive letter, directory names).
**Input:** `normalize_file_path("c:\\users\\DIETL\\vscode projects\\taste_systems\\gtm_operating_system\\foo.md", "C:\\Users\\dietl\\VSCode Projects\\taste_systems\\gtm_operating_system")`
**Expected:** `"foo.md"`
**Red:** Case-sensitive comparison would fail to strip prefix.
**Green:** Case-insensitive prefix match strips correctly.

#### `test_normalize_trailing_separator`
**What:** Project path with trailing separator handled correctly.
**Input:** `normalize_file_path("C:\\Users\\project\\foo.md", "C:\\Users\\project\\")`
**Expected:** `"foo.md"`
**Red:** Double separator or empty prefix remnant.
**Green:** Leading separator stripped cleanly.

### Integration Test

#### `test_session_dedup_after_normalization`
**What:** When a session contains the same file accessed via absolute and relative paths, `aggregate_session` deduplicates them into a single entry in `files_read`.
**Setup:** Create two `ParsedMessage` objects:
  - Message 1: `ToolUse` with `file_path = Some("C:\\Users\\dietl\\...\\foo.md")` (absolute)
  - Message 2: `ToolUse` with `file_path = Some("foo.md")` (relative)
**Call:** `aggregate_session("test", "C:\\Users\\dietl\\...\\project", &messages, 0)`
**Expected:** `summary.files_read` contains exactly one entry: `"foo.md"`
**Red:** Without normalization, both paths inserted as separate entries.
**Green:** Both paths normalize to `"foo.md"`, HashSet deduplicates.

---

## Success Criteria

### CLI Verification

After implementing and rebuilding (`cd core && cargo build --release`):

```bash
# Re-sync sessions to rebuild with normalized paths
tastematter sync --force

# Verify: each file appears exactly once
tastematter query flex --files "*audit*"
# Expected: _system/temp/code_audit_report.md appears ONCE with combined count (7)

# Verify: forward slashes in output
tastematter query flex --time 7d --limit 10
# Expected: all paths use forward slashes (e.g., _system/state/pipeline.yaml)

# Verify: heat scores not inflated by duplicates
tastematter query heat --limit 20
# Expected: no file appears twice under different path forms
```

### Automated Tests

```bash
cd core && cargo test
# All new tests pass (7 unit + 1 integration)
# All existing tests continue to pass (regression)
```

---

## Edge Cases

| Case | Handling |
|------|----------|
| Empty path | Returned unchanged (empty string) |
| Pseudo-path (GREP:, GLOB:) | Returned unchanged |
| `.claude/skills/foo/SKILL.md` (synthetic Skill path) | Already relative, passes through with forward slashes |
| Path with mixed separators (`C:/Users\dietl/...`) | Normalized to all forward slashes before comparison |
| Project path = `"unknown"` (extraction failed) | No prefix match, path returned with separator normalization only |
| Path equals project path exactly (no file component) | Returns empty string after trim -- unlikely in practice |

---

## Handoff Checklist

Before marking implementation complete, the implementing agent must verify:

- [ ] `normalize_file_path()` function added to `jsonl_parser.rs`
- [ ] `aggregate_session()` calls `normalize_file_path()` on every `tool_use.file_path`
- [ ] All 7 unit tests + 1 integration test written and passing
- [ ] All existing tests still pass (`cargo test` clean)
- [ ] `cargo build --release` succeeds
- [ ] `tastematter sync --force` completes without error on real data
- [ ] `tastematter query flex --files "*audit*"` returns each file exactly once
- [ ] Output paths use forward slashes consistently
- [ ] Binary copied to PATH if applicable

---

**Created:** 2026-02-06
**Source:** LIVE-01 from codebase audit (Context Package 57)
**Audit Reference:** `specs/audits/2026-02-06/audit_data_pipeline.md` Section 1.1
