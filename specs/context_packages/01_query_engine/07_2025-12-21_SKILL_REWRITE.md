---
title: "SKILL REWRITE"
package_number: 7
date: 2025-12-21
migrated_from: "apps/context-os/specs/context_os_intelligence/context_packages/07_2025-12-21_SKILL_REWRITE.md"
tags:
  - context-package
  - query-engine
  - legacy
---

# Implementation Context Package 05: CLI Hypercube Complete + Skill Strategic Rewrite

**Status:** ALL PHASES COMPLETE - Ready for Testing
**Created:** 2025-12-22
**Agent Handoff:** Session ended after Phase D strategic skill rewrite

---

## Executive Summary

**The CLI Hypercube Refactor is COMPLETE across all 4 phases:**

| Phase | Status | Tests | Description |
|-------|--------|-------|-------------|
| Phase A | ✅ | 8/8 | JSON output for 6 existing commands |
| Phase B | ✅ | 32/32 | QuerySpec, QueryEngine, `query flex` |
| Phase C | ✅ | 28/28 | QueryReceipt, QueryLedger, `query verify`, `query receipts` |
| Phase D | ✅ | N/A | Skill rewrite: interface docs → strategic methodology |

**Total: 68/68 tests pass**

**Key deliverable:** The `context-query` skill was strategically rewritten from interface documentation (404 lines) to a comprehensive search methodology guide (579 lines).

---

## Background Context

### The Problem Solved

1. **CLI lacked flexible queries** → Added `query flex` with 5D hypercube slicing
2. **Agent claims were unverifiable** → Added receipt system with `query verify`
3. **Skill taught syntax, not semantics** → Rewrote with search strategies

### The 5D Hypercube Model

```
Files × Sessions × Time × Chains × AccessType
```

Every query slices this hypercube and aggregates results.

### Key CLI Commands

```bash
# Primary query command
query flex --files "*pattern*" --agg count,recency,sessions,chains --format json

# Verification
query verify <receipt_id>
query receipts --limit 10
```

---

## Current State

### Files Modified/Created

| File | Lines | Status |
|------|-------|--------|
| `src/context_os_events/query_engine.py` | ~960 | Created (Phases B+C) |
| `src/context_os_events/cli.py` | +~340 | Modified (Phases A+B+C) |
| `tests/test_query_engine.py` | ~350 | Created (Phase B) |
| `tests/test_verification.py` | ~670 | Created (Phase C) |
| `tests/test_cli_query.py` | +~160 | Modified (Phase A) |
| `.claude/skills/context-query/SKILL.md` | 579 | Rewritten (Phase D) |

### Skill Transformation (Phase D)

**Before (404 lines):**
- Interface reference (commands, options)
- Basic query patterns
- Citation requirements

