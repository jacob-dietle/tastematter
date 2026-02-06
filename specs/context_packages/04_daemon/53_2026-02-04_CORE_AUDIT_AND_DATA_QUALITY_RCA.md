---
title: "Tastematter Context Package 53"
package_number: 53
date: 2026-02-04
status: current
previous_package: "[[52_2026-02-04_TIMESTAMP_BUG_FIX_AND_RELEASE]]"
related:
  - "[[core/src/capture/jsonl_parser.rs]]"
  - "[[core/src/query.rs]]"
  - "[[core/src/daemon/sync.rs]]"
  - "[[core/src/storage.rs]]"
  - "[[core/src/types.rs]]"
  - "[[core/src/main.rs]]"
  - "[[intel/src/index.ts]]"
  - "[[intel/src/types/shared.ts]]"
  - "[[core/src/intelligence/client.rs]]"
  - "[[core/src/intelligence/types.rs]]"
  - "[[specs/canonical/12_CONTEXT_RESTORATION_API_SPEC.md]]"
  - "[[specs/implementation/phase_04_core_improvements/01_HEAT_COMMAND_SPEC.md]]"
tags:
  - context-package
  - tastematter
  - data-quality
  - rca
  - core-audit
---

# Tastematter - Context Package 53

## Executive Summary

**FULL CORE AUDIT CONDUCTED. NEW DATA QUALITY BUG FOUND. END-TO-END RCA NEEDED.** Applied epistemic grounding + context gap analysis skills to prepare for context restoration implementation. Live CLI testing revealed that recent sessions (post-Feb 4 sync) have `files_read: []` despite the daemon running - a separate bug from the timestamp fix in pkg 52. Intel service is not running, zero chain enrichment happening. Context restoration blocked on data quality RCA before implementation proceeds.

## What Was Done This Session

### 1. Skills Applied: Epistemic Grounding + Context Gap Analysis

Ran both skills systematically against tastematter core + CLI:

**Epistemic grounding findings:**
- Context sensitivity: HIGH (data formats, parsing, multi-component architecture)
- 13 canonical specs found, 168 total documentation files
- All assumptions about Rust core verified STRONG
- Intel service assumptions DISPROVEN (see section 3)

