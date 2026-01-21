# Phase 0: Glob Bug Fix - Specification

**Date:** 2026-01-15
**Status:** In Progress
**Blocking:** All downstream phases (Tauri integration, database queries)

---

## Executive Summary

The Python CLI indexer uses `*.jsonl` glob patterns which miss 218 agent sessions stored in subdirectories. This is a one-line fix in three files, but requires TDD validation to ensure correctness.

---

## Problem Statement

### The Bug

Claude Code stores session files in a **hierarchical** structure, not flat:

```
~/.claude/projects/{encoded-path}/
├── session-abc123.jsonl              # Top-level session (FOUND)
├── session-abc123/                   # Session directory
│   ├── subagents/                    # Agent children
│   │   └── agent-def456.jsonl        # ❌ MISSED by *.jsonl
│   └── tool-results/
│       └── toolu_xxx.txt
└── agent-ghi789.jsonl                # Top-level agent (FOUND)
```

### Evidence

```
OLD glob (*.jsonl):   765 files found
NEW glob (**/*.jsonl): 983 files found
DIFF:                 +218 agent sessions MISSING
```

### Impact

| Component | Impact |
|-----------|--------|
| `claude_sessions` table | Missing 218 sessions |
| `chain_graph` table | References non-existent sessions |
| JOIN queries | Fail silently (no matching sessions) |
| Chain filtering | Incomplete results |

---

## Root Cause Analysis

### Why This Happened

1. **Initial assumption:** Claude Code stores sessions in flat directory
2. **Reality:** Claude Code uses hierarchical storage for agent sessions
3. **Discovery:** Package 12 investigation found `subagents/` directories

### Files With Bug

| File | Line | Function | Current | Fix |
|------|------|----------|---------|-----|
| `jsonl_parser.py` | 191 | `find_session_files()` | `*.jsonl` | `**/*.jsonl` |
| `cli.py` | 444 | `build_chains()` CLI | `*.jsonl` | `**/*.jsonl` |
| `inverted_index.py` | 233 | `build_inverted_index()` | `*.jsonl` | `**/*.jsonl` |
| `chain_graph.py` | 217 | `build_chain_graph()` | `**/*.jsonl` | ✅ Already fixed |

---

## Solution Design

### Approach: Minimal Change

Each fix is a single character change: `*.jsonl` → `**/*.jsonl`

**Why `**/*.jsonl` works:**
- `**` matches any number of directories (including zero)
- `*.jsonl` matches any `.jsonl` file
- Combined: finds all `.jsonl` files at any depth

### Type Contract

**Input:** Directory path containing Claude Code session files
**Output:** List of all `.jsonl` files, including those in subdirectories

```python
# Before (bug):
files = list(project_dir.glob("*.jsonl"))  # Only top-level

# After (fix):
files = list(project_dir.glob("**/*.jsonl"))  # All levels
```

---

## TDD Implementation Plan

### Test 1: jsonl_parser.py

**File:** `tests/capture/test_jsonl_parser.py`
**Class:** `TestSubdirectoryDiscovery`

```python
def test_find_session_files_includes_subdirectories(self):
    """Should find JSONL files in subagents/ subdirectories.

    RED: Run before fix - finds only top-level files
    GREEN: Fix glob pattern - finds all files
    """
    # Create hierarchical structure
    # Top-level: session-abc123.jsonl
    # Subdirectory: session-abc123/subagents/agent-def456.jsonl

    # Assert both files found
    assert len(files) == 2
```

### Test 2: cli.py

**File:** `tests/test_cli_build_chains.py`
**Function:** `test_build_chains_counts_subdirectory_files`

```python
def test_build_chains_counts_subdirectory_files():
    """build-chains should count JSONL files in subdirectories.

    RED: Reports fewer files than exist
    GREEN: Reports correct count
    """
    # Create 3 files: 2 top-level, 1 in subdirectory
    # Assert count == 3
```

### Test 3: inverted_index.py

**File:** `tests/index/test_inverted_index.py`
**Class:** `TestSubdirectoryIndexing`

```python
def test_build_index_includes_subdirectory_sessions(self):
    """Should index file accesses from agent sessions in subdirectories.

    RED: Index missing subdirectory session accesses
    GREEN: Index includes all session accesses
    """
    # Create sessions with different file accesses
    # Assert both access patterns indexed
```

---

## Implementation Order

| Step | Action | Verification |
|------|--------|--------------|
| 1 | Write test for jsonl_parser.py | - |
| 2 | Run test | Should FAIL (RED) |
| 3 | Fix jsonl_parser.py:191 | - |
| 4 | Run test | Should PASS (GREEN) |
| 5 | Write test for cli.py | - |
| 6 | Run test | Should FAIL (RED) |
| 7 | Fix cli.py:444 | - |
| 8 | Run test | Should PASS (GREEN) |
| 9 | Write test for inverted_index.py | - |
| 10 | Run test | Should FAIL (RED) |
| 11 | Fix inverted_index.py:233 | - |
| 12 | Run test | Should PASS (GREEN) |
| 13 | Run all tests | All PASS |
| 14 | Rebuild database | `tastematter daemon rebuild` |
| 15 | Verify chain counts | ~356 sessions expected |

---

## Verification Plan

### Unit Tests

```bash
cd apps/tastematter/cli

# Run new TDD tests
pytest tests/capture/test_jsonl_parser.py::TestSubdirectoryDiscovery -v
pytest tests/test_cli_build_chains.py::test_build_chains_counts_subdirectory_files -v
pytest tests/index/test_inverted_index.py::TestSubdirectoryIndexing -v

# Run all tests (ensure no regressions)
pytest tests/ -v
```

### Integration Test

```bash
# Rebuild database with fixed indexer
tastematter daemon rebuild

# Verify session count increased
tastematter query flex --time 30d --agg count,sessions

# Verify chain linking improved
tastematter query chains --limit 10
# Expected: Largest chain ~356 sessions (was ~90)
```

### Manual Verification

Compare against Claude Code UI:
1. Open Claude Code
2. Check session count in UI
3. Should match `tastematter query` output

---

## Success Criteria

- [ ] All 3 new TDD tests pass
- [ ] All existing tests still pass (no regressions)
- [ ] Database rebuilt successfully
- [ ] Session count: 983 (was 765)
- [ ] Largest chain: ~356 sessions (was ~90)
- [ ] Chain filtering works in UI

---

## Risk Assessment

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Performance regression (more files to scan) | Low | `**/*.jsonl` is standard; Python handles efficiently |
| Breaking existing tests | Low | Tests use temp directories; unaffected |
| Other glob patterns in codebase | Medium | Grep search confirmed only 4 locations |

---

## References

- **Package 12:** `specs/context_packages/04_daemon/12_2026-01-15_GLOB_BUG_DISCOVERY.md`
- **Package 13:** `specs/context_packages/04_daemon/13_2026-01-15_CANONICAL_DATA_MODEL_COMPLETE.md`
- **Canonical Spec:** `specs/canonical/07_CLAUDE_CODE_DATA_MODEL.md`

---

## Change Log

| Date | Change |
|------|--------|
| 2026-01-15 | Initial spec created |
