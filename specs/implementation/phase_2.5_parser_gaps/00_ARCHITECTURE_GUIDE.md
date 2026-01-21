# Phase 2.5: Parser Gap Fix - Architecture Guide

**Status:** SPECIFICATION COMPLETE
**Effort:** ~75 lines of changes across 2 files
**Complexity:** SMALL MODIFICATION (not new architecture)

---

## Executive Summary

Fix 2 validated parser gaps that cause missing file paths in context queries.

| Gap | Data Source | Impact | Lines to Add |
|-----|-------------|--------|--------------|
| 1 | `user.toolUseResult.filePath` | Missing file creation confirmations | ~35 |
| 2 | `file-history-snapshot` records | Missing tracked file paths | ~40 |

**Success Metric:** No missing files in queries (100% capture rate for supported data sources)

---

## Problem Statement

### Current State (BROKEN)

```
JSONL Record Types:
├── assistant → tool_use.input.file_path  ✅ CAPTURED
├── user      → toolUseResult.filePath    ❌ IGNORED (Gap 1)
├── file-history-snapshot → keys          ❌ REJECTED (Gap 2)
└── tool_result, system, summary          → (no file paths, correctly ignored)
```

**Evidence (Package 17 Ground Truth):**
- Gap 1: 134 files contain `toolUseResult` with `filePath` → 0% captured
- Gap 2: ~1,689 `file-history-snapshot` records → 0% captured

### After Fix

```
JSONL Record Types:
├── assistant → tool_use.input.file_path  ✅ CAPTURED (unchanged)
├── user      → toolUseResult.filePath    ✅ CAPTURED (Gap 1 fixed)
├── file-history-snapshot → keys          ✅ CAPTURED (Gap 2 fixed)
└── tool_result, system, summary          → (correctly ignored)
```

---

## Data Structures

### Gap 1: toolUseResult (Canonical Spec lines 256-267)

```json
{
  "type": "user",
  "toolUseResult": {
    "type": "create | text | update",
    "filePath": "C:\\path\\to\\file.md",
    "content": "...",
    "file": {
      "filePath": "C:\\path\\to\\file.md",
      "content": "..."
    }
  }
}
```

**Classification Rules:**
| toolUseResult.type | access_type | Reasoning |
|--------------------|-------------|-----------|
| `"create"` | `create` | User confirmed file was created |
| `"update"` | `write` | User confirmed file was modified |
| `"text"` | `read` | User confirmed file content read |

### Gap 2: file-history-snapshot (Canonical Spec lines 353-372)

```json
{
  "type": "file-history-snapshot",
  "snapshot": {
    "trackedFileBackups": {
      "/path/to/file1.py": { "version": 3, ... },
      "/path/to/file2.md": { "version": 1, ... }
    }
  }
}
```

**Classification:**
- All keys in `trackedFileBackups` → `access_type: "read"`
- Reasoning: Claude is tracking/monitoring these files

---

## Files to Modify

### 1. `cli/src/context_os_events/capture/jsonl_parser.py`

| Location | Current | Change |
|----------|---------|--------|
| Line 311 | Rejects `file-history-snapshot` | Accept and extract paths |
| Line 330 | Only extracts from assistant | Also extract `toolUseResult` from user |

### 2. `cli/src/context_os_events/index/inverted_index.py`

| Location | Current | Change |
|----------|---------|--------|
| Line 141 | `if record.get("type") != "assistant": continue` | Also process user and file-history-snapshot |

### NOT Modified

- **Database schema** - Existing `file_accesses` table handles all cases
- **Type definitions** - Existing `FileAccess` dataclass sufficient
- **Query engine** - No changes needed (reads from same tables)

---

## Data Flow (Unchanged Architecture)

```
┌─────────────────────────────────────────────────────────────────┐
│                         JSONL Files                              │
│  ~/.claude/projects/{encoded-path}/**/*.jsonl                    │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    extract_file_accesses()                       │
│  inverted_index.py:110                                           │
│                                                                  │
│  BEFORE:                                                         │
│  └── assistant.message.content[].tool_use → FileAccess          │
│                                                                  │
│  AFTER (add 2 paths):                                            │
│  ├── assistant.message.content[].tool_use → FileAccess          │
│  ├── user.toolUseResult.filePath          → FileAccess  [NEW]   │
│  └── file-history-snapshot.snapshot.keys  → FileAccess  [NEW]   │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    persist_inverted_index()                      │
│  inverted_index.py:300                                           │
│  (unchanged - just writes FileAccess records to DB)              │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    file_conversation_index                       │
│  SQLite table (schema unchanged)                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Success Criteria

### Phase Complete When:

- [ ] All TDD tests pass (6 new tests)
- [ ] All existing 461 tests still pass
- [ ] `toolUseResult.filePath` contributes to `file_accesses` table
- [ ] `file-history-snapshot` keys contribute to `file_accesses` table
- [ ] Database rebuild shows increased file counts
- [ ] Context package documents the fix

### Verification Commands

```bash
# 1. Run new TDD tests
cd apps/tastematter/cli
pytest tests/index/test_inverted_index.py -v -k "toolUseResult or file_history"

# 2. Run full test suite (regression)
pytest tests/ -v --tb=short

# 3. Rebuild database
tastematter parse-sessions && tastematter build-index

# 4. Query and compare
# Before: X files with chain filter
# After: Should be higher (new sources captured)
./core/target/release/context-os query flex --time 7d --agg count

# 5. Verify specific record types captured
sqlite3 ~/.context-os/context_os_events.db \
  "SELECT tool_name, COUNT(*) FROM file_conversation_index GROUP BY tool_name"
```

---

## Related Documents

- [[specs/canonical/07_CLAUDE_CODE_DATA_MODEL.md]] - Canonical JSONL spec
- [[specs/context_packages/04_daemon/17_2026-01-16_PARSER_GAP_ANALYSIS_GROUND_TRUTH.md]] - Gap analysis
- [[cli/src/context_os_events/capture/jsonl_parser.py]] - Parser implementation
- [[cli/src/context_os_events/index/inverted_index.py]] - Index builder