**Context gap analysis findings:**
- No existing `context` CLI subcommand (TRUE GAP)
- Existing query primitives (flex, co-access, chains, sessions) → PATTERN GAP
- Context package file-system discovery → TRUE GAP (query engine is DB-only)
- Intel synthesis endpoint → TRUE GAP (doesn't exist yet)

### 2. Codebase Inventory (Complete)

| Component | Language | Source Files | Tests | Status |
|-----------|----------|-------------|-------|--------|
| Rust core (query engine) | Rust | 31 files | 269 (259 lib + 10 integration) | SHIPPED v0.1.0-alpha.15 |
| Python indexer (daemon) | Python | 47 files | 495 | LEGACY (being replaced) |
| Intel service | TypeScript | 21 files | 181 passing, 8 failing | 67% complete |
| Frontend (Tauri/Svelte) | Svelte/TS | 42 files | ~20 | PAUSED |
| Total | - | 141 files | ~805 | - |

### 3. Intel Service Audit

**7 endpoints exist, all coded:**

| Endpoint | Agent | Purpose |
|----------|-------|---------|
| `POST /api/intel/name-chain` | chain-naming.ts | Name chains from files/sessions |
| `POST /api/intel/name-chain-ab` | chain-naming.ts | A/B test naming quality |
| `POST /api/intel/summarize-chain` | chain-summary.ts | Chain summary + workstream tags |
| `POST /api/intel/analyze-commit` | commit-analysis.ts | Commit risk analysis |
| `POST /api/intel/summarize-session` | session-summary.ts | Session summary + focus area |
| `POST /api/intel/generate-insights` | insights.ts | Pattern detection |
| `POST /api/intel/gitops-decide` | gitops-decision.ts | Intelligent git ops |

**Critical findings:**
- `POST /api/intel/synthesize-context` does NOT exist (needed for context restoration Phase 2)
- Rust IntelClient only wraps 2 of 7 endpoints: `name_chain()` and `summarize_chain()`
- Intel service was NOT RUNNING during testing → zero chain enrichment in DB
- Graceful degradation pattern works (returns `Ok(None)` on failure)
[VERIFIED: intel/src/index.ts endpoints, core/src/intelligence/client.rs methods]

### 4. Live CLI Testing - Data Quality Bug Found

**Commands run and results:**

| Command | Result | Verdict |
|---------|--------|---------|
| `tastematter daemon status` | Installed: Yes, Running: Yes | OK |
| `tastematter intel health` | UNAVAILABLE (localhost:3002) | NOT RUNNING |
| `tastematter query chains --limit 10` | Returns chains, NO `generated_name` fields | DEGRADED (no intel) |
| `tastematter query flex --time 7d` | Returns file data with access counts | WORKS (older data) |
| `tastematter query sessions --time 7d` | ALL sessions: `file_count: 0, duration_seconds: 0` | **BUG** |
| `tastematter query co-access workstreams.yaml` | Correct PMI scores, meaningful results | WORKS |
| `tastematter query search "intel"` | Returns files with access counts | WORKS |

**NEW BUG: Empty recent sessions**

Recent sessions (post-Feb 4 daemon sync) all show:
```json
{
  "session_id": "...",
  "file_count": 0,
  "total_accesses": 0,
  "duration_seconds": 0,
  "files": [],
  "top_files": []
}
```

The daemon is creating session records but NOT populating `files_read`. Session shells exist, file data doesn't. This is likely the same class of bug as pkg 47 (daemon parsed but never persisted) but for a different code path.

**Chain explosion:** 129 chains in 7d window, most single-session with 0 files. Inflated chain count from empty sessions getting unique chain assignments.

**What still works:** `query flex` and `query co-access` work because they query `files_read` JSON arrays from OLDER sessions that do have data. They're returning correct but stale results.
[VERIFIED: CLI output captured 2026-02-04]

### 5. Architecture Layering (Confirmed from pkg 51)

```
Layer 3: SYNTHESIS (context restoration)  ← BLOCKED on Layers 0-2
Layer 2: DERIVED METRICS (heat command)   ← UNBLOCKED (timestamps fixed)
Layer 1: PRIMITIVES (flex, co-access)     ← WORKS (but stale data)
Layer 0: DATA QUALITY                     ← BROKEN (empty recent sessions)
```

Context restoration cannot ship on broken data. RCA first.

## Current State

### Data Quality Issues (Priority Order)

| ID | Issue | Severity | Status | Location |
|----|-------|----------|--------|----------|
| DQ-001 | Timestamp bug (all identical) | P0 | **FIXED** (pkg 52) | jsonl_parser.rs:412 |
| DQ-002 | Recent sessions have `files_read: []` | **P0** | **NEW - OPEN** | Daemon sync path (sync.rs? jsonl_parser.rs?) |
| DQ-003 | `duration_seconds: 0` for recent sessions | P1 | OPEN | Session boundary detection |
| DQ-004 | Chain explosion (129 chains/7d, most empty) | P2 | OPEN (consequence of DQ-002) | chain_graph logic |
| DQ-005 | `access_count == session_count` for all files | P2 | UNKNOWN (may be correct) | flex query SQL |
| DQ-006 | Intel service not auto-starting with daemon | P3 | OPEN | No integration exists |

### Decision Queue
| ID | Item | Status |
|----|------|--------|
| dq_006 | RCA empty sessions before new features | **NEW - BLOCKING** |
| dq_007 | Intel service auto-start with daemon? | NEW |

### Stale CLAUDE.md

`apps/tastematter/CLAUDE.md` is significantly stale:
- Says "04_daemon: 1 package" (actual: 53)
- Says "chain linking broken" (fixed in daemon pkg 02)
- Missing intel service entirely
- Missing release infrastructure
- Missing telemetry
Should be updated after RCA resolves.

## RCA Investigation Plan (Next Session)

### Phase 1: Trace the Write Path (DQ-002)

The daemon sync path needs end-to-end tracing:

```
JSONL files on disk
    ↓ (1) jsonl_parser.rs parses into SessionSummary
    ↓ (2) SessionSummary converts to SessionInput (types.rs From impl)
    ↓ (3) sync.rs calls insert_session() or upsert_session()
    ↓ (4) query.rs:query_sessions() reads back
```

**Hypothesis A:** Parser extracts files but conversion drops them
- Check: `SessionSummary.files_read` → `SessionInput.files_read` conversion (types.rs:518-535)
- The `From<SessionSummary> for SessionInput` impl serializes `files_read` to JSON string

**Hypothesis B:** Upsert overwrites existing file data with empty
- Check: `upsert_session()` in query.rs:1202 - does it overwrite `files_read`?
- If session already exists from earlier sync, upsert might blank the files

**Hypothesis C:** Incremental mode skips file parsing
- Check: `--incremental` flag behavior in parse-sessions
- Daemon may use incremental mode that skips already-seen sessions

**Hypothesis D:** Daemon sync path doesn't call file extraction
- Check: `sync.rs` - does `run_sync()` call `parse-sessions` with correct flags?
- May create session records without running full parse

### Phase 2: Verify Each Step

```bash
# Step 1: Does the parser extract files?
tastematter parse-sessions --project "." --format json | head -5
# Check if files_read is populated in output

# Step 2: Does the DB have files for older sessions?
tastematter query sessions --time 90d --limit 5 --format json
# Compare old vs new sessions

# Step 3: What does the daemon actually call?
# Read sync.rs to trace the exact sequence
```

### Phase 3: Fix + Regression Test

Once root cause identified:
- Fix the specific code path
- Add regression test (session with files_read must persist)
- Re-run daemon sync
- Verify with `query sessions --time 7d`

## File Locations

| File | Purpose | Relevance to RCA |
|------|---------|-------------------|
| [[core/src/daemon/sync.rs]] | Daemon sync orchestration | **PRIMARY** - traces what gets called |
| [[core/src/capture/jsonl_parser.rs]] | JSONL → SessionSummary | Check file extraction |
| [[core/src/types.rs]] | SessionSummary → SessionInput conversion | Check From impl (line 518) |
| [[core/src/query.rs]] | insert_session/upsert_session | Check upsert behavior (line 1202) |
| [[core/src/storage.rs]] | DB connection, schema | Check table schema |
| [[core/src/main.rs]] | CLI commands, daemon subcommands | Check what daemon calls |
| [[intel/src/index.ts]] | Intel service entry point | 7 endpoints, none for synthesis |
| [[intel/src/types/shared.ts]] | Zod schemas for all endpoints | Type contracts |
| [[core/src/intelligence/client.rs]] | Rust → TS HTTP client | Only 2 of 7 endpoints wrapped |
| [[core/src/intelligence/types.rs]] | Rust-side intel types | Matches TS schemas |

## For Next Agent

**Context Chain:**
- Previous: [[52_2026-02-04_TIMESTAMP_BUG_FIX_AND_RELEASE]] (timestamp fix, clippy cleanup)
- This package: Full core audit, new data quality bug found, RCA plan
- Next action: RCA the empty sessions bug (DQ-002)

**Start here:**
1. Read this package for the full audit findings
2. Read [[core/src/daemon/sync.rs]] - trace what `run_sync()` actually does
3. Read [[core/src/capture/jsonl_parser.rs]] - check if files are extracted
4. Read [[core/src/types.rs]] lines 518-535 - check From conversion
5. Read [[core/src/query.rs]] line 1202 - check upsert behavior
6. Run: `tastematter parse-sessions --project "." --format summary` to verify parser output

**Do NOT:**
- Start implementing context restoration or heat command until DQ-002 is resolved
- Assume the data is correct - verify every step of the write path
- Skip reading sync.rs - that's where the daemon orchestration lives
- Trust the session query output at face value (many ghost sessions)

**Key insight:**
The daemon creates session records but doesn't populate their file lists. This is the same class of bug as pkg 47 (parsed but never persisted) but possibly for a different field or code path. The fix may be as simple as ensuring `files_read` is included in the upsert, or it could reveal deeper issues in the sync orchestration. RCA first, then build features on solid data.
[INFERRED: From CLI output showing 0-file sessions + comparison with pkg 47 pattern]
