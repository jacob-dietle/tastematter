---
title: "IMPLEMENTATION C"
package_number: 5
date: 2025-12-21
migrated_from: "apps/context-os/specs/context_os_intelligence/context_packages/05_2025-12-21_IMPLEMENTATION_C.md"
tags:
  - context-package
  - query-engine
  - legacy
---

# Implementation Context Package 03: CLI Hypercube + Verification Layer

**Status:** PHASE B COMPLETE - Ready for Phase C
**Created:** 2025-12-21
**Agent Handoff:** Session ended after Phase B implementation (TDD complete)

---

## Executive Summary

**Phases A and B are COMPLETE.** The CLI now has:
1. JSON output for all 6 existing query commands (`--format json`)
2. A new `query flex` command for flexible hypercube slicing
3. QuerySpec, QueryResult, and QueryEngine classes

**Next agent should start Phase C:** Add QueryReceipt, QueryLedger, and `query verify` command.

---

## Read These Files First (In Order)

### Step 1: Understand the Specifications (10 min)

1. **`specs/context_os_intelligence/12_CLI_HYPERCUBE_SPEC.md`**
   - Hypercube model (5D: Files × Sessions × Time × Chains × AccessType)
   - QuerySpec, QueryResult type contracts
   - QueryEngine implementation details
   - **Phase B tests (already implemented)**

2. **`specs/context_os_intelligence/13_VERIFICATION_LAYER_SPEC.md`**
   - QueryReceipt, VerificationResult type contracts
   - QueryLedger storage design (30-day TTL)
   - `query verify` command interface
   - **Phase C tests (to be implemented)**

### Step 2: Understand What Was Implemented (15 min)

3. **`src/context_os_events/query_engine.py`** (NEW - ~400 lines)
   - `QuerySpec` dataclass with validation
   - `QueryResult` dataclass with `to_json()` and `to_table()`
   - `QueryEngine` class with slice → aggregate → render pipeline
   - Already generates `receipt_id` (ready for Phase C)

4. **`src/context_os_events/cli.py`**
   - Lines 107-139: `format_option` and `results_to_json()` (Phase A)
   - Lines 1668-1753: `query flex` command (Phase B)

5. **`tests/test_query_engine.py`** (NEW - ~350 lines)
   - 32 tests for QuerySpec, QueryEngine, and CLI integration
   - All pass (52 seconds)

6. **`tests/test_cli_query.py`**
   - Lines 873-1033: `TestPhaseAJsonOutput` (8 tests, all pass)

---

## Background Context: The Problem

### The 5D Hypercube Model

The Context OS index treats file access data as a 5-dimensional hypercube:

```
┌─────────────────────────────────────────────────────────────────┐
│                    CONTEXT HYPERCUBE                            │
├─────────────────────────────────────────────────────────────────┤
│  Dimension 1: FILES      - All file paths ever touched          │
│  Dimension 2: SESSIONS   - Claude Code session UUIDs            │
│  Dimension 3: TIME       - Temporal axis (days, weeks)          │
│  Dimension 4: CHAINS     - Conversation chains (leafUuid)       │
│  Dimension 5: ACCESS_TYPE - read | write | create               │
├─────────────────────────────────────────────────────────────────┤
│  Every "query" = slice + aggregate + render                     │
└─────────────────────────────────────────────────────────────────┘
```

### Why Verification Matters (Phase C)

Agent synthesizes from query results, but synthesis is **unverifiable**:
- Agent: "You worked on 138 Pixee files"
- User: "How do I verify that?"
- Currently: No answer - query results are ephemeral

**Solution:** Every query returns a `receipt_id`. User can run `query verify <receipt_id>` to confirm results match or detect drift.

---

## Work Completed

### Phase A: JSON Output (COMPLETE)

| Command | Status | Evidence |
|---------|--------|----------|
| `query search` | ✅ | `--format json` works |
| `query file` | ✅ | `--format json` works |
| `query session` | ✅ | `--format json` works |
| `query chains` | ✅ | `--format json` works |
| `query co-access` | ✅ | `--format json` works |
| `query recent` | ✅ | `--format json` works |

**Tests:** 8/8 pass (`TestPhaseAJsonOutput`)

### Phase B: QuerySpec + QueryEngine + query flex (COMPLETE)

