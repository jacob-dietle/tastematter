---
title: "Tastematter Context Package 61"
package_number: 61
date: 2026-02-11
status: current
previous_package: "[[60_2026-02-07_CWD_FIX_HEAT_FIX_RELEASE_ALPHA19]]"
related:
  - "[[specs/implementation/e2e_user_experience_pipeline/00_ARCHITECTURE_GUIDE.md]]"
  - "[[specs/implementation/stress_testing/00_ARCHITECTURE_GUIDE.md]]"
  - "[[core/src/capture/jsonl_parser.rs]]"
  - "[[.github/workflows/staging.yml]]"
tags:
  - context-package
  - tastematter
  - e2e-testing
  - bugfix
  - ci
---

# Tastematter - Context Package 61

## Executive Summary

Shipped E2E user experience pipeline + UTF-8 parser fix. The E2E pipeline installs Claude Code, generates real sessions, installs tastematter from staging, runs the full workflow, and asserts results — on all 3 platforms in CI. Parser fix resolves the `is_char_boundary` panic Victor discovered. All 11 CI jobs green. Then specced out a 6-phase stress testing architecture (~88 net-new tests) targeting the critically undertested storage and query modules.

## Session Activity

### 1. UTF-8 Parser Fix (jsonl_parser.rs)

**Bug:** `excerpt.truncate(MAX_EXCERPT_CHARS)` at line 669 panics when the truncation point falls mid-character in a multi-byte UTF-8 sequence (emoji, CJK, Cyrillic). Discovered by Victor during live testing — binary installed fine, daemon ran, but crashed on real session data.

**Fix 1 — Safe truncation (line 668-677):**
```rust
// Walk backward to find valid char boundary
let mut truncate_at = MAX_EXCERPT_CHARS;
while truncate_at > 0 && !excerpt.is_char_boundary(truncate_at) {
    truncate_at -= 1;
}
excerpt.truncate(truncate_at);
```
[VERIFIED: [[core/src/capture/jsonl_parser.rs]]:668-677]

**Fix 2 — Panic recovery in sync_sessions (line 925-939):**
Wrapped `aggregate_session()` in `std::panic::catch_unwind` so one bad session doesn't crash the entire daemon batch. Logs error and skips session.
[VERIFIED: [[core/src/capture/jsonl_parser.rs]]:925-939]

**4 new regression tests:**
- `test_truncate_multibyte_utf8_does_not_panic` — 4-byte emoji at exact boundary
- `test_truncate_with_cjk_characters` — 3-byte CJK
- `test_truncate_with_cyrillic_near_boundary` — 2-byte Cyrillic
- `test_aggregate_session_with_mixed_unicode_messages` — multi-message mixed
[VERIFIED: all 69 jsonl_parser tests pass, `cargo test -- --test-threads=2`]

### 2. E2E Pipeline (staging.yml)

**New `e2e-test` job** added to staging workflow, runs after `upload-staging`:

| Stage | What | Platform |
|-------|------|----------|
| 1 | Setup Node.js 20 | All |
| 2 | `npm install -g @anthropic-ai/claude-code` | All |
| 3 | Generate 4 real sessions via `claude -p` (Haiku) | All |
| 4 | Install tastematter from staging channel | Win=pwsh, Unix=bash |
| 5 | Run daemon once + all query commands | All |
| 6 | Hard assertions: no panics, result_count > 0, receipt_id | All |
| 7 | Agent quality eval (Haiku rates 1-10, non-blocking) | All |
| 8 | Upload all artifacts | All |

**Matrix:** windows-latest, ubuntu-latest, macos-latest
**Timeout:** 25 minutes
**Requires:** `ANTHROPIC_API_KEY` GitHub secret (added)
[VERIFIED: [[.github/workflows/staging.yml]]:228-447]

### 3. CI Run Results

**First run (commit 89cec71):** E2E failed — `ANTHROPIC_API_KEY` secret was empty (not yet added to repo). Claude Code returned "Not logged in" for all 4 sessions. Daemon still parsed them without panics (parser fix confirmed). Query returned 0 results because sessions had no real content.

**Rerun (after secret added):** All 11 jobs green.

| Platform | Daemon | Sessions | Query Results | Context | Eval |
|----------|--------|----------|---------------|---------|------|
| Windows | 4 parsed, 0 panics | 4 files | 3 results | receipt_id ok | 6/10 |
| Ubuntu | 4 parsed, 0 panics | 4 files | 3 results | receipt_id ok | 6/10 |
| macOS | 4 parsed, 0 panics | 4 files | 3 results | receipt_id ok | 6/10 |

[VERIFIED: `gh run view 21888031396` — all jobs conclusion=success]

### 4. Agent Quality Eval: Why 6/10

The eval agent sees two error messages in daemon output on every run:
1. `"Git sync error: fatal: not a git repository"` — expected in CI (no git repo) but alarming to first-time user
2. `"Intel: Service unavailable - skipping enrichment"` — Intel service not running in CI

Also: only 3/4 sessions produce query results (one too thin to index). These are real UX issues, not just CI artifacts — a first-time user on a machine without nearby git repos would see the same errors.

### 5. Stress Testing Spec

Audited test density across all modules:

| Module | Lines | Tests | Per 100L | Risk |
|--------|-------|-------|----------|------|
| storage.rs | 922 | 2 | 0.2 | **CRITICAL** |
| query.rs | 2181 | 5 | 0.2 | **CRITICAL** |
| sync.rs | 1190 | 8 | 0.7 | HIGH |
| context_restore.rs | 1120 | 12 | 1.1 | MEDIUM |
| chain_graph.rs | 1172 | 30 | 2.6 | LOW |
| jsonl_parser.rs | 1994 | 69 | 3.5 | LOW |

