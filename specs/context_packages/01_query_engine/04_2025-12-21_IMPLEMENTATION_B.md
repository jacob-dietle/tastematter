---
title: "IMPLEMENTATION B"
package_number: 4
date: 2025-12-21
migrated_from: "apps/context-os/specs/context_os_intelligence/context_packages/04_2025-12-21_IMPLEMENTATION_B.md"
tags:
  - context-package
  - query-engine
  - legacy
---

# Implementation Context Package 02: CLI Hypercube + Verification Layer

**Status:** PHASE A COMPLETE - Ready for Phase B
**Created:** 2025-12-21
**Agent Handoff:** Session ended after Phase A implementation

---

## Executive Summary

**Phase A is COMPLETE.** All 6 existing query commands now support `--format json`.

**Next agent should start Phase B:** Create QuerySpec, QueryEngine, and `query flex` command.

---

## Read These Files First (In Order)

### Step 1: Understand the Specs (15 min)

1. `specs/context_os_intelligence/12_CLI_HYPERCUBE_SPEC.md`
   - Type contracts: QuerySpec, QueryResult, row types
   - Test specifications for Phase B
   - QueryEngine implementation details

2. `specs/context_os_intelligence/13_VERIFICATION_LAYER_SPEC.md`
   - Type contracts: QueryReceipt, VerificationResult, QueryLedger
   - Test specifications for Phase C

### Step 2: Understand What Was Implemented (10 min)

3. `src/context_os_events/cli.py`
   - Lines 107-139: NEW `format_option` and `results_to_json()` helper
   - Lines 1520-1577: `query_search` with `--format json`
   - Lines 1103-1225: `query_file` with `--format json`
   - Lines 1228-1281: `query_co_access` with `--format json`
   - Lines 1284-1357: `query_recent` with `--format json`
   - Lines 1360-1425: `query_chains` with `--format json`
   - Lines 1428-1526: `query_session` with `--format json`

4. `tests/test_cli_query.py`
   - Lines 873-1033: NEW `TestPhaseAJsonOutput` class with 8 passing tests

---

## Work Completed (Phase A)

### TDD Cycle Executed

| Step | Status | Evidence |
|------|--------|----------|
| Write tests first | DONE | 8 tests in `TestPhaseAJsonOutput` |
| Run tests (RED) | DONE | 6 failed with "No such option: --format" |
| Implement code | DONE | Added `--format json` to 6 commands |
| Run tests (GREEN) | DONE | All 8 tests pass (67.28s) |

### Code Added

**Helper functions (cli.py lines 107-139):**
```python
format_option = click.option(
    "--format", "output_format",
    type=click.Choice(["json", "table"]),
    default="table",
    help="Output format: json or table (default: table)"
)

def results_to_json(results: list, command: str, **extra_fields) -> str:
    """Serialize query results to JSON for agent consumption."""
    output = {
        "command": command,
        "timestamp": datetime.now().isoformat(),
        "result_count": len(results),
        "results": results,
        **extra_fields
    }
    return json.dumps(output, indent=2, default=str)
```

**Commands modified:**
- `query search` - Added `@format_option`, JSON output path
- `query file` - Added `@format_option`, JSON output path
- `query session` - Added `@format_option`, JSON output path
- `query chains` - Added `@format_option`, JSON output path
- `query co-access` - Added `@format_option`, JSON output path
- `query recent` - Added `@format_option`, JSON output path

### Tests Added

```python
class TestPhaseAJsonOutput:
    """Phase A tests: All existing query commands support --format json."""

    def test_query_search_supports_json_format(self, project_runner): ...
    def test_query_file_supports_json_format(self, project_runner): ...
    def test_query_session_supports_json_format(self, project_runner): ...
    def test_query_chains_supports_json_format(self, project_runner): ...
    def test_query_co_access_supports_json_format(self, project_runner): ...
    def test_query_recent_supports_json_format(self, project_runner): ...
    def test_default_format_is_table(self, project_runner): ...
    def test_json_output_contains_timestamp(self, project_runner): ...
```

---

## Next Steps (Phases B, C, D)

### Phase B: QuerySpec + QueryEngine (~200 lines, 2-3 hours)

**TDD Workflow:**
1. Copy test cases from Spec 12:
   - `test_query_spec.py` (~80 lines)
   - `test_query_engine_slicing.py` (~120 lines)
   - `test_query_engine_aggregation.py` (~100 lines)
   - `test_query_engine_rendering.py` (~60 lines)
