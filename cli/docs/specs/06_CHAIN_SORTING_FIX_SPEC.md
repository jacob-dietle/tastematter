# Chain Display Sorting Fix Specification

**Date:** 2026-01-04
**Status:** Ready for Implementation
**Type:** Bug Fix (TDD)

---

## Problem Statement

`get_all_chains()` sorts by `time_range` which is always `None`, causing multi-session chains (the interesting ones) to be buried after 600+ single-session chains.

### Evidence

```
Direct Python call to build_chain_graph():
  - 625 chains total
  - 12 chains with >1 session
  - Largest: 104 sessions (chain 9fd2c418)

CLI "query chains --limit 10":
  - Shows 625 total (correct)
  - Only 1 multi-session chain in results (wrong - they're buried)
```

### Root Cause Chain

1. `build_chain_graph()` creates Chain objects with `time_range=None` (chain_graph.py:244)
2. `get_all_chains()` sorts by `time_range[1]` for recency (context_index.py:155)
3. All chains return `datetime.min` since `time_range` is None
4. Sort is effectively random (dict insertion order)
5. `query chains --limit 10` shows first 10, which happen to be single-session chains
6. Multi-session chains are buried after 600+ single-session chains

---

## Solution

Sort by `session_count` descending (primary), then recency (secondary).

### Files to Modify

| File | Lines | Change |
|------|-------|--------|
| `src/context_os_events/index/context_index.py` | 139-159 | Fix `get_all_chains()` sort order |
| `tests/index/test_context_index.py` | (new) | Add `TestGetAllChainsSorting` class |

---

## TDD Specification

### Test Cases (RED Phase)

**File:** `tests/index/test_context_index.py`

#### Test 1: `test_sorts_by_session_count_descending`
- Create 3 chains: 1 session, 3 sessions, 5 sessions
- Add in random order
- Assert: 5-session chain first, 3-session second, 1-session last

#### Test 2: `test_secondary_sort_by_recency_when_same_session_count`
- Create 2 chains with same session count (2 each)
- One has old time_range, one has new time_range
- Assert: newer chain appears first

#### Test 3: `test_handles_none_time_range_gracefully`
- Create 2 chains with `time_range=None` (the bug scenario)
- Different session counts
- Assert: larger chain appears first despite None time_range

### Implementation (GREEN Phase)

**Current (buggy):**
```python
def get_all_chains(self) -> List[Chain]:
    """Get all chains sorted by recency (newest first)."""
    chains = list(self._chains.values())

    def get_latest_timestamp(chain: Chain) -> datetime:
        if hasattr(chain, 'nodes') and chain.nodes:
            timestamps = [n.timestamp for n in chain.nodes if n.timestamp]
            return max(timestamps) if timestamps else datetime.min
        if hasattr(chain, 'time_range') and chain.time_range:
            return chain.time_range[1]
        return datetime.min

    chains.sort(key=get_latest_timestamp, reverse=True)
    return chains
```

**Fixed:**
```python
def get_all_chains(self) -> List[Chain]:
    """Get all chains sorted by session count (largest first), then recency.

    Returns:
        List of Chain objects, largest chains first
    """
    chains = list(self._chains.values())

    def sort_key(chain: Chain) -> tuple:
        # Primary: session count (descending via negative)
        if hasattr(chain, 'sessions'):
            session_count = len(chain.sessions)
        elif hasattr(chain, 'nodes'):
            session_count = len(chain.nodes)
        else:
            session_count = 0

        # Secondary: recency (descending)
        if hasattr(chain, 'nodes') and chain.nodes:
            timestamps = [n.timestamp for n in chain.nodes if n.timestamp]
            recency = max(timestamps) if timestamps else datetime.min
        elif hasattr(chain, 'time_range') and chain.time_range:
            recency = chain.time_range[1]
        else:
            recency = datetime.min

        return (-session_count, recency)

    chains.sort(key=sort_key, reverse=True)
    return chains
```

---

## Verification

### Test Command
```bash
cd apps/context_os_events
.venv/Scripts/python -m pytest tests/index/test_context_index.py -v -k "TestGetAllChainsSorting"
```

### Manual Verification
After fix, `context-os query chains --limit 10` should show:
```
Chain ID    | Sessions | Files | Time Range
9fd2c418    | 104      | ...   | ...
7f389600    | 81       | ...   | ...
...
```

---

## Commit Message

```
fix(chains): Sort get_all_chains by session_count descending

Bug: Multi-session chains were buried after 600+ single-session chains
because time_range was always None, making sort order effectively random.

Fix: Sort by session_count (primary), recency (secondary).

Evidence: Direct Python call found chain with 104 sessions,
but CLI 'query chains --limit 10' only showed 1-session chains.
```

---

## Background: Chain Linking IS Working

This is a display bug, not a data bug. The chain linking mechanism works correctly:

- **751 JSONL files** in `~/.claude/projects/`
- **144 files** contain `leafUuid` references (session continuations)
- **132 sessions** have parent links in database
- **Largest chain:** 104 sessions linked together

The data is correct; we just weren't surfacing the interesting chains first.