**After (579 lines):**
- Strategy Selection Guide (question → strategy mapping)
- 8 Search Strategies with concrete CLI examples
- Result Interpretation (pattern signatures)
- Multi-Query Workflows
- Index Understanding (what's captured vs not)
- Preserved citation/verification sections

### New Skill Structure

```
SKILL.md (579 lines)
├── Core Principle
├── Quick Reference (default query pattern)
├── Strategy Selection Guide (table: question → strategy → first query)
├── Search Strategies
│   ├── 1. Pilot Drilling (breadth → depth)
│   ├── 2. Known Anchor Expansion
│   ├── 3. Temporal Bracketing
│   ├── 4. Chain Walking
│   ├── 5. Negative Space Search
│   ├── 6. Triangulation
│   ├── 7. Cluster Discovery
│   └── 8. Recency Gradient
├── Result Interpretation
│   ├── Access/Session Pattern Signatures
│   ├── Session Type Patterns (human vs agent)
│   ├── Chain Signatures
│   └── Temporal Patterns
├── Multi-Query Workflows
│   ├── "What am I working on for [Client]?"
│   ├── "What did I abandon?"
│   ├── "Reconstruct what happened last week"
│   └── Progressive Disclosure Pattern
├── Index Understanding
│   ├── What's Captured
│   ├── What's NOT Captured
│   └── Limitations & Gotchas
├── CLI Reference (condensed)
├── Citation Requirements
├── Verification Workflow
└── Success Criteria
```

---

## Jobs To Be Done (Next Agent)

### 1. Test the Rewritten Skill

The skill was rewritten but not fully tested. Need to verify:

- [ ] Strategy Selection Guide leads to correct strategies
- [ ] Search strategies produce useful results
- [ ] Multi-query workflows are followable
- [ ] Result interpretation matches actual data patterns

### 2. Validate Strategy-Question Mapping

Test these questions and verify the skill chooses the right strategy:

| Question | Expected Strategy |
|----------|-------------------|
| "What am I working on for Pixee?" | Pilot Drilling |
| "What's related to cli.py?" | Known Anchor Expansion |
| "What did I abandon?" | Negative Space Search |
| "What happened last Tuesday?" | Temporal Bracketing |
| "Is my claim about 138 files accurate?" | Triangulation |

### 3. Identify Gaps in Strategies

After testing, note any:
- Questions that don't map to existing strategies
- Strategies that don't produce useful results
- Missing interpretation patterns

---

## Test Plan

### Test 1: Pilot Drilling Strategy

**Input:** "What am I working on?"

**Expected behavior:**
1. Agent consults Strategy Selection Guide
2. Selects "Pilot Drilling" strategy
3. Runs: `query flex --time 30d --agg count,recency,sessions,chains --limit 50 --format json`
4. Identifies clusters by path pattern
5. Drills into hot cluster
6. Synthesizes with `[receipt_id]` citations

**Verify:**
- [ ] Used full aggregations (not just count)
- [ ] Cited receipt ID
- [ ] Interpreted patterns (not just dumped data)
- [ ] Told user how to verify

---

### Test 2: Known Anchor Expansion

**Input:** "What's related to query_engine.py?"

**Expected behavior:**
1. Selects "Known Anchor Expansion" strategy
2. Runs: `query co-access query_engine.py --format json`
3. Expands to session context
4. Identifies file relationships

**Verify:**
- [ ] Used co-access command
- [ ] Followed session trails
- [ ] Identified work unit (files that go together)

---

### Test 3: Negative Space Search

**Input:** "What did I abandon?"

**Expected behavior:**
1. Selects "Negative Space Search" strategy
2. Runs: `query flex --time 30d --agg count,recency --limit 50 --format json`
3. Identifies old files (oldest last_access)
4. Checks if chains continued or orphaned
5. Reports potentially abandoned work

**Verify:**
- [ ] Sorted/filtered by recency
- [ ] Checked chain status for old files
- [ ] Distinguished completed vs abandoned

---

### Test 4: Result Interpretation

**Input:** Run `query flex --files "*" --agg count,recency,sessions,chains --limit 10 --format json`

**Verify agent interprets:**
- [ ] High access + low sessions = reference doc
- [ ] Agent sessions (agent-*) vs human sessions (UUID)
- [ ] Chain size signatures (2-5 files vs 20+ files)

---

### Test 5: Multi-Query Workflow

**Input:** "Reconstruct what I did last week"

**Expected behavior:**
1. Runs multiple queries (not just one)
2. Gets week's activity
3. Gets chain overview
4. Gets session details for major chains
5. Synthesizes chronological narrative

**Verify:**
- [ ] Used multiple queries (not single query)
- [ ] Followed the workflow steps
- [ ] Produced narrative (not just data)

---

### Test 6: Citation Compliance

**For any query response, verify:**
- [ ] Every numeric claim has `[receipt_id]`
- [ ] User told how to verify
- [ ] Format: "Found X files [q_abc123]"

---

## Quick Start for Next Agent

### Read These Files First

1. **This context package** (you're reading it)
2. **The skill:** `.claude/skills/context-query/SKILL.md`
3. **Spec 12:** `specs/context_os_intelligence/12_CLI_HYPERCUBE_SPEC.md` (if needed)
4. **Spec 13:** `specs/context_os_intelligence/13_VERIFICATION_LAYER_SPEC.md` (if needed)

### Run Tests to Verify System Works

```bash
cd apps/context_os_events

# All tests should pass (68/68)
.venv/Scripts/python -m pytest tests/ -v

# Specifically:
.venv/Scripts/python -m pytest tests/test_cli_query.py::TestPhaseAJsonOutput -v  # 8 tests
.venv/Scripts/python -m pytest tests/test_query_engine.py -v                      # 32 tests
.venv/Scripts/python -m pytest tests/test_verification.py -v                      # 28 tests
```

### Test the Skill

```bash
# Invoke the skill with test questions:
# 1. "What am I working on for Pixee?"
# 2. "What's related to cli.py?"
# 3. "What did I abandon?"

# Verify the agent:
# - Selects appropriate strategy
# - Runs correct queries
# - Interprets results (not just dumps data)
# - Cites receipt IDs
```

---

## Known Issues / Considerations

1. **Skill not tested end-to-end** - Was rewritten but testing was interrupted
2. **Strategy effectiveness unknown** - Need real-world validation
3. **Progressive disclosure** - Skill says "start at Level 4" but should verify this is practical
4. **Chain walking** - May need `--chain` filter to work properly (verify implementation)

---

## Success Criteria for Testing Phase

- [ ] All 5 test scenarios pass
- [ ] Agent selects appropriate strategy for question type
- [ ] Multi-query workflows are followed (not single queries for complex questions)
- [ ] Result interpretation shows pattern recognition
- [ ] Every claim cites `[receipt_id]`
- [ ] No "raw data dumping" - insights synthesized

---

## File Locations

| File | Purpose |
|------|---------|
| `.claude/skills/context-query/SKILL.md` | The rewritten skill (test this) |
| `src/context_os_events/query_engine.py` | QuerySpec, QueryEngine, QueryReceipt, QueryLedger |
| `src/context_os_events/cli.py` | CLI commands (query flex, verify, receipts) |
| `tests/test_*.py` | Unit and integration tests |
| `specs/.../12_CLI_HYPERCUBE_SPEC.md` | Hypercube specification |
| `specs/.../13_VERIFICATION_LAYER_SPEC.md` | Verification specification |

---

**Last Updated:** 2025-12-22
**Previous Package:** IMPLEMENTATION_CONTEXT_PACKAGE_04.md (Phase C complete)
**Next Action:** Test the rewritten skill with the 5 test scenarios above
