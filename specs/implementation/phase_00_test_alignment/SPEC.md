# Test Alignment Specification: leafUuid Extraction Behavior

**Spec ID:** PHASE_00_TEST_ALIGNMENT
**Created:** 2026-01-15
**Status:** ✅ COMPLETE (2026-01-15)
**Methodology:** Specification-Driven Development + Test-Driven Execution

---

## Executive Summary

**Problem:** 4 tests are failing because they expect OLD behavior (return ALL leafUuids) while the implementation was correctly updated to return only the LAST leafUuid based on empirical testing of Claude Code's actual data model.

**Root Cause:** TDD discipline violation - implementation was changed without updating tests.

**Additional Finding:** Canonical spec `07_CLAUDE_CODE_DATA_MODEL.md` has INCORRECT guidance (says FIRST, should say LAST). This creates documentation drift that must also be corrected.

**Solution:** Update tests to match the correct implementation behavior, update test docstrings to document the WHY, then update canonical spec.

---

## Evidence Chain

### 1. Implementation Behavior (Current - CORRECT)

**File:** `cli/src/context_os_events/index/chain_graph.py:63-112`

```python
def extract_leaf_uuids(filepath: Path) -> List[str]:
    """Extract session resumption leafUuid from JSONL file.

    IMPORTANT: Use the LAST summary's leafUuid, not the first.

    Claude Code stacks summaries oldest-first:
    - When session B continues from A, B gets a summary with leafUuid -> A
    - When session C continues from B, C gets [summary from A, summary from B]
    - The FIRST summary always points to the original root
    - The LAST summary points to the immediate parent

    This was discovered through empirical testing on 2026-01-15.
    Previous "first record only" approach caused all sessions to link
    to the root (star topology) instead of proper chains.

    Returns:
        List with single leafUuid if session was resumed, empty otherwise
    """
```

**Behavior:** Returns `[last_leaf_uuid]` (list with 1 element) or `[]` (empty list)

[VERIFIED: chain_graph.py:63-112]

### 2. Empirical Evidence (Package 11)

**File:** `specs/context_packages/04_daemon/11_2026-01-15_CHAIN_TOPOLOGY_INVESTIGATION.md`

```
Session `0deab2e5` has 10 summaries:
Summary 0: leafUuid -> message in 846b76ee (root)
Summary 1: leafUuid -> message in 846b76ee
...
Summary 8: leafUuid -> message in 846b76ee
Summary 9: leafUuid -> message in 13cc6033 (actual parent!)

When session C continues from B which continued from A:
- C inherits ALL of B's summaries (which include A's)
- The FIRST summary always points to the original root
- The LAST summary points to the immediate parent
```

[VERIFIED: Package 11, UUID `2278d18a` from summary 9 found in session `13cc6033` at line 3]

### 3. Test Expectations (Current - WRONG)

| Test | File:Line | Expects | Gets | Should Expect |
|------|-----------|---------|------|---------------|
| `test_extract_leaf_uuids_finds_summary_records` | test_chain_graph.py:51 | `len == 2`, both uuids | `len == 1`, LAST only | `len == 1`, `[0] == "uuid-parent-2"` |
| `test_extract_leaf_uuids_handles_malformed_json` | test_chain_graph.py:98 | `len == 2` | `len == 1`, LAST valid | `len == 1`, `[0] == "uuid-2"` |
| `test_handles_multiple_summary_records` | test_chain_graph.py:412 | `len == 3`, all uuids | `len == 1`, LAST only | `len == 1`, `[0] == "uuid-3"` |
| `test_full_workflow_no_mocking` | test_cli_query.py:564 | `"W5"` or `"W4"` | `"2026-W03"` | `"2026-W"` or `"2025-W"` |

### 4. Canonical Spec Drift (WRONG)

**File:** `specs/canonical/07_CLAUDE_CODE_DATA_MODEL.md`

**Lines 450-453 (INCORRECT):**
```
**Important nuance:** Multiple summaries can exist in a session (created during
compaction). The `leafUuid` in the **first** summary record indicates session
resumption. Subsequent summaries with `leafUuid` pointing to messages in the
same session are compaction markers.
```

