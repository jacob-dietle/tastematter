---
title: "Drift Analysis & Test Fixes Planned"
package_number: 15
date: 2026-01-15
status: superseded
previous_package: "[[14_2026-01-15_GLOB_BUG_TDD_FIX_COMPLETE]]"
related:
  - "[[~/.claude/plans/synchronous-coalescing-harbor.md]]"
  - "[[cli/tests/index/test_chain_graph.py]]"
  - "[[specs/canonical/02_ROADMAP.md]]"
  - "[[specs/canonical/06_RUST_PORT_SPECIFICATION.md]]"
tags:
  - context-package
  - tastematter
  - drift-analysis
  - tdd
  - test-fixes
---

# Drift Analysis & Test Fixes Planned - Context Package 15

## Executive Summary

Conducted comprehensive drift analysis comparing original roadmap against actual progress. Found ~3 hours of unplanned investigation work was **fully justified** (recovered +223 sessions, +29% data). Identified 3 failing tests that need updating - tests are WRONG, not implementation. Plan updated and approved.

## What Was Done This Session

### 1. Drift Analysis (Using 3 Parallel Explore Agents)

**Original Roadmap (02_ROADMAP.md):**
- Phase 0: Performance Foundation → ✅ Complete (achieved <2ms)
- Phase 1-5: Stigmergic Display, Multi-Repo, Agent UI → **DEFERRED**

**What Actually Happened:**
- Jan 12: Chain linking bug discovered → deep investigation
- Jan 13: Architecture audit → decision to port Python → Rust (56-78 hrs)
- Jan 15: Glob bug discovered during topology investigation

**Rust Port Progress:**

| Phase | Name | Status | Evidence |
|-------|------|--------|----------|
| 0 | Glob Bug Fix (Python CLI) | ✅ COMPLETE | Package 14, 988 sessions |
| 1 | Storage Foundation | ✅ COMPLETE | 26 Rust tests passing |
| 2 | Tauri Integration | ✅ COMPLETE | Package 10, direct library calls |
| 3 | Git Sync | ⬜ Not started | - |
| 4 | JSONL Parser | ⬜ Not started | - |
| 5 | Chain Graph | ⬜ Not started | - |
| 6 | Daemon | ⬜ Not started | - |

### 2. Drift Assessment

| Work Type | Hours | Justified? |
|-----------|-------|------------|
| Planned (Rust port phases 0-2) | ~8-10 | ✅ On track |
| Unplanned investigation | ~3 | ✅ YES - recovered 223 sessions |
| Total drift | ~3 hrs | Worth it: +29% data completeness |

**Key Insight:** The glob bug (`*.jsonl` → `**/*.jsonl`) was discovered BECAUSE the chain topology fix (Package 11) only yielded 17% improvement. Investigating further found the REAL bug. This is **good debugging discipline**.

### 3. TDD Health Check

**Test Suite Status:** DRIFTING - 3 tests out of sync with implementation

| Test File | Line | Issue |
|-----------|------|-------|
| `test_chain_graph.py` | 51 | Expects 2 leafUuids, gets 1 (LAST) |
| `test_chain_graph.py` | 98 | Expects 2 leafUuids, gets 1 (LAST) |
| `test_chain_graph.py` | 412 | Expects 3 leafUuids, gets 1 (LAST) |

**Root Cause:** Implementation changed to use LAST leafUuid (correct per Package 11 discovery), but tests were never updated. This is a **TDD discipline violation** - tests must be updated when spec changes.

**Verdict:** Tests are WRONG, not implementation. The LAST leafUuid behavior is correct because Claude Code stacks summaries oldest-first, so LAST = immediate parent, FIRST = root ancestor.

### 4. Plan File Updated

**Location:** `~/.claude/plans/synchronous-coalescing-harbor.md`

Updated with:
- Accurate status (Phase 0-2 complete)
- Drift analysis section
- Exact test fix details with code changes
- Next steps and verification commands

## Current State

**Test Results:**
- 461 tests collected
- 457 passing
- 4 failing (3 leafUuid tests + 1 week number test)
- [VERIFIED: pytest output from earlier session]

**Database:**
- 1057 sessions (grew from 988)
- 182 chains
- Largest chain: 335 sessions
- [VERIFIED: tastematter status output]

## Files Modified This Session

| File | Change |
|------|--------|
| `~/.claude/plans/synchronous-coalescing-harbor.md` | Updated status, added drift analysis, exact test fixes |

## For Next Agent

### Context Chain

| # | Date | Title | Key Content |
|---|------|-------|-------------|
| 14 | 2026-01-15 | GLOB_BUG_TDD_FIX_COMPLETE | TDD fix applied, database rebuilt |
| 15 | 2026-01-15 | **DRIFT_ANALYSIS_TEST_FIXES_PLANNED** | Roadmap analysis, 3 test fixes identified |

### Start Here

1. Read this package (you're doing it now)
2. Read plan file: `~/.claude/plans/synchronous-coalescing-harbor.md`
3. Fix the 3 failing tests in `cli/tests/index/test_chain_graph.py`

### Jobs To Be Done (Next Session)

1. [ ] **Fix 3 failing tests in test_chain_graph.py** (~15 min)
   - Line 51: `assert len(leaf_uuids) == 1` and `assert leaf_uuids[0] == "uuid-parent-2"`
   - Line 98: `assert len(leaf_uuids) == 1` and `assert leaf_uuids[0] == "uuid-2"`
   - Line 412: `assert len(leaf_uuids) == 1` and `assert leaf_uuids[0] == "uuid-3"`
   - Update docstrings to explain LAST behavior

2. [ ] **Verify full test suite passes** - Target: 461 passing, 0 failing

3. [ ] **Decide next phase** - Options:
   - Phase 3: Git Sync (8-12 hours)
   - Phase 4: JSONL Parser (12-16 hours)
   - Defer Rust port, focus on features

### Test Fix Details

The tests expect ALL leafUuids but implementation correctly returns only LAST:

```python
# Why LAST is correct:
# Claude Code stacks summaries oldest-first:
# - Session C continues from B, gets [summary from A, summary from B]
# - FIRST summary always points to root ancestor
# - LAST summary points to immediate parent
# We want immediate parent for proper chain linking
```

### Do NOT

- Revert the LAST leafUuid implementation (it's correct)
- Skip updating test docstrings (document the new behavior)
- Assume the 4th failing test (week number) is related

---

**Document Status:** SUPERSEDED by [[16_2026-01-15_TEST_ALIGNMENT_COMPLETE]]
**Session Duration:** ~45 minutes
**Primary Work:** Drift analysis, TDD health check, plan update
**Methodology Applied:** Specification-driven development, TDD review