| Component | Status | Location |
|-----------|--------|----------|
| `QuerySpec` | ✅ | `query_engine.py:44-99` |
| `QueryResult` | ✅ | `query_engine.py:102-158` |
| `QueryEngine` | ✅ | `query_engine.py:165-450` |
| `query flex` | ✅ | `cli.py:1668-1753` |

**Tests:** 32/32 pass (`test_query_engine.py`)

**TDD Workflow Executed:**
1. RED: Created 32 tests → all failed (module not found)
2. GREEN: Implemented code → all 32 tests pass
3. Verified: Phase A tests still pass (no regression)

### Manual Test Result

```bash
context-os query flex --files "*pixee*" --limit 5 --format json
```

Output:
```json
{
  "receipt_id": "q_82db3d",
  "timestamp": "2025-12-22T00:31:34.274528+00:00",
  "query": {
    "files": "*pixee*",
    "time": null,
    "agg": ["count"],
    "limit": 5,
    "sort": "count"
  },
  "result_count": 138,
  "results": [...],
  "aggregations": {
    "count": {"total_files": 138, "total_accesses": 195}
  }
}
```

---

## Next Steps: Phase C (Verification Layer)

### TDD Workflow

1. **Create test file** `tests/test_verification.py` with tests from Spec 13
2. **Run tests** - should fail (RED)
3. **Implement:**
   - `QueryReceipt` dataclass in `query_engine.py`
   - `QueryLedger` class for storage
   - `verify()` method on QueryEngine
   - `query verify` and `query receipts` CLI commands
4. **Run tests** - should pass (GREEN)

### Type Contracts (from Spec 13)

```python
@dataclass
class QueryReceipt:
    """Verifiable record of a query execution."""
    receipt_id: str                      # "q_" + 6 hex chars
    timestamp: str                       # ISO format
    query_spec: QuerySpec                # The query that was run
    result_hash: str                     # sha256 of canonical JSON results
    result_count: int                    # Total results
    result_snapshot: List[dict]          # Full results (for audit)

    @staticmethod
    def generate_id(timestamp: str, query: QuerySpec) -> str:
        """Generate deterministic receipt ID."""
        content = f"{timestamp}:{query}"
        return "q_" + hashlib.sha256(content.encode()).hexdigest()[:6]

    def compute_hash(self) -> str:
        """Compute hash of results for verification."""
        canonical = json.dumps(self.result_snapshot, sort_keys=True)
        return "sha256:" + hashlib.sha256(canonical.encode()).hexdigest()


@dataclass
class VerificationResult:
    """Result of verifying a query receipt."""
    receipt_id: str
    original_timestamp: str
    verification_timestamp: str
    status: Literal["MATCH", "DRIFT"]
    original_hash: str
    current_hash: str
    drift_summary: Optional[str]         # "3 new files, 2 removed"


class QueryLedger:
    """Storage for query receipts with 30-day TTL."""

    def __init__(self, path: Path, ttl_days: int = 30):
        self.path = path
        self.ttl_days = ttl_days

    def save(self, receipt: QueryReceipt) -> None: ...
    def load(self, receipt_id: str) -> Optional[QueryReceipt]: ...
    def cleanup(self) -> int: ...  # Returns count of deleted receipts
```

### CLI Commands to Add

```bash
# Verify a query receipt
context-os query verify <receipt_id> [--verbose]

# List recent receipts
context-os query receipts [--limit N]
```

### Ledger Storage Location

```
~/.context-os/query_ledger/
├── q_82db3d.json
├── q_abc123.json
└── ...
```

### Key Tests to Copy from Spec 13

```python
def test_receipt_id_is_deterministic():
    """Same query at same timestamp produces same receipt_id."""

def test_receipt_hash_is_deterministic():
    """Same results produce same hash."""

def test_ledger_saves_and_loads_receipt():
    """Receipts can be saved and loaded by ID."""

def test_ledger_ttl_cleanup():
    """Receipts older than 30 days are deleted on cleanup."""

def test_verify_match_when_data_unchanged():
    """Verification returns MATCH when data hasn't changed."""

def test_verify_drift_when_data_changed():
    """Verification returns DRIFT when results differ."""

def test_e2e_query_receipt_verify_cycle():
    """Complete workflow: query → receipt → verify."""
```

---

## File Locations Summary

