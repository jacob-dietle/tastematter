---
title: "Tastematter Context Package 57"
package_number: 57
date: 2026-02-06
status: current
previous_package: "[[56_2026-02-06_VERIFICATION_AND_RELEASE_ALPHA17]]"
related:
  - "[[specs/canonical/14_CODEBASE_AUDIT_2026-02-06.md]]"
  - "[[specs/canonical/07_CLAUDE_CODE_DATA_MODEL_V2.md]]"
  - "[[specs/canonical/12_CONTEXT_RESTORATION_API_SPEC.md]]"
  - "[[specs/canonical/02_ROADMAP.md]]"
  - "[[specs/audits/2026-02-06/audit_data_pipeline.md]]"
  - "[[specs/audits/2026-02-06/audit_runtime_services.md]]"
  - "[[specs/audits/2026-02-06/audit_frontend_and_specs.md]]"
  - "[[specs/audits/2026-02-06/cross_check_data_pipeline.md]]"
tags:
  - context-package
  - tastematter
  - audit
  - data-model
  - epistemic-grounding
  - strategic-fork
---

# Tastematter - Context Package 57

## Executive Summary

Completed full codebase audit using 3-agent team (data-pipeline, runtime-services, frontend-and-specs) with 2 cross-verifications. Audited all 30 Rust source files, 48+ Svelte/TS frontend files, and 15 canonical specs against the v2 Data Model Spec. Found 14 bugs, 4 cross-check findings, 16 gaps, 8 dead code areas. Applied epistemic grounding skill to assess confidence. Ran live CLI queries to validate findings against real UX — discovered 3 additional issues not caught by static audit (path duplication, missing chain names, heat noise). Produced strategic analysis of two development forks: foundational data accuracy vs contextual enrichment.

## Global Context

### Architecture (Two-Layer)
```
Layer 2: Intelligence (TypeScript sidecar at localhost:3002)
  - Chain naming, summarization, future context restoration
  - Graceful degradation (works without it)

Layer 1: Deterministic Index (Rust binary)
  - JSONL parsing → SQLite → queries
  - <50ms response times achieved
  - Daemon syncs every 30 min
```

### What This Audit Established

The v2 Data Model Spec (built by prior 4-agent team) is the ground truth. This audit checked whether the Rust code actually implements it. Key finding: **the code extracts 77.6% of records but only 21% of linking mechanisms** (3 of 14). The parser captures file paths but discards message content, token usage, conversation order, and agent identity beyond filename heuristics.

## Session Activity

### 1. Team Audit (3 agents, ~15 min parallel)

**Agent 1 (data-pipeline):** Audited capture/, index/, storage.rs, query.rs, types.rs
- 10 bugs found, 12 gaps identified
- chain_graph.rs confirmed fundamentally correct
- persist_chains() destructive DROP confirmed as highest-severity bug
[VERIFIED: [[specs/audits/2026-02-06/audit_data_pipeline.md]]]

**Agent 2 (runtime-services):** Audited daemon/, intelligence/, telemetry/, http.rs, main.rs
- 7 critical gaps: DaemonState unwired, chains destructive, inverted index not persisted, telemetry disabled in daemon, GitOps dead, 4 empty Intel tables, HTTP missing 6 queries
- Windows support confirmed functional (VBS startup script approach)
[VERIFIED: [[specs/audits/2026-02-06/audit_runtime_services.md]]]

**Agent 3 (frontend-and-specs):** Audited all frontend files + 15 canonical specs
- 23 components all functional, 8 Tauri commands, 6 stores (2 dead)
- 4 frontend bugs (WorkstreamView wrong method, duplicate code, broken E2E test)
- Spec alignment: 4 implemented, 4 NOT STARTED, 7 partial/reference/dead
[VERIFIED: [[specs/audits/2026-02-06/audit_frontend_and_specs.md]]]

### 2. Cross-Verifications (2 agents, ~5 min each)

- Runtime verified data-pipeline findings → confirmed 5 concordant bugs, found 0 contradictions
- Data-pipeline verified runtime findings → discovered XCHECK-1 (chain_metadata schema conflict, HIGH severity)
[VERIFIED: [[specs/audits/2026-02-06/cross_check_data_pipeline.md]]]

### 3. Epistemic Grounding