2. Run tests (RED - should fail)
3. Create `src/context_os_events/query_engine.py` with:
   - `QuerySpec` dataclass (~40 lines)
   - `QueryResult` dataclass (~30 lines)
   - `QueryEngine` class (~130 lines)
4. Add `query flex` CLI command (~50 lines)
5. Run tests (GREEN - should pass)

**Key implementation details from Spec 12:**
```python
@dataclass
class QuerySpec:
    files: Optional[str] = None          # Glob pattern
    time: Optional[str] = None           # "7d", "2w", "2025-W50"
    chain: Optional[str] = None          # chain_id or "active"
    session: Optional[str] = None        # session_id prefix
    access: Optional[str] = None         # "r", "w", "c", "rw"
    agg: List[str] = field(default_factory=lambda: ["count"])
    format: Literal["json", "table"] = "json"
    limit: int = 20
    sort: Literal["count", "recency", "alpha"] = "count"
```

### Phase C: Verification Layer (~300 lines, 2-3 hours)

**Files to modify:**
- `query_engine.py` - Add QueryReceipt, QueryLedger, verify()
- `cli.py` - Add `query verify`, `query receipts` commands

**Ledger storage:** `~/.context-os/query_ledger/` with 30-day TTL

### Phase D: Update Skill (~50 lines, 30 min)

**File:** `.claude/skills/context-query/SKILL.md`
- Add hypercube model explanation
- Add `query flex` examples
- Add citation format requirements `[receipt_id]`
- Add verification workflow

---

## Test Commands

```bash
# Run Phase A tests (should all pass)
cd apps/context_os_events
.venv/Scripts/python -m pytest tests/test_cli_query.py::TestPhaseAJsonOutput -v

# Run all tests
.venv/Scripts/python -m pytest tests/ -v

# Test JSON output manually
"C:/Users/dietl/.context-os/bin/context-os.cmd" query search pixee --format json
"C:/Users/dietl/.context-os/bin/context-os.cmd" query recent --format json
```

---

## Common Pitfalls

1. **Path handling:** Windows paths need forward slashes in JSON
2. **Time parsing:** Use `datetime.fromisoformat()` with `.replace("Z", "+00:00")`
3. **JSON serialization:** Use `default=str` for datetime objects
4. **Test fixture:** Use `project_runner` fixture that changes to correct directory

---

## File Locations Summary

| File | Purpose | Status |
|------|---------|--------|
| `specs/.../12_CLI_HYPERCUBE_SPEC.md` | Hypercube spec + tests | READ for Phase B |
| `specs/.../13_VERIFICATION_LAYER_SPEC.md` | Verification spec + tests | READ for Phase C |
| `src/context_os_events/cli.py` | CLI commands | MODIFIED (Phase A) |
| `src/context_os_events/query_engine.py` | QuerySpec, QueryEngine | CREATE (Phase B) |
| `tests/test_cli_query.py` | CLI tests | MODIFIED (Phase A tests added) |
| `.claude/skills/context-query/SKILL.md` | Skill docs | UPDATE (Phase D) |

---

## Success Criteria Checklist

### Phase A (COMPLETE)
- [x] All 6 query commands support `--format json`
- [x] JSON output is valid and parseable
- [x] Default format is `table` (backwards compatible)
- [x] All Phase A tests pass (8/8)

### Phase B (TODO)
- [ ] QuerySpec dataclass implemented with validation
- [ ] QueryEngine executes slice -> aggregate -> render pipeline
- [ ] `query flex` command works with all options
- [ ] Multi-filter queries work (AND logic)
- [ ] All Phase B tests pass

### Phase C (TODO)
- [ ] Receipt IDs are deterministic
- [ ] Ledger saves/loads receipts correctly
- [ ] 30-day TTL enforced
- [ ] verify() detects MATCH vs DRIFT
- [ ] `query verify` command works
- [ ] All Phase C tests pass

### Phase D (TODO)
- [ ] Skill documents hypercube model
- [ ] Skill shows `query flex` usage
- [ ] Skill mandates citation format
- [ ] Skill shows verification workflow

---

## For Next Agent

**Start here:**
1. Read this context package
2. Read Spec 12 (12_CLI_HYPERCUBE_SPEC.md)
3. Invoke test-driven-execution skill
4. Begin Phase B: Create test file, run RED, implement, run GREEN

**TDD is mandatory.** Write tests before code. See Spec 12 for test specifications.

---

**Last Updated:** 2025-12-21
**Previous Package:** IMPLEMENTATION_CONTEXT_PACKAGE.md (Phase 0 - specs written)
**Next Action:** Start Phase B - QuerySpec + QueryEngine
