---
title: "Temporal Edges Quality Refinement — Complete"
package_number: 42
date: 2026-02-19
status: current
previous_package: "[[41_2026-02-17_TEMPORAL_EDGES_FULL_IMPLEMENTATION]]"
related:
  - "[[specs/canonical/20_TEMPORAL_EDGES_QUALITY_REFINEMENT]]"
  - "[[specs/canonical/19_TEMPORAL_EDGES_SPEC]]"
  - "[[core/src/context_restore.rs]]"
  - "[[core/src/index/file_edges.rs]]"
  - "[[core/src/query.rs]]"
  - "[[core/src/types.rs]]"
  - "[[core/src/storage.rs]]"
  - "[[core/src/capture/jsonl_parser.rs]]"
tags:
  - context-package
  - temporal-edges
  - quality-refinement
  - complete
---

# Context Package 42: Temporal Edges Quality Refinement — Complete

**Date:** 2026-02-19
**Status:** Complete — all 3 fixes implemented and verified end-to-end
**Previous:** [[41_2026-02-17_TEMPORAL_EDGES_FULL_IMPLEMENTATION]]

## Executive Summary

Spec #20 (Temporal Edges Quality Refinement) is fully implemented. Three fixes make temporal edges surface in `tastematter context` output as work_patterns. **Before:** 1,614 edges in DB but 0/5 clusters had work_patterns. **After:** 5/5 clusters have work_patterns with entry_points and typical_sequences populated.

## What Changed

### Fix 1: Path Normalization — COMPLETE ✅

**Problem:** Cluster files use relative paths (`apps/tastematter/core/src/types.rs`), edge files use absolute Windows paths (`C:\Users\dietl\...\types.rs`). `build_work_patterns()` does raw string comparison → always returns None.

**Solution:** Reused existing `normalize_file_path()` from `capture/jsonl_parser.rs:208`. Added `project_root: &str` to `build_work_patterns()` and `build_work_clusters()`. Both cluster files and edge paths normalized before comparison.

**Additional fix during verification:** The `project_root` passed was initially CWD (wrong — gave `core/` subdirectory). Needed git repo root detection that matches the root used for `files_read` in `claude_sessions`. Final solution: walk up from CWD collecting `.git` ancestors, then find the candidate root where stripping it from edge paths produces paths matching cluster files. This handles nested git repos (`~/.git`, `gtm_operating_system/.git`, `apps/tastematter/.git`).

**Also fixed:** `topological_sort_edges` was returning raw absolute paths for `typical_sequence`. Replaced with `topological_sort_pairs` that operates on already-normalized (src, tgt) string pairs. Old function removed (dead code).

**Files:**
- `context_restore.rs` — normalize_file_path integration, `topological_sort_pairs` function, 4 new cross-platform tests
- `query.rs` — Git repo root detection, pass project_root to build_work_clusters, query threshold lowered from 3→2

**Tests:** 34/34 passing [VERIFIED: cargo test --lib context_restore --test-threads=1]

### Fix 2: Lift Metric — COMPLETE ✅

**Problem:** `MIN_SESSION_COUNT=3` kills rare-but-meaningful patterns. Need statistical significance.

**Solution:** Added `lift: f64` to `AggregatedEdge`, computed as `(edge_sessions × total_sessions) / (source_sessions × target_sessions)`. Added global session count query, updated schema migration, updated all test literals.

**Files:**
- `file_edges.rs` — lift field, `target_session_counts` HashMap, lift computation, test schema
- `storage.rs` — Migration: `ALTER TABLE file_edges ADD COLUMN lift REAL`
- `types.rs` — `lift: Option<f64>` on `FileEdge`
- `query.rs` — SELECT includes lift column

**Tests:** 26/27 passing (pre-existing incremental failure only) [VERIFIED: cargo test --lib file_edges --test-threads=1]

### Fix 3: Lower Threshold with Lift Guard — COMPLETE ✅

**Problem:** MIN_SESSION_COUNT=3 kills 99.9% of signal (260K→254 edges). Lowering to 2 without guard would let noise through.

**Solution:**
- `MIN_SESSION_COUNT`: 3 → 2
- `MIN_LIFT_THRESHOLD`: new constant = 2.0
- `apply_noise_filters`: session_count=2 requires lift >= 2.0 to survive; session_count>=3 always passes

**Tests added:**
- `test_noise_filter_session_2_high_lift_survives` — session=2, lift=8.0 → passes
- `test_noise_filter_session_2_low_lift_filtered` — session=2, lift=0.5 → filtered
- `test_noise_filter_session_3_survives_regardless_of_lift` — session=3, lift=0.3 → passes
- Updated `test_noise_filter_min_session_count` — session=1 with lift=100 → always filtered

**Tests:** 26/27 passing [VERIFIED: cargo test --lib file_edges --test-threads=1]

## End-to-End Verification

```
$ tastematter context "tastematter" --time 30d --format json
Total clusters: 5
  Core Sync Engine: entry_points: [types.rs, storage.rs, sync.rs]
  Query & Storage: entry_points: [Cargo.toml, types.rs, storage.rs, sync.rs]
  Type System: entry_points: [sync.rs, types.rs, Cargo.toml, storage.rs]
  Session Capture: entry_points: [storage.rs, types.rs, sync.rs]
  Daemon Coordination: entry_points: [storage.rs, sync.rs, types.rs]

Clusters with work_pattern: 5/5
```

All paths in output are relative. No debug output on stderr.

## Pre-existing Issue (Not Fixed)

`test_extract_file_edges_incremental` still fails — stores `Utc::now()` as extraction marker but `get_sessions_since` filters on event `timestamp`. Separate concern from spec #20.

## File State Summary

| File | Changes | Tests |
|------|---------|-------|
| `context_restore.rs` | Fix 1: normalize paths, topological_sort_pairs, 4 tests | 34/34 |
| `file_edges.rs` | Fix 2: lift metric, Fix 3: threshold+guard, 7 changed tests | 26/27 |
| `query.rs` | Git root detection, threshold 3→2, lift in SELECT | Compile-verified |
| `types.rs` | lift: Option<f64> on FileEdge | Compile-verified |
| `storage.rs` | ALTER TABLE migration for lift column | Migration ran on real DB |

## For Next Agent

All spec #20 work is done. Remaining work in temporal edges:

1. **Spec #20 optimization** — `extract_session_edges()` is O(events²) per session. Spec exists at `specs/canonical/20_FILE_PAIR_EXTRACTION_OPTIMIZATION.md`. Large sessions (>500 events) cause hangs. The existing edges were extracted from smaller sessions; full re-extraction needs this optimization.

2. **Incremental test fix** — `test_extract_file_edges_incremental` needs `parsed_at` column or marker logic change. Low priority.

3. **work_targets** — Currently empty because read_then_edit edges are rare (only 7 in DB). Will improve as more sessions are parsed with the optimization.

**Critical: `--test-threads=1` or `--test-threads=2` is required** — parallel tests cause OOM.