| File | Purpose | Status |
|------|---------|--------|
| `specs/.../12_CLI_HYPERCUBE_SPEC.md` | Hypercube spec + Phase B tests | ✅ Complete |
| `specs/.../13_VERIFICATION_LAYER_SPEC.md` | Verification spec + Phase C tests | READ for Phase C |
| `src/context_os_events/query_engine.py` | QuerySpec, QueryResult, QueryEngine | ✅ Complete |
| `src/context_os_events/cli.py` | CLI commands | ✅ Phase A+B complete |
| `tests/test_query_engine.py` | Phase B tests (32 tests) | ✅ All pass |
| `tests/test_cli_query.py` | Phase A tests (8 tests) | ✅ All pass |
| `tests/test_verification.py` | Phase C tests | CREATE in Phase C |

---

## Test Commands

```bash
cd apps/context_os_events

# Verify Phase A tests still pass
.venv/Scripts/python -m pytest tests/test_cli_query.py::TestPhaseAJsonOutput -v

# Verify Phase B tests still pass
.venv/Scripts/python -m pytest tests/test_query_engine.py -v

# Run Phase C tests (after creating them)
.venv/Scripts/python -m pytest tests/test_verification.py -v

# Manual test of query flex
"C:/Users/dietl/.context-os/bin/context-os.cmd" query flex --files "*pixee*" --format json

# Manual test of verify (after Phase C)
"C:/Users/dietl/.context-os/bin/context-os.cmd" query verify q_82db3d
```

---

## Success Criteria Checklist

### Phase A (COMPLETE)
- [x] All 6 query commands support `--format json`
- [x] JSON output is valid and parseable
- [x] Default format is `table` (backwards compatible)
- [x] All Phase A tests pass (8/8)

### Phase B (COMPLETE)
- [x] QuerySpec dataclass implemented with validation
- [x] QueryResult dataclass with to_json() and to_table()
- [x] QueryEngine executes slice → aggregate → render pipeline
- [x] `query flex` command works with all options
- [x] Multi-filter queries work (AND logic)
- [x] All Phase B tests pass (32/32)

### Phase C (TODO)
- [ ] QueryReceipt dataclass with generate_id() and compute_hash()
- [ ] QueryLedger with save(), load(), cleanup()
- [ ] 30-day TTL enforced
- [ ] verify() method detects MATCH vs DRIFT
- [ ] `query verify` command works
- [ ] `query receipts` command works
- [ ] All Phase C tests pass

### Phase D (TODO)
- [ ] Skill documents hypercube model
- [ ] Skill shows `query flex` usage
- [ ] Skill mandates citation format `[receipt_id]`
- [ ] Skill shows verification workflow

---

## Common Patterns Used

### Mock vs Real Index

The QueryEngine supports both mock objects (for testing) and real ContextIndex:

```python
# Check for mock structure first (Mock auto-creates attributes)
if hasattr(self.index, 'file_sessions') and isinstance(self.index.file_sessions, dict):
    return list(self.index.file_sessions.keys())
# Then check for real ContextIndex
elif hasattr(self.index, '_inverted_index') and isinstance(self.index._inverted_index, dict):
    return list(self.index._inverted_index.keys())
```

### Receipt ID Generation

Already implemented in `query_engine.py`:

```python
def _generate_receipt_id(self, timestamp: str, spec: QuerySpec) -> str:
    content = f"{timestamp}:{spec.files}:{spec.time}:{spec.agg}"
    hash_hex = hashlib.sha256(content.encode()).hexdigest()[:6]
    return f"q_{hash_hex}"
```

Phase C needs to:
1. Save the full receipt to ledger
2. Add result hash for verification
3. Implement verify() to compare hashes

---

## For Next Agent

**Start here:**
1. Read this context package
2. Read Spec 13 (`13_VERIFICATION_LAYER_SPEC.md`)
3. Invoke test-driven-execution skill
4. Create `tests/test_verification.py` with tests from Spec 13
5. Run tests (RED)
6. Implement QueryReceipt, QueryLedger, verify(), CLI commands
7. Run tests (GREEN)

**TDD is mandatory.** Write tests before code. See Spec 13 for test specifications.

**Key insight:** The `receipt_id` is already being generated in Phase B. Phase C adds:
- Saving full receipt to disk
- Computing result hash
- verify() to re-run query and compare hashes

---

**Last Updated:** 2025-12-21
**Previous Package:** IMPLEMENTATION_CONTEXT_PACKAGE_02.md (Phase A complete)
**Next Action:** Start Phase C - Verification Layer (QueryReceipt, QueryLedger, verify)
