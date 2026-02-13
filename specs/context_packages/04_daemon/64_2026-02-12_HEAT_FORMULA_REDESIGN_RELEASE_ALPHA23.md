---
title: "Tastematter Context Package 64"
package_number: 64
date: 2026-02-12
status: current
previous_package: "[[63_2026-02-11_PHASE6_E2E_AND_INTEL_SILENT]]"
related:
  - "[[core/src/types.rs]]"
  - "[[core/src/query.rs]]"
  - "[[core/src/context_restore.rs]]"
  - "[[core/src/daemon/sync.rs]]"
  - "[[.claude/skills/context-query/references/heat-metrics-model.md]]"
tags:
  - context-package
  - tastematter
  - heat-formula
  - release
---

# Tastematter - Context Package 64

## Executive Summary

Replaced the broken heat formula with session-spread + exponential decay, fixed two production quick wins, and released as v0.1.0-alpha.23. All staging E2E tests pass (11 jobs green across 3 platforms). Heat was broken in production — all 50 files returned HOT (scores 0.84-1.0) due to RCR always approaching 1.0.

## Global Context

### Architecture: Heat Calculation Pipeline

```
claude_sessions DB
       │
       ▼
  query_heat() ──► SQL: COUNT(DISTINCT session_id) per file
       │                + total_sessions subquery
       ▼
  compute_heat_score(velocity, specificity, recency)
       │   weights: 0.30 / 0.35 / 0.35
       ▼
  classify_heat() ──► initial absolute thresholds (kept for single-item API)
       │
       ▼
  classify_heat_percentile() ──► OVERRIDES with rank-based levels
       │   Top 10%=HOT, 10-30%=WARM, 30-60%=COOL, Bottom 40%=COLD
       ▼
  HeatResult with distributed heat levels
```

### Root Cause Analysis

| Component | Old (Broken) | New (Fixed) |
|-----------|-------------|-------------|
| Signal | RCR = count_7d / count_long | Specificity = 1.0 - (session_spread) |
| Why broken | Active user → RCR ≈ 1.0 always | IDF principle: rare files score higher |
| Recency | Step function: 1.0/0.5/0.0 | Exponential decay: e^(-0.1 * days) |
| Why broken | 1-day and 6-day scored same (0.5) | Smooth gradient: 0.90 vs 0.55 |
| Classification | Absolute thresholds (>0.7 HOT) | Percentile-based (always distributes) |
| Why broken | Floor for any recent file ≈ 0.70 | Rank-based, dataset-independent |

### Key Design Decisions

- Specificity uses IDF-like inversion: `1.0 - (sessions_touching_file / total_sessions)` [VERIFIED: [[core/src/query.rs]]:1115-1118]
- Exponential decay `(-0.1 * days).exp()` chosen for half-life ≈ 7 days [VERIFIED: [[core/src/types.rs]]:930]
- Percentile classification applied AFTER truncation to work on final result set [VERIFIED: [[core/src/query.rs]]:1161-1162]
- `classify_heat()` kept for single-item contexts; `classify_heat_percentile()` overrides in `query_heat()` [VERIFIED: [[core/src/query.rs]]:1122,1162]

## Local Problem Set

### Completed This Session

**Heat Formula Redesign (TDD):**
- [X] Replace RCR with session-spread specificity [VERIFIED: [[core/src/query.rs]]:1053-1128]
- [X] Replace recency step function with exponential decay [VERIFIED: [[core/src/types.rs]]:907-943]
- [X] Update compute_heat_score weights to 0.30/0.35/0.35 [VERIFIED: [[core/src/types.rs]]:945-953]
- [X] Add classify_heat_percentile() function [VERIFIED: [[core/src/types.rs]]:955-988]
- [X] Wire percentile into query_heat() after truncation [VERIFIED: [[core/src/query.rs]]:1161-1162]
- [X] Recompute summary counts AFTER percentile reclassification [VERIFIED: [[core/src/query.rs]]:1164-1181]
- [X] 3 new TDD tests (distribution, exponential decay, percentile levels) [VERIFIED: [[core/src/types.rs]]:1314-1431]
- [X] 20 existing tests updated for new formula [VERIFIED: cargo test types::tests 63 passed]

**Rename RCR → Specificity (all consumers):**
- [X] HeatItem.rcr → .specificity [VERIFIED: [[core/src/types.rs]]:613]
- [X] HeatSortBy::Rcr → ::Specificity [VERIFIED: [[core/src/types.rs]]:586]
- [X] CLI --sort rcr → --sort specificity [VERIFIED: [[core/src/main.rs]]:384,745]
- [X] Table header RCR → SPEC [VERIFIED: [[core/src/main.rs]]:1382]
- [X] CSV header/output updated [VERIFIED: [[core/src/main.rs]]:1522,1530]
- [X] Integration test debug print updated [VERIFIED: [[core/tests/integration_test.rs]]:390-391]

