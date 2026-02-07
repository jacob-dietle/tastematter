# Files Written Query Coverage Specification

**Status:** Proposed
**Priority:** Medium
**Bug IDs:** BUG-05, BUG-06, BUG-10
**Depends On:** Spec 02 (Path Normalization) -- paths in `files_written` must be normalized before querying

---

## Problem Statement

All seven query functions in `core/src/query.rs` only query `claude_sessions.files_read` via `json_each(s.files_read)`. The `files_written` column is populated by the parser but never queried. Files that were only written (via Edit, Write, NotebookEdit tools) but never independently read are completely invisible to every query command.

### Evidence

**BUG-05 (Medium):** `query_flex` uses `json_each(s.files_read)` at line 66. Files only present in `files_written` are excluded from flex results.
[VERIFIED: `core/src/query.rs:66`]

**BUG-06 (Medium):** `query_heat` uses `json_each(s.files_read)` at line 875. Write-heavy files (frequently edited but rarely read independently) have artificially low heat scores.
[VERIFIED: `core/src/query.rs:875`]

**BUG-10 (Medium):** `query_sessions` file_count computed from `json_each(s.files_read)` only at lines 433-438. Sessions that primarily wrote files show artificially low file counts.
[VERIFIED: `core/src/query.rs:433-438`]

**Additional affected functions (not individually bug-tracked):**
- `query_search` (line 591-639): uses `json_each(s.files_read)` only [VERIFIED: `core/src/query.rs:601`]
- `query_file` (line 641-755): all three match attempts (exact, suffix, substring) search only `files_read` [VERIFIED: `core/src/query.rs:655`, `686`, `714`]
- `query_co_access` (line 757-841): both session-finding and co-access queries search only `files_read` [VERIFIED: `core/src/query.rs:771`, `801`]
- `query_timeline` (line 206-411): bucket, file, and per-file-bucket queries all use `files_read` only [VERIFIED: `core/src/query.rs:224`, `278`, `317`]

**Cross-check confirmation:** `cross_check_data_pipeline.md` section 6.2 confirms this is a data quality issue visible to the frontend -- `SessionData.file_count` only counts read files.
[VERIFIED: `specs/audits/2026-02-06/cross_check_data_pipeline.md:216`]

**GAP-10 from audit:** "files_written never queried by any query function. Write-only file access patterns are invisible."
[VERIFIED: `specs/audits/2026-02-06/audit_data_pipeline.md:483`]

---

## Root Cause

Every query function in `query.rs` follows this pattern:

```sql
FROM claude_sessions s, json_each(s.files_read)
WHERE s.files_read IS NOT NULL AND s.files_read != '[]'
```

The `files_written` column exists in the schema (TEXT, JSON array) and is populated during sync, but no query function ever references it.

---

## Fix Strategy: UNION ALL CTE

### Reusable `all_files` CTE Pattern

Replace the direct `json_each(s.files_read)` join with a CTE that unions both arrays:

```sql
WITH all_files AS (
    SELECT s.session_id, s.started_at, s.ended_at, s.files_read, s.files_written,
           json_each.value as file_path, 'read' as access_type
    FROM claude_sessions s, json_each(s.files_read)
    WHERE s.files_read IS NOT NULL AND s.files_read != '[]'
    UNION ALL
    SELECT s.session_id, s.started_at, s.ended_at, s.files_read, s.files_written,
           json_each.value as file_path, 'write' as access_type
    FROM claude_sessions s, json_each(s.files_written)
    WHERE s.files_written IS NOT NULL AND s.files_written != '[]'
)
SELECT file_path, COUNT(*) as total_access_count, ...
FROM all_files
...
```

The `access_type` column is available for future use (e.g. showing read vs write in results) but is not required by current consumers.

---

## Implementation Steps

### 1. `query_flex` (lines 54-149)

**Before (line 60-68):**
```sql
SELECT
    json_each.value as file_path,
    COUNT(*) as total_access_count,
    MAX(s.started_at) as last_access,
    COUNT(DISTINCT s.session_id) as session_count
FROM claude_sessions s, json_each(s.files_read)
LEFT JOIN chain_graph cg ON s.session_id = cg.session_id
WHERE s.files_read IS NOT NULL AND s.files_read != '[]'
```

