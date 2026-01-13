---
title: "IMPLEMENTATION A"
package_number: 3
date: 2025-12-21
migrated_from: "apps/context-os/specs/context_os_intelligence/context_packages/03_2025-12-21_IMPLEMENTATION_A.md"
tags:
  - context-package
  - query-engine
  - legacy
---

# Implementation Context Package: CLI Hypercube + Verification Layer

**Status:** READY FOR IMPLEMENTATION
**Created:** 2025-12-21
**Estimated Total Effort:** 6-8 hours across 4 phases

---

## Executive Summary

Two specs are complete and ready for TDD implementation:

1. **Spec 12: CLI Hypercube Refactor** - Flexible query interface treating index as 5D hypercube
2. **Spec 13: Verification Layer** - Query receipts, audit trail, agent verification

**Core Insight:** Query = Slice + Aggregate + Render. Every query returns a receipt for verification.

---

## Read These Files First (In Order)

### Step 1: Understand the Specs (20 min)

1. `specs/context_os_intelligence/12_CLI_HYPERCUBE_SPEC.md`
   - Type contracts: QuerySpec, QueryResult, row types
   - Test specifications: 35+ tests
   - Implementation steps for Phases A & B

2. `specs/context_os_intelligence/13_VERIFICATION_LAYER_SPEC.md`
   - Type contracts: QueryReceipt, VerificationResult, QueryLedger
   - Test specifications: 30+ tests
   - Implementation steps for Phase C

### Step 2: Understand Existing Code (15 min)

3. `src/context_os_events/cli.py`
   - Lines 1-100: CLI structure, Click groups
   - `query` group and existing commands (search, file, session, chains, co-access, recent)
   - Focus on how commands call ContextIndex

4. `src/context_os_events/index/context_index.py`
   - ContextIndex class methods used by CLI
   - `get_sessions_for_file()`, `get_files_for_session()`, `get_co_accessed()`

### Step 3: Review the Skill (5 min)

5. `.claude/skills/context-query/SKILL.md`
   - Current CLI invocation patterns
   - Will need update in Phase D for citations

---

## Implementation Phases

### Phase A: Add JSON Output to Existing Commands (~300 lines, 1-2 hours)

**Goal:** All 6 existing query commands support `--format json`

**TDD Workflow:**
1. Copy test cases from Spec 12 → `tests/test_cli_query_commands.py`
2. Run tests (RED - should fail)
3. Implement `--format` option for each command
4. Run tests (GREEN - should pass)

**Commands to modify:**
- `query search` - add format option, JSON serialization
- `query file` - add format option, JSON serialization
- `query session` - add format option, JSON serialization
- `query chains` - add format option, JSON serialization
- `query co-access` - add format option, JSON serialization
- `query recent` - add format option, JSON serialization

**Pattern for each command:**
```python
format_option = click.option(
    "--format", "output_format",
    type=click.Choice(["json", "table"]),
    default="table",
    help="Output format"
)

# In command function:
if output_format == "json":
    click.echo(json.dumps({...}, indent=2))
else:
    # existing Rich table output
```

**Success criteria:**
- [ ] All 6 commands accept `--format json`
- [ ] JSON output is valid and parseable
- [ ] Default is `table` (backwards compatible)
- [ ] All Phase A tests pass

---

### Phase B: QuerySpec + QueryEngine (~200 lines, 2-3 hours)

**Goal:** Create flexible query interface with hypercube model

**TDD Workflow:**
1. Copy test cases from Spec 12:
   - `test_query_spec.py` (~80 lines)
   - `test_query_engine_slicing.py` (~120 lines)
   - `test_query_engine_aggregation.py` (~100 lines)
   - `test_query_engine_rendering.py` (~60 lines)
2. Run tests (RED)
3. Create `query_engine.py` with QuerySpec, QueryResult, QueryEngine
4. Run tests (GREEN)
5. Add `query flex` CLI command
6. Run integration tests

**Files to create:**
- `src/context_os_events/query_engine.py` - New file with:
  - `QuerySpec` dataclass (~40 lines)
  - `QueryResult` dataclass (~30 lines)
  - `QueryEngine` class (~130 lines)

**Files to modify:**
- `cli.py` - Add `query flex` command (~50 lines)

**Key implementation details:**
```python
# QueryEngine.execute() flow:
def execute(self, spec: QuerySpec) -> QueryResult:
    errors = spec.validate()
    if errors:
        raise ValueError(f"Invalid query: {errors}")

    data = self._slice(spec)           # Filter by files/time/chain/access
    results, aggs = self._aggregate(data, spec)  # Compute stats
    results = self._sort_and_limit(results, spec)

    return QueryResult(
        receipt_id=self._generate_receipt_id(spec),
        timestamp=datetime.now(timezone.utc).isoformat(),
        query=spec,
        result_count=len(data),
        results=results,
        aggregations=aggs,
    )
```

**Success criteria:**
- [ ] QuerySpec validates aggregations, access types, limits
- [ ] QueryEngine slices by file pattern, time, chain, access
- [ ] QueryEngine aggregates count, recency, sessions, chains
- [ ] QueryEngine sorts and limits correctly
- [ ] `query flex` command works with all options
- [ ] JSON output includes all fields
- [ ] All Phase B tests pass