**Quick Wins:**
- [X] Git sync errors → log::info (not user-facing) [VERIFIED: [[core/src/daemon/sync.rs]]:93-95]
- [X] Default aggregations always include count [VERIFIED: [[core/src/query.rs]]:1851-1857]

**Context Integration:**
- [X] Wire heat into executive_summary (hot_file_count, focus_ratio) [VERIFIED: [[core/src/context_restore.rs]]:89-98]
- [X] Add hot_file_count + focus_ratio to ExecutiveSummary struct [VERIFIED: [[core/src/types.rs]]:687-690]

**Reference Doc:**
- [X] Rewrite heat-metrics-model.md for new formula [VERIFIED: [[.claude/skills/context-query/references/heat-metrics-model.md]]]

**Release:**
- [X] Committed as d146032, tagged v0.1.0-alpha.23 [VERIFIED: git log]
- [X] Staging: 11/11 jobs green (build + smoke + E2E on 3 platforms) [VERIFIED: gh run view 21972320537]
- [X] Release: 5/5 jobs green (promote + smoke on 3 platforms + GitHub release) [VERIFIED: gh run view 21972320848]
- [X] CI fmt fix: 3cbdd7d pushed [VERIFIED: git log]

### Jobs To Be Done (Next Session)

1. [ ] **Production validation:** Run `tastematter query heat --time 30d --format json` on real data, verify distribution across 3+ heat levels - Success criteria: NOT all HOT
2. [ ] **Context validation:** Run `tastematter context "tastematter" --format json`, verify `insights` array is non-empty - Success criteria: at least 1 "abandoned" insight detected
3. [ ] **Upgrade local binary:** Install v0.1.0-alpha.23 via staging channel to validate locally
4. [ ] **Backport to dev:** `git checkout dev && git merge master && git push origin dev` - Success criteria: CI passes on dev

## File Locations

| File | Purpose | Status |
|------|---------|--------|
| [[core/src/types.rs]] | Heat functions + struct + tests | Modified (+212/-29) |
| [[core/src/query.rs]] | Heat SQL + aggregations | Modified (+74/-22) |
| [[core/src/main.rs]] | CLI consumers (table, CSV, args) | Modified (+14/-14) |
| [[core/src/context_restore.rs]] | Executive summary + insights | Modified (+15/-4) |
| [[core/src/daemon/sync.rs]] | Git error downgrade | Modified (+8/-8) |
| [[core/tests/integration_test.rs]] | Debug print consumer | Modified (+4/-4) |
| [[.claude/skills/context-query/references/heat-metrics-model.md]] | Reference doc v2.0 | Rewritten |

## Test State

- Unit tests: 63 types + 22 query + 21 context_restore = 106 relevant tests, all passing
- Integration tests: 12/12 passing (including test_query_heat_basic)
- Total: 403+ unit tests passing
- CI: fmt fix pushed, awaiting green
- Staging E2E: 11/11 green (all assertions pass including heat, chains, context)
- Command: `cargo test -- --test-threads=1` from `core/` dir
- Last run: 2026-02-12 21:20 EST
- Evidence: [VERIFIED: test output captured during session]

### Test Commands for Next Agent

```bash
# Verify heat formula tests
cd core && cargo test types::tests -- --test-threads=1

# Verify query + aggregation tests
cd core && cargo test query::tests -- --test-threads=1

# Verify context integration
cd core && cargo test context_restore::tests -- --test-threads=1

# Full compile check
cd core && cargo check

# Production validation (after installing alpha.23)
tastematter query heat --time 30d --format json
tastematter context "tastematter" --format json
```

## For Next Agent

**Context Chain:**
- Previous: [[63_2026-02-11_PHASE6_E2E_AND_INTEL_SILENT]] (stress testing complete)
- This package: Heat formula redesign + release v0.1.0-alpha.23
- Next action: Validate on production data, then decide if further tuning needed

**Start here:**
1. Read this context package
2. Read [[.claude/skills/context-query/references/heat-metrics-model.md]] for formula spec
3. Run `tastematter query heat --time 30d --format json` to validate production differentiation
4. If all HOT still, check `classify_heat_percentile()` in [[core/src/types.rs]]:955-988

**Do NOT:**
- Revert to RCR — it's mathematically degenerate for active users
- Use absolute thresholds alone — they don't adapt to score distributions
- Run `cargo test` without `--test-threads=1` on this machine (OOM risk)

**Key insight:**
The heat problem was two separate issues: (1) RCR had zero discriminative power because count_7d ≈ count_30d for active users, and (2) absolute thresholds don't adapt to score distributions. Fixing only one would not solve the problem — both specificity AND percentile classification were needed.
[VERIFIED: production data analysis showing all 50 files scoring 0.84-1.0 with old formula]