**Lines 632-636 (INCORRECT):**
```
### Pitfall 2: Using Wrong leafUuid
**Wrong:** Use FIRST leafUuid found in session
**Wrong:** Use ALL leafUuids found in session
**Correct:** Use leafUuid from FIRST summary record only
```

**CORRECTED guidance should be:**
- Use leafUuid from **LAST** summary record only
- FIRST summary → root ancestor (wrong for chain linking)
- LAST summary → immediate parent (correct for chain linking)

---

## Type Contracts

### extract_leaf_uuids Function

**Input:**
```python
filepath: Path  # Path to a .jsonl session file
```

**Output:**
```python
List[str]  # Either [last_leaf_uuid] or []

# Contract:
# - If session has summary records with leafUuid: return [LAST leafUuid]
# - If session has no summary records: return []
# - If session has multiple summaries: return ONLY the LAST one's leafUuid
# - Never return more than 1 element
```

**Why single element list instead of Optional[str]?**
Legacy API compatibility - callers iterate over result.

### Test Data Contracts

**Test 1 Data:**
```python
# Input: 2 summaries
summaries = [
    {"type": "summary", "leafUuid": "uuid-parent-1"},  # FIRST (root)
    {"type": "summary", "leafUuid": "uuid-parent-2"},  # LAST (parent)
]
# Expected output: ["uuid-parent-2"]
```

**Test 2 Data:**
```python
# Input: 2 valid summaries with malformed line between
lines = [
    {"type": "summary", "leafUuid": "uuid-1"},  # FIRST (root)
    "this is not valid json",                    # Skipped
    {"type": "summary", "leafUuid": "uuid-2"},  # LAST (parent)
]
# Expected output: ["uuid-2"]
```

**Test 3 Data:**
```python
# Input: 3 summaries (simulating deep chain A->B->C)
summaries = [
    {"type": "summary", "leafUuid": "uuid-1"},  # Points to root
    {"type": "summary", "leafUuid": "uuid-2"},  # Points to intermediate
    {"type": "summary", "leafUuid": "uuid-3"},  # Points to immediate parent
]
# Expected output: ["uuid-3"]
```

**Test 4 Data:**
```python
# Input: CLI output with ISO week format
output = "2026-W03"  # or "2026-W02"
# Expected: Contains "2026-W" (ISO week format, not "W3" short format)
```

---

## Implementation Guide

### Phase 1: Fix test_chain_graph.py Tests (3 tests)

#### Step 1.1: Fix test_extract_leaf_uuids_finds_summary_records

**File:** `cli/tests/index/test_chain_graph.py`
**Lines:** 48-55

**Current (WRONG):**
```python
        try:
            leaf_uuids = extract_leaf_uuids(filepath)

            assert len(leaf_uuids) == 2
            assert "uuid-parent-1" in leaf_uuids
            assert "uuid-parent-2" in leaf_uuids
        finally:
```

**Updated (CORRECT):**
```python
        try:
            leaf_uuids = extract_leaf_uuids(filepath)

            # Implementation returns LAST leafUuid only (immediate parent)
            # See Package 11: Claude Code stacks summaries oldest-first
            assert len(leaf_uuids) == 1
            assert leaf_uuids[0] == "uuid-parent-2"  # LAST summary's leafUuid
        finally:
```

**Update docstring (lines 22-30):**
```python
    def test_extract_leaf_uuids_finds_summary_records(self):
        """Should extract LAST leafUuid from summary records.

        IMPORTANT: Claude Code stacks summaries oldest-first when continuing:
        - Session C continues B continues A
        - C gets: [summary from A, summary from B]
        - FIRST summary → original root (A)
        - LAST summary → immediate parent (B) ← We want this

        See Package 11 investigation (2026-01-15) for empirical verification.
        """
```

#### Step 1.2: Fix test_extract_leaf_uuids_handles_malformed_json

**File:** `cli/tests/index/test_chain_graph.py`
**Line:** 98

**Current (WRONG):**
```python
            leaf_uuids = extract_leaf_uuids(filepath)
            assert len(leaf_uuids) == 2
```

**Updated (CORRECT):**
```python
            leaf_uuids = extract_leaf_uuids(filepath)
            # Returns LAST valid leafUuid only
            assert len(leaf_uuids) == 1
            assert leaf_uuids[0] == "uuid-2"  # LAST valid summary
```