Wrote 6-phase architecture guide: `[[specs/implementation/stress_testing/00_ARCHITECTURE_GUIDE.md]]`

| Phase | Target | Tests |
|-------|--------|-------|
| 1. Storage Hardening | storage.rs | 15 |
| 2. Query Engine Adversarial | query.rs | 20 |
| 3. Sync Orchestration | daemon/sync.rs | 15 |
| 4. Context Restore Edge Cases | context_restore.rs | 12 |
| 5. Input Resilience | Cross-cutting | 18 |
| 6. E2E Pipeline Enhancement | staging.yml | 8 |

**Key insight:** Parser was overrepresented in testing (3.5/100L). Storage and query at 0.2/100L are 17x more undertested and are the foundation everything else depends on. SQL injection via `query_context` project names is an untested attack vector. Sync idempotency (run twice, same count) is how real users break things.

## Current State

- **Test count:** 292 existing + 4 new = 296 across 23 files
- **CI pipeline:** staging.yml has build (4 targets) + upload + smoke-test (3 platforms) + e2e-test (3 platforms) = 11 jobs
- **Last commit:** `89cec71` on master — "feat: E2E user experience pipeline + UTF-8 parser fix"
- **Latest release:** v0.1.0-alpha.21 (this commit not yet tagged)

## Jobs To Be Done (Next Session)

### Implement Stress Testing Phases 1-5

1. [ ] **Phase 1: Storage Hardening** (15 tests) — Priority: CRITICAL
   - Idempotent upserts, concurrent access, schema migration, large data
   - Success criteria: storage.rs goes from 2 → 17 tests

2. [ ] **Phase 2: Query Engine Adversarial** (20 tests) — Priority: CRITICAL
   - SQL injection, time parsing edge cases, empty results, performance
   - Success criteria: query.rs goes from 5 → 25 tests

3. [ ] **Phase 3: Sync Orchestration** (15 tests) — Priority: HIGH
   - Idempotency (run twice = same count), empty .claude, concurrent daemons
   - Success criteria: sync.rs goes from 8 → 23 tests

4. [ ] **Phase 4: Context Restore Edge Cases** (12 tests) — Priority: MEDIUM
   - Empty DB, stale sessions, Intel merge mismatches, unicode paths
   - Success criteria: context_restore.rs goes from 12 → 24 tests

5. [ ] **Phase 5: Input Resilience** (18 tests) — Priority: HIGH
   - BOM, CRLF, null bytes, >10MB lines, content-as-array, locked files
   - Cross-cutting across jsonl_parser, sync, chain_graph, config
   - Success criteria: all inputs have adversarial coverage

6. [ ] **Phase 6: E2E Pipeline Enhancement** (8 scenarios) — Priority: MEDIUM
   - Emoji sessions, idempotency check, DB recovery, performance budget
   - Success criteria: eval rating improves from 6/10 to 7+/10

### Fix Agent Quality Eval Pipeline

The eval step outputs "Not logged in" when the API key isn't passed correctly through `$(cat ...)` command substitution. Need to investigate whether the eval's `claude -p` receives the key correctly. Low priority since eval is non-blocking.

## File Locations

| File | Purpose | Status |
|------|---------|--------|
| [[core/src/capture/jsonl_parser.rs]] | Parser: UTF-8 fix + panic recovery + 4 tests | Committed |
| [[.github/workflows/staging.yml]] | E2E pipeline: 8-stage job on 3 platforms | Committed |
| [[specs/implementation/stress_testing/00_ARCHITECTURE_GUIDE.md]] | 6-phase stress test architecture | Written, uncommitted in GTM repo |
| [[specs/implementation/e2e_user_experience_pipeline/00_ARCHITECTURE_GUIDE.md]] | Original E2E spec | Reference |

## Test Commands for Next Agent

```bash
# Verify parser tests pass
cd apps/tastematter && cargo test --manifest-path core/Cargo.toml -- jsonl_parser::tests --test-threads=2

# Run all tests
cargo test --manifest-path core/Cargo.toml -- --test-threads=2

# Trigger E2E pipeline (push to master)
cd apps/tastematter && git push origin master

# Check latest CI run
cd apps/tastematter && gh run list --limit 3
```

## For Next Agent

**Context Chain:**
- Previous: [[60_2026-02-07_CWD_FIX_HEAT_FIX_RELEASE_ALPHA19]] (cwd fix, heat fix, alpha.19)
- This package: E2E pipeline + parser fix + stress testing spec
- Next action: Implement stress testing phases 1-5

**Start here:**
1. Read this context package
2. Read [[specs/implementation/stress_testing/00_ARCHITECTURE_GUIDE.md]] for full test matrices
3. Start with Phase 1 (storage) or Phase 2 (query) — highest ROI
4. Run `cargo test -- --test-threads=2` after each phase
5. Phases 1-5 are independent — can parallelize with teams

**Do NOT:**
- Skip `--test-threads=2` (will OOM and crash VS Code)
- Add more jsonl_parser tests without addressing storage/query first
- Modify existing tests (append-only test additions)
- Tag a release until stress tests are in place

**Key insight:**
The parser was already well-tested (3.5/100L). The real risk is storage (0.2/100L) and query (0.2/100L) — the foundation and the user-facing API are 17x less tested than the parser. SQL injection via project names in `query_context` is completely untested. [INFERRED: test density audit across all 23 test files]