**After:**
```sql
WITH all_files AS (
    SELECT s.session_id, s.started_at, json_each.value as file_path
    FROM claude_sessions s, json_each(s.files_read)
    WHERE s.files_read IS NOT NULL AND s.files_read != '[]'
    UNION ALL
    SELECT s.session_id, s.started_at, json_each.value as file_path
    FROM claude_sessions s, json_each(s.files_written)
    WHERE s.files_written IS NOT NULL AND s.files_written != '[]'
)
SELECT
    af.file_path,
    COUNT(*) as total_access_count,
    MAX(af.started_at) as last_access,
    COUNT(DISTINCT af.session_id) as session_count
FROM all_files af
LEFT JOIN chain_graph cg ON af.session_id = cg.session_id
WHERE 1=1
```

**Notes:**
- Dynamic filter clauses (time, chain, session, files) append to the CTE or the outer WHERE as appropriate.
- Time filter should be pushed into both CTE legs for performance: `AND s.started_at >= datetime('now', '-N days')`.
- Chain filter uses the outer JOIN: `AND cg.chain_id = ?`.
- File pattern filter applies to `af.file_path LIKE ?`.
- `GROUP BY af.file_path` collapses duplicates from UNION ALL.

### 2. `query_heat` (lines 853-989)

**Before (line 869-883):**
```sql
SELECT json_each.value as file_path,
       SUM(CASE WHEN s.started_at >= datetime('now', '-7 days') THEN 1 ELSE 0 END) as count_7d,
       COUNT(*) as count_long,
       MIN(s.started_at) as first_access,
       MAX(s.started_at) as last_access
FROM claude_sessions s, json_each(s.files_read)
WHERE s.started_at >= datetime('now', '-{days} days')
  AND s.files_read IS NOT NULL AND s.files_read != '[]'
  {file_filter}
GROUP BY json_each.value
```

**After:**
```sql
WITH all_files AS (
    SELECT s.session_id, s.started_at, json_each.value as file_path
    FROM claude_sessions s, json_each(s.files_read)
    WHERE s.started_at >= datetime('now', '-{days} days')
      AND s.files_read IS NOT NULL AND s.files_read != '[]'
    UNION ALL
    SELECT s.session_id, s.started_at, json_each.value as file_path
    FROM claude_sessions s, json_each(s.files_written)
    WHERE s.started_at >= datetime('now', '-{days} days')
      AND s.files_written IS NOT NULL AND s.files_written != '[]'
)
SELECT af.file_path,
       SUM(CASE WHEN af.started_at >= datetime('now', '-7 days') THEN 1 ELSE 0 END) as count_7d,
       COUNT(*) as count_long,
       MIN(af.started_at) as first_access,
       MAX(af.started_at) as last_access
FROM all_files af
WHERE 1=1
  {file_filter}
GROUP BY af.file_path
```

**Notes:**
- Time filter pushed into both CTE legs for performance.
- File filter applies to outer `af.file_path LIKE ?`.
- Heat scoring logic in Rust remains unchanged.

### 3. `query_sessions` (lines 416-576)

**3a. File count in main query (lines 425-441):**

**Before:**
```sql
CASE
    WHEN s.files_read IS NULL OR s.files_read = '[]' THEN 0
    ELSE (SELECT COUNT(*) FROM json_each(s.files_read))
END as file_count
```

**After:**
```sql
(
    CASE WHEN s.files_read IS NULL OR s.files_read = '[]' THEN 0
         ELSE (SELECT COUNT(*) FROM json_each(s.files_read)) END
    +
    CASE WHEN s.files_written IS NULL OR s.files_written = '[]' THEN 0
         ELSE (SELECT COUNT(*) FROM json_each(s.files_written)) END
) as file_count
```

**Note:** This counts read + written. A file appearing in both arrays is counted twice. For `total_accesses`, same pattern applies. True dedup would require a more complex subquery, but for file_count purposes, double-counting is acceptable since it reflects total file operations (a read AND a write are two operations).

**3b. Top files per session (lines 485-491):**

**Before:**
```sql
SELECT json_each.value as file_path, 1 as access_count, s.started_at as first_accessed_at
FROM claude_sessions s, json_each(s.files_read)
WHERE s.session_id = ?
LIMIT 5
```

**After:**
```sql
SELECT file_path, 1 as access_count, started_at as first_accessed_at
FROM (
    SELECT json_each.value as file_path, s.started_at
    FROM claude_sessions s, json_each(s.files_read)
    WHERE s.session_id = ? AND s.files_read IS NOT NULL AND s.files_read != '[]'
    UNION
    SELECT json_each.value as file_path, s.started_at
    FROM claude_sessions s, json_each(s.files_written)
    WHERE s.session_id = ? AND s.files_written IS NOT NULL AND s.files_written != '[]'
)
LIMIT 5
```