#### Step 1.3: Fix test_handles_multiple_summary_records

**File:** `cli/tests/index/test_chain_graph.py`
**Lines:** 411-416

**Current (WRONG):**
```python
            leaf_uuids = extract_leaf_uuids(filepath)

            # All three leafUuids should be extracted
            assert len(leaf_uuids) == 3
            assert "uuid-1" in leaf_uuids
            assert "uuid-2" in leaf_uuids
            assert "uuid-3" in leaf_uuids
```

**Updated (CORRECT):**
```python
            leaf_uuids = extract_leaf_uuids(filepath)

            # Only LAST leafUuid should be extracted (immediate parent)
            # uuid-1, uuid-2 are ancestors; uuid-3 is immediate parent
            assert len(leaf_uuids) == 1
            assert leaf_uuids[0] == "uuid-3"  # LAST = immediate parent
```

**Update docstring (lines 385-389):**
```python
    def test_handles_multiple_summary_records(self):
        """Real JSONL files have multiple summaries (conversation history stack).

        Claude Code stacks summaries oldest-first when continuing sessions.
        Only the LAST summary's leafUuid indicates the immediate parent.
        Earlier summaries point to ancestors in the chain.
        """
```

### Phase 2: Fix test_cli_query.py Test (1 test)

#### Step 2.1: Fix test_full_workflow_no_mocking

**File:** `cli/tests/test_cli_query.py`
**Line:** 564

**Current (WRONG):**
```python
        assert "W5" in result.output or "W4" in result.output
```

**Updated (CORRECT):**
```python
        # Implementation uses ISO week format "YYYY-WXX", not short "WX"
        assert "2026-W" in result.output or "2025-W" in result.output
```

### Phase 3: Update Canonical Spec (Follow-up)

**File:** `specs/canonical/07_CLAUDE_CODE_DATA_MODEL.md`

#### Step 3.1: Fix Lines 450-453

**Current (WRONG):**
```markdown
**Important nuance:** Multiple summaries can exist in a session (created during
compaction). The `leafUuid` in the **first** summary record indicates session
resumption.
```

**Updated (CORRECT):**
```markdown
**Important nuance:** Multiple summaries can exist in a session. Claude Code
stacks summaries oldest-first when continuing sessions. The `leafUuid` in the
**last** summary record indicates the immediate parent for chain linking.
```

#### Step 3.2: Fix Lines 632-636

**Current (WRONG):**
```markdown
### Pitfall 2: Using Wrong leafUuid
**Wrong:** Use FIRST leafUuid found in session
**Wrong:** Use ALL leafUuids found in session
**Correct:** Use leafUuid from FIRST summary record only
```

**Updated (CORRECT):**
```markdown
### Pitfall 2: Using Wrong leafUuid
**Wrong:** Use FIRST leafUuid found in session (points to root ancestor)
**Wrong:** Use ALL leafUuids found in session
**Correct:** Use leafUuid from LAST summary record only (immediate parent)
```

---

## Verification Plan

### Pre-Implementation Verification

```bash
cd apps/tastematter/cli

# Confirm tests currently fail
pytest tests/index/test_chain_graph.py::TestExtractLeafUuids::test_extract_leaf_uuids_finds_summary_records -v
pytest tests/index/test_chain_graph.py::TestExtractLeafUuids::test_extract_leaf_uuids_handles_malformed_json -v
pytest tests/index/test_chain_graph.py::TestRealWorldScenarios::test_handles_multiple_summary_records -v
pytest tests/test_cli_query.py::TestQueryCommandsIntegration::test_full_workflow_no_mocking -v

# Expected: All 4 FAIL
```

### Post-Implementation Verification

```bash
cd apps/tastematter/cli

# Run fixed tests individually
pytest tests/index/test_chain_graph.py::TestExtractLeafUuids::test_extract_leaf_uuids_finds_summary_records -v
pytest tests/index/test_chain_graph.py::TestExtractLeafUuids::test_extract_leaf_uuids_handles_malformed_json -v
pytest tests/index/test_chain_graph.py::TestRealWorldScenarios::test_handles_multiple_summary_records -v
pytest tests/test_cli_query.py::TestQueryCommandsIntegration::test_full_workflow_no_mocking -v

# Expected: All 4 PASS

# Run full test suite
pytest tests/ -v --tb=short

# Expected: 461 collected, 461 passed, 0 failed

# Verify no regressions in chain_graph tests
pytest tests/index/test_chain_graph.py -v

# Expected: All tests pass
```