---

### Phase C: Verification Layer (~300 lines, 2-3 hours)

**Goal:** Every query returns verifiable receipt; user can verify claims

**TDD Workflow:**
1. Copy test cases from Spec 13:
   - `test_query_receipt.py` (~100 lines)
   - `test_query_ledger.py` (~100 lines)
   - `test_verification.py` (~100 lines)
   - `test_verification_e2e.py` (~80 lines)
2. Run tests (RED)
3. Implement QueryReceipt, QueryLedger, verify()
4. Run tests (GREEN)
5. Add CLI commands
6. Run E2E tests

**Files to modify:**
- `query_engine.py` - Add:
  - `QueryReceipt` dataclass (~60 lines)
  - `QueryLedger` class (~80 lines)
  - `VerificationResult` dataclass (~30 lines)
  - `QueryEngine.verify()` method (~50 lines)

- `cli.py` - Add:
  - `query verify` command (~50 lines)
  - `query receipts` command (~50 lines)

**Ledger storage:**
- Location: `~/.context-os/query_ledger/`
- Format: One JSON file per receipt (`q_abc123.json`)
- TTL: 30 days rolling cleanup

**Key implementation details:**
```python
# Receipt ID generation (content-addressed):
def generate_id(timestamp, query_spec, results):
    content = json.dumps({...}, sort_keys=True)
    return f"q_{hashlib.sha256(content.encode()).hexdigest()[:6]}"

# Verification flow:
def verify(receipt_id, verbose=False):
    receipt = ledger.load(receipt_id)
    if not receipt:
        return VerificationResult(status="NOT_FOUND")

    current = self.execute(receipt.query_spec)
    if current.receipt_id == receipt.receipt_id:
        return VerificationResult(status="MATCH")
    else:
        return VerificationResult(status="DRIFT", ...)
```

**Success criteria:**
- [ ] Receipt IDs are deterministic (same query = same ID)
- [ ] Ledger saves/loads receipts correctly
- [ ] 30-day TTL enforced
- [ ] verify() detects MATCH vs DRIFT
- [ ] `query verify` command works
- [ ] `query receipts` command lists recent
- [ ] All Phase C tests pass

---

### Phase D: Update Context-Query Skill (~50 lines, 30 min)

**Goal:** Agent knows new interface and citation requirements

**File to modify:** `.claude/skills/context-query/SKILL.md`

**Add sections:**
1. Hypercube model explanation
2. `query flex` examples
3. Citation format requirements
4. Verification workflow

**Citation format:**
```markdown
Based on query results [q_7f3a2b], you worked on 147 Pixee files...

To verify: `context-os query verify q_7f3a2b`
```

**Success criteria:**
- [ ] Skill documents hypercube model
- [ ] Skill shows `query flex` usage
- [ ] Skill mandates citation format `[receipt_id]`
- [ ] Skill shows verification workflow

---

## Test Commands

```bash
# Run all tests
cd apps/context_os_events
.venv/Scripts/python -m pytest tests/ -v

# Run specific test file
.venv/Scripts/python -m pytest tests/test_query_spec.py -v

# Run tests matching pattern
.venv/Scripts/python -m pytest tests/ -k "query_engine" -v
```

---

## CLI Wrapper Reminder

The CLI uses a wrapper script. Test commands like:

```bash
# Using wrapper (from any directory)
"C:/Users/dietl/.context-os/bin/context-os.cmd" query flex --files "*pixee*" --format json

# Or directly from venv
cd apps/context_os_events
.venv/Scripts/python -m context_os_events.cli query flex --files "*pixee*" --format json
```

---

## Common Pitfalls

1. **Path handling:** Windows paths need forward slashes in JSON, backslashes in filesystem
2. **Time parsing:** Use `datetime.fromisoformat()` with `.replace("Z", "+00:00")` for UTC
3. **JSON serialization:** Use `default=str` for datetime objects
4. **Ledger directory:** Create with `mkdir(parents=True, exist_ok=True)`
5. **Hash determinism:** Always use `sort_keys=True` in `json.dumps()`

---

## File Locations Summary

| File | Purpose |
|------|---------|
| `specs/.../12_CLI_HYPERCUBE_SPEC.md` | Hypercube spec + tests |
| `specs/.../13_VERIFICATION_LAYER_SPEC.md` | Verification spec + tests |
| `src/context_os_events/cli.py` | CLI commands (modify) |
| `src/context_os_events/query_engine.py` | QuerySpec, QueryEngine, QueryReceipt (create) |
| `tests/test_query_*.py` | Test files (create) |
| `.claude/skills/context-query/SKILL.md` | Skill docs (update) |

---

## Success Metrics (Overall)

After all phases:
- [ ] All query commands support `--format json`
- [ ] `query flex` supports multi-dimensional slicing
- [ ] Every query returns a `receipt_id`
- [ ] `query verify` confirms or detects drift
- [ ] Skill updated with citation requirements
- [ ] All tests pass (unit + integration + E2E)
- [ ] Backwards compatible (existing commands work unchanged)

---

**Last Updated:** 2025-12-21
**Next Action:** Start Phase A - copy tests, run RED, implement, run GREEN