**Note:** Uses `UNION` (not `UNION ALL`) to dedup files that appear in both read and written for a single session.

**3c. Chain summary (lines 524-537):**

**Before:**
```sql
COUNT(DISTINCT json_each.value) as file_count
FROM chain_graph cg
JOIN claude_sessions s ON cg.session_id = s.session_id
LEFT JOIN json_each(s.files_read) ON s.files_read IS NOT NULL AND s.files_read != '[]'
```

**After:**
```sql
COUNT(DISTINCT file_path) as file_count
FROM chain_graph cg
JOIN claude_sessions s ON cg.session_id = s.session_id
LEFT JOIN (
    SELECT s2.session_id, json_each.value as file_path
    FROM claude_sessions s2, json_each(s2.files_read)
    WHERE s2.files_read IS NOT NULL AND s2.files_read != '[]'
    UNION
    SELECT s2.session_id, json_each.value as file_path
    FROM claude_sessions s2, json_each(s2.files_written)
    WHERE s2.files_written IS NOT NULL AND s2.files_written != '[]'
) af ON af.session_id = s.session_id
```

### 4. `query_search` (lines 591-639)

**Before (line 598-607):**
```sql
SELECT json_each.value as file_path, COUNT(*) as access_count
FROM claude_sessions s, json_each(s.files_read)
WHERE s.files_read IS NOT NULL AND s.files_read != '[]'
  AND LOWER(json_each.value) LIKE ?
GROUP BY json_each.value
ORDER BY access_count DESC
LIMIT ?
```

**After:**
```sql
WITH all_files AS (
    SELECT json_each.value as file_path
    FROM claude_sessions s, json_each(s.files_read)
    WHERE s.files_read IS NOT NULL AND s.files_read != '[]'
    UNION ALL
    SELECT json_each.value as file_path
    FROM claude_sessions s, json_each(s.files_written)
    WHERE s.files_written IS NOT NULL AND s.files_written != '[]'
)
SELECT file_path, COUNT(*) as access_count
FROM all_files
WHERE LOWER(file_path) LIKE ?
GROUP BY file_path
ORDER BY access_count DESC
LIMIT ?
```

### 5. `query_file` (lines 641-755)

Three match attempts (exact, suffix, substring) all need updating. Each uses the same pattern.

**Before (exact match, line 651-659):**
```sql
SELECT DISTINCT s.session_id, s.started_at as last_access, cg.chain_id
FROM claude_sessions s, json_each(s.files_read)
LEFT JOIN chain_graph cg ON s.session_id = cg.session_id
WHERE json_each.value = ?
ORDER BY s.started_at DESC
LIMIT ?
```

**After (exact match):**
```sql
WITH all_files AS (
    SELECT s.session_id, s.started_at, json_each.value as file_path, 'read' as access_type
    FROM claude_sessions s, json_each(s.files_read)
    WHERE s.files_read IS NOT NULL AND s.files_read != '[]'
    UNION ALL
    SELECT s.session_id, s.started_at, json_each.value as file_path, 'write' as access_type
    FROM claude_sessions s, json_each(s.files_written)
    WHERE s.files_written IS NOT NULL AND s.files_written != '[]'
)
SELECT DISTINCT af.session_id, af.started_at as last_access, cg.chain_id
FROM all_files af
LEFT JOIN chain_graph cg ON af.session_id = cg.session_id
WHERE af.file_path = ?
ORDER BY af.started_at DESC
LIMIT ?
```

**Note:** The `access_type` column could populate `FileSessionInfo.access_types` correctly (currently hardcoded to `["read"]` at line 673). This is a bonus improvement.

Apply the same CTE pattern to the suffix match (line 681-690) and substring match (line 714) queries. The CTE can be defined once per function call and reused across all three attempts by making it a constant SQL fragment.

### 6. `query_co_access` (lines 757-841)

**6a. Session-finding query (line 770-772):**

**Before:**
```sql
SELECT DISTINCT s.session_id
FROM claude_sessions s, json_each(s.files_read)
WHERE json_each.value LIKE ?
```