Applied epistemic-context-grounding skill to assess audit confidence:
- Spot-checked 5 file:line citations against source code — all 5 confirmed
- Graded assumptions: 8 STRONG, 1 WEAK (telemetry), 1 NOT DIRECTLY VERIFIED (schema conflict runtime behavior)
- Cross-check concordance: strongest evidence pattern (independent agreement, 0 contradictions)
- Overall confidence: HIGH with 3 caveats (XCHECK-1 runtime behavior, spec alignment depth, telemetry)

### 4. Live UX Testing

Ran actual CLI queries (`tastematter query chains/flex/heat/sessions`) against production DB:
- **LIVE-01 (P0):** Path duplication — same file stored as relative AND absolute path, double-counted in queries
- **LIVE-02 (P0):** Chain names missing — hex IDs only, `generated_name` not surfaced in CLI output
- **LIVE-03 (P1):** Heat dominated by auto-loaded `.claude/skills/*.md` files, not real work files
[VERIFIED: CLI output captured in session, receipt_id ec43e42d for flex query]

### 5. Strategic Fork Analysis

Interrogated the two development forks:

**Fork 1 (Foundation):** Fix path normalization, include files_written in queries, non-destructive chains, schema unification. ~2-4 hours. Prerequisites for everything else.

**Fork 2 (Enrichment):** Context Restoration API (Spec 12). Combines deterministic queries with LLM synthesis. Depends on Fork 1 being correct — corrupted PMI scores from path duplication would poison intelligence layer outputs.

**Strategic recommendation:** Fix Fork 1 items 1-4, then go straight to Fork 2 Phase 1 (deterministic context command, no LLM). Don't expand parser depth yet (parentUuid, progress records) — no current consumer needs it.

### 6. Binary Update