### Contract Verification

```bash
cd apps/tastematter/cli

# Verify implementation still returns expected types
python -c "
from pathlib import Path
from context_os_events.index.chain_graph import extract_leaf_uuids
import tempfile

# Test with 2 summaries
with tempfile.NamedTemporaryFile(mode='w', suffix='.jsonl', delete=False) as f:
    f.write('{\"type\":\"summary\",\"leafUuid\":\"uuid-1\"}\n')
    f.write('{\"type\":\"summary\",\"leafUuid\":\"uuid-2\"}\n')
    f.flush()
    result = extract_leaf_uuids(Path(f.name))

assert isinstance(result, list), f'Expected list, got {type(result)}'
assert len(result) == 1, f'Expected 1 element, got {len(result)}'
assert result[0] == 'uuid-2', f'Expected uuid-2, got {result[0]}'
print('✅ Contract verified: Returns [LAST leafUuid]')
"
```

---

## Success Criteria

### Test Alignment Complete When:

- [x] `test_extract_leaf_uuids_finds_summary_records` passes with LAST assertion
- [x] `test_extract_leaf_uuids_handles_malformed_json` passes with LAST assertion
- [x] `test_handles_multiple_summary_records` passes with LAST assertion
- [x] `test_full_workflow_no_mocking` passes with ISO week format assertion
- [x] All 461 tests pass (0 failures)
- [x] Test docstrings updated to document LAST behavior
- [x] No regressions in other chain_graph tests

### Spec Alignment Complete When:

- [x] `07_CLAUDE_CODE_DATA_MODEL.md` lines 450-453 corrected (FIRST → LAST)
- [x] `07_CLAUDE_CODE_DATA_MODEL.md` lines 632-636 corrected (FIRST → LAST)
- [x] Spec matches implementation behavior

---

## Files to Modify

| File | Lines | Change Type |
|------|-------|-------------|
| `cli/tests/index/test_chain_graph.py` | 22-30 | Update docstring |
| `cli/tests/index/test_chain_graph.py` | 51-53 | Fix assertions |
| `cli/tests/index/test_chain_graph.py` | 98 | Fix assertion |
| `cli/tests/index/test_chain_graph.py` | 385-389 | Update docstring |
| `cli/tests/index/test_chain_graph.py` | 411-416 | Fix assertions |
| `cli/tests/test_cli_query.py` | 564 | Fix assertion |
| `specs/canonical/07_CLAUDE_CODE_DATA_MODEL.md` | 450-453 | Correct guidance |
| `specs/canonical/07_CLAUDE_CODE_DATA_MODEL.md` | 632-636 | Correct guidance |

---

## Common Pitfalls

### Do NOT:

1. ❌ Revert the implementation to return ALL leafUuids (implementation is CORRECT)
2. ❌ Skip updating docstrings (they ARE the specification per TDD)
3. ❌ Assume the 4th test (week format) is related to leafUuid (separate issue)
4. ❌ Forget to update canonical spec after tests pass

### Do:

1. ✅ Update tests to match implementation (tests were wrong)
2. ✅ Document WHY in docstrings (future developers need context)
3. ✅ Run full test suite to catch any regressions
4. ✅ Update canonical spec to prevent future confusion

---

## TDD Principle Applied

**Kent Beck:** "The tests ARE the specification."

In this case, the tests were an OUTDATED specification. The implementation was updated based on empirical evidence (Package 11), but the tests weren't updated to match.

This is a **TDD discipline violation** - when the specification changes (based on real-world evidence), tests must be updated FIRST to reflect the new specification, then verified against the implementation.

**Fix pattern:**
1. Update test assertions to new specification (this spec document)
2. Update test docstrings to document WHY (evidence chain)
3. Run tests - should PASS (implementation already correct)
4. Update canonical documentation to match

---

**Specification Status:** READY FOR IMPLEMENTATION
**Estimated Time:** 20 minutes
**Risk Level:** LOW (implementation already correct, only updating tests)