**After:**
```sql
SELECT DISTINCT session_id FROM (
    SELECT s.session_id, json_each.value as file_path
    FROM claude_sessions s, json_each(s.files_read)
    WHERE s.files_read IS NOT NULL AND s.files_read != '[]'
    UNION ALL
    SELECT s.session_id, json_each.value as file_path
    FROM claude_sessions s, json_each(s.files_written)
    WHERE s.files_written IS NOT NULL AND s.files_written != '[]'
) WHERE file_path LIKE ?
```

**6b. Co-access query (line 797-808):**

**Before:**
```sql
SELECT json_each.value as file_path, COUNT(DISTINCT s.session_id) as co_count
FROM claude_sessions s, json_each(s.files_read)
WHERE s.session_id IN (...)
  AND json_each.value NOT LIKE ?
GROUP BY json_each.value
ORDER BY co_count DESC
LIMIT ?
```

**After:**
```sql
WITH all_files AS (
    SELECT s.session_id, json_each.value as file_path
    FROM claude_sessions s, json_each(s.files_read)
    WHERE s.files_read IS NOT NULL AND s.files_read != '[]'
    UNION ALL
    SELECT s.session_id, json_each.value as file_path
    FROM claude_sessions s, json_each(s.files_written)
    WHERE s.files_written IS NOT NULL AND s.files_written != '[]'
)
SELECT file_path, COUNT(DISTINCT session_id) as co_count
FROM all_files
WHERE session_id IN (...)
  AND file_path NOT LIKE ?
GROUP BY file_path
ORDER BY co_count DESC
LIMIT ?
```

### 7. `query_timeline` (lines 206-411)

Three sub-queries need updating:

**7a. Daily buckets (lines 218-228):**

**Before:**
```sql
SELECT date(s.started_at) as date, COUNT(*) as access_count,
       COUNT(DISTINCT json_each.value) as files_touched,
       GROUP_CONCAT(DISTINCT s.session_id) as sessions
FROM claude_sessions s, json_each(s.files_read)
LEFT JOIN chain_graph cg ON s.session_id = cg.session_id
WHERE s.started_at >= datetime('now', '-{days} days')
  AND s.files_read IS NOT NULL AND s.files_read != '[]'
```

**After:**
```sql
WITH all_files AS (
    SELECT s.session_id, s.started_at, json_each.value as file_path
    FROM claude_sessions s, json_each(s.files_read)
    WHERE s.started_at >= datetime('now', '-{days} days')
      AND s.files_read IS NOT NULL AND s.files_read != '[]'
    UNION ALL
    SELECT s.session_id, s.started_at, json_each.value as file_path
    FROM claude_sessions s, json_each(s.files_written)
    WHERE s.started_at >= datetime('now', '-{days} days')
      AND s.files_written IS NOT NULL AND s.files_written != '[]'
)
SELECT date(af.started_at) as date, COUNT(*) as access_count,
       COUNT(DISTINCT af.file_path) as files_touched,
       GROUP_CONCAT(DISTINCT af.session_id) as sessions
FROM all_files af
LEFT JOIN chain_graph cg ON af.session_id = cg.session_id
WHERE 1=1
```

**7b. Per-file timeline (lines 272-301):** Same CTE pattern, using `af.file_path` instead of `json_each.value`.

**7c. Per-file bucket counts (lines 312-330):** Same CTE pattern.

---

## Deduplication Strategy

### When UNION ALL duplicates matter

A file appearing in BOTH `files_read` AND `files_written` within the same session produces two rows from `UNION ALL`. This affects different queries differently:

| Query | Dedup Needed? | How Handled |
|-------|---------------|-------------|
| `query_flex` | No | `GROUP BY file_path` collapses duplicates. `COUNT(*)` becomes total operations (read+write), which is the desired semantic. |
| `query_heat` | No | `GROUP BY file_path` collapses. Both read and write operations contribute to heat, which is correct. |
| `query_sessions` file_count | Acceptable | Counts total file operations. A file read AND written = 2 operations. |
| `query_sessions` top_files | Yes | Use `UNION` (not `UNION ALL`) to show unique files. |
| `query_search` | No | `GROUP BY file_path` collapses. |
| `query_file` | Yes | `SELECT DISTINCT session_id` already handles this. |
| `query_co_access` | No | `COUNT(DISTINCT session_id)` and `GROUP BY file_path` handle this. |
| `query_timeline` buckets | No | `COUNT(DISTINCT file_path)` for `files_touched`. `COUNT(*)` for `access_count` correctly counts all operations. |

### Key principle