Updated installed binary from v0.1.0-alpha.15-dirty to v0.1.0-alpha.17-dirty (fresh release build from audit's cargo build --release verification). Daemon was running on PID 9100, killed and replaced.

## Local Problem Set

### Completed This Session
- [X] Full 3-agent codebase audit with cross-verification [VERIFIED: [[specs/canonical/14_CODEBASE_AUDIT_2026-02-06.md]]]
- [X] Epistemic grounding assessment [VERIFIED: session transcript]
- [X] Live UX testing against production DB [VERIFIED: CLI receipts]
- [X] Strategic fork analysis (Fork 1 vs Fork 2) [VERIFIED: session transcript]
- [X] Binary updated to latest build [VERIFIED: `tastematter --version` = v0.1.0-alpha.17-dirty]
- [X] Audit reports moved to `specs/audits/2026-02-06/` [VERIFIED: 4 files in directory]

### Jobs To Be Done (Next Session)

**Fork 1 — Foundation Fixes (do first, ~2-4 hours):**

1. [ ] **Path normalization** (LIVE-01, P0) — Canonicalize all file paths to project-relative form during parsing in `jsonl_parser.rs`. Success criteria: `tastematter query flex --files "*audit*"` returns each file exactly once.

2. [ ] **Surface chain names in CLI** (LIVE-02, P0) — Include `chain_metadata.generated_name` in `query_chains` and `query_sessions` JSON output. Fall back to `first_user_message` if no Intel name. Success criteria: `tastematter query chains --format json` shows human-readable names.

3. [ ] **Non-destructive persist_chains** (BUG-07, P0) — Replace `DROP TABLE IF EXISTS` + `CREATE TABLE` with `INSERT OR REPLACE` in `query.rs:1426-1461`. Success criteria: chain queries return consistent data during sync.

4. [ ] **Unify chain_metadata schema** (XCHECK-1, P0) — Merge columns from `storage.rs:207-214` and `cache.rs:403-411` into single CREATE TABLE with all columns. Success criteria: Intel cache writes succeed for all columns.

5. [ ] **Include files_written in queries** (BUG-05/06/10, P1) — Add `UNION` with `json_each(s.files_written)` to `query_flex`, `query_heat`, `query_sessions` and other file-based queries. Success criteria: `tastematter query flex --files "*audit*"` shows files that were written, not just read.

**Fork 2 — Context Restoration Phase 1 (after Fork 1, ~4-6 hours):**

6. [ ] **`tastematter context "<query>"` command** (Spec 12 Phase 1) — Deterministic-only, no LLM. Aggregates flex query + co-access + context package discovery. Success criteria: returns work_clusters, suggested_reads, timeline from existing DB.

## Key Files

| File | Purpose | Status |
|------|---------|--------|
| [[specs/canonical/14_CODEBASE_AUDIT_2026-02-06.md]] | Final synthesis audit report | Written (with LIVE- addendum) |
| [[specs/audits/2026-02-06/audit_data_pipeline.md]] | Agent 1 detailed findings | Written |
| [[specs/audits/2026-02-06/audit_runtime_services.md]] | Agent 2 detailed findings + cross-check §8 | Written |
| [[specs/audits/2026-02-06/audit_frontend_and_specs.md]] | Agent 3 detailed findings | Written |
| [[specs/audits/2026-02-06/cross_check_data_pipeline.md]] | Agent 1 cross-check of Agent 2 | Written |
| [[specs/canonical/07_CLAUDE_CODE_DATA_MODEL_V2.md]] | Ground truth data model | Reference |
| [[specs/canonical/12_CONTEXT_RESTORATION_API_SPEC.md]] | Fork 2 target spec | Reference |
| [[specs/canonical/02_ROADMAP.md]] | 6-phase roadmap | Reference |
| [[core/src/query.rs]] | All query functions (bugs here) | Needs fixes |
| [[core/src/capture/jsonl_parser.rs]] | JSONL parser (path normalization needed) | Needs fixes |
| [[core/src/storage.rs]] | DB schema (chain_metadata conflict) | Needs fixes |
| [[core/src/intelligence/cache.rs]] | Intel cache (competing schema) | Needs fixes |

## Bug & Gap Summary (from audit)

### Bugs: 14 + 4 cross-check + 3 live = 21 total

| Priority | Count | Key Items |
|----------|-------|-----------|
| P0 | 4 | Path duplication, chain names missing, persist_chains destructive, chain_metadata schema conflict |
| P1 | 7 | files_written invisible (3 queries), total_messages undercounts, heat noise, files_created dropped, schema divergence |
| P2 | 10 | Timestamp fallback, Skill invisible, dead data columns, duplicate frontend code, broken E2E test, etc. |

### Gaps: 16 data model + runtime

Most significant: progress records ignored (27K), token usage not extracted, inverted index not persisted, telemetry disabled in daemon, HTTP missing 6 query types.

## Test State

- Rust core: `cargo build --release` passes (verified during audit)
- Rust tests: Not run this session (should run before fixes)
- Frontend tests: 20 test files exist; E2E test broken (expects 90d button)
- Command: `cd core && cargo test` to verify

## For Next Agent

**Context Chain:**
- Previous: [[56_2026-02-06_VERIFICATION_AND_RELEASE_ALPHA17]] (alpha.17 release, heat fix)
- This package: Full codebase audit + strategic fork analysis
- Next action: Fork 1 foundation fixes (path normalization first)

**Start here:**
1. Read this context package
2. Read [[specs/canonical/14_CODEBASE_AUDIT_2026-02-06.md]] §Recommendations for priority-ranked fix list
3. Read [[core/src/capture/jsonl_parser.rs]] to understand path extraction (lines 220-340 for tool path extraction)
4. Run `cd core && cargo test` to baseline test state before making changes

**Strategic decision made this session:**
Fix Fork 1 (4 foundation bugs) THEN go to Fork 2 Phase 1 (deterministic context command). Don't expand parser depth (parentUuid, progress records) until a consumer needs it. The intelligence layer (Fork 2) is where product differentiation lives, but it requires accurate foundation data.

**Do NOT:**
- Expand parser to extract all 14 linking mechanisms (no consumer yet)
- Parse progress/system records (no query or spec needs them yet)
- Build intelligence features before fixing path duplication (would inherit corruption)
- Edit existing audit reports (append-only — write new findings as addenda)

**Key insight:**
The audit found that **correct code can produce bad UX**. The path duplication (LIVE-01) isn't a code bug in the traditional sense — the parser faithfully stores whatever path the tool call contained. But it means every downstream metric (counts, PMI, heat) is corrupted by duplicates. This is the single highest-impact fix.
[VERIFIED: `tastematter query flex --files "*audit*"` returns `_system\temp\code_audit_report.md` (5 accesses) AND `C:\Users\dietl\...\code_audit_report.md` (2 accesses) — same file, double-counted]