For aggregation queries (`COUNT`, `GROUP BY`), UNION ALL duplicates are either collapsed by grouping or represent legitimate distinct operations (reading a file AND writing it are two operations).

For listing queries (top_files, file sessions), use `UNION` or `SELECT DISTINCT` to avoid showing the same file/session twice.

---

## TDD Test Plan

All tests should be added to `core/src/query.rs` (inline `#[cfg(test)]` module) or a dedicated integration test file.

### Test fixtures

Create a test database with sessions that have:
- Session A: `files_read: ["a.rs", "b.rs"]`, `files_written: ["c.rs"]` -- c.rs is write-only
- Session B: `files_read: ["a.rs"]`, `files_written: ["a.rs", "d.rs"]` -- a.rs is read+write, d.rs is write-only
- Session C: `files_read: null`, `files_written: ["e.rs"]` -- pure-write session

### Tests

1. **`test_flex_includes_files_written`**
   - Run `query_flex` with no filters.
   - Assert `c.rs`, `d.rs`, `e.rs` appear in results (write-only files).
   - Assert `a.rs` and `b.rs` also appear (read files still work).

2. **`test_flex_deduplicates_read_and_written`**
   - Run `query_flex` with no filters.
   - Assert `a.rs` appears exactly once in results (not duplicated).
   - Assert its `total_access_count` is 3 (read in A, read in B, written in B).
   - Assert its `session_count` is 2 (appears in both sessions A and B).

3. **`test_heat_includes_write_heavy_files`**
   - Insert sessions where `e.rs` appears in `files_written` across 10 sessions but never in `files_read`.
   - Run `query_heat`.
   - Assert `e.rs` appears in heat results with non-zero heat score.
   - Previously, `e.rs` would have been completely invisible.

4. **`test_sessions_file_count_includes_written`**
   - Run `query_sessions`.
   - For Session A: assert `file_count >= 3` (2 read + 1 written).
   - For Session C: assert `file_count >= 1` (1 written, 0 read).
   - Previously, Session C would show `file_count: 0`.

5. **`test_search_finds_written_only_files`**
   - Run `query_search` with pattern `"e.rs"`.
   - Assert results contain `e.rs` with `access_count >= 1`.
   - Previously, this search would return 0 results.

6. **`test_file_query_finds_written_only_files`**
   - Run `query_file` with `file_path: "c.rs"`.
   - Assert `found: true`.
   - Assert sessions list contains Session A.
   - Previously, this would return `found: false`.

7. **`test_co_access_considers_written_files`**
   - Run `query_co_access` with anchor `"a.rs"`.
   - Assert `c.rs` appears in co-access results (co-accessed via Session A: a.rs read, c.rs written).
   - Previously, `c.rs` would be missing from co-access.

8. **`test_timeline_includes_written_files`**
   - Run `query_timeline`.
   - Assert `files_touched` count includes write-only files.
   - Assert `e.rs` appears in the files list.
   - Previously, timeline would not reflect write-only file activity.

---

## Success Criteria

1. `tastematter query flex --time 7d` shows files that were only written (never read).
2. `tastematter heat` gives non-zero heat scores to write-heavy files.
3. `tastematter query sessions --time 7d` shows correct file_count including written files.
4. `tastematter query search <pattern>` finds files that were only written.
5. `tastematter query file <path>` finds sessions that wrote to a file.
6. `tastematter query co-access <path>` includes files written in same sessions.
7. `tastematter query timeline --time 30d` includes write-only files in daily buckets.
8. All 8 tests pass.
9. Latency remains under 100ms for all queries (the CTE adds minimal overhead since both `json_each` expansions are on the same table).

---

## Handoff Checklist

- [ ] Read `core/src/query.rs` fully before starting
- [ ] Spec 02 (Path Normalization) has been applied first so `files_written` paths are normalized
- [ ] Implement CTE pattern for `query_flex` first (simplest, most used)
- [ ] Write `test_flex_includes_files_written` and `test_flex_deduplicates_read_and_written` first
- [ ] Apply same CTE pattern to remaining 6 query functions
- [ ] Write remaining 6 tests
- [ ] Run full test suite: `cd core && cargo test`
- [ ] Manual smoke test: `./target/release/context-os query flex --time 7d` -- verify write-only files appear
- [ ] Manual smoke test: `./target/release/context-os heat` -- verify write-heavy files have heat
- [ ] Verify latency remains <100ms via `--format json` timing fields

---

**Created:** 2026-02-06
**Author:** Tastematter data pipeline audit
