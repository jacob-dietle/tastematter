---
title: "Temporal Edges Quality Refinement"
type: architecture-spec
created: 2026-02-18
last_updated: 2026-02-18
status: draft
foundation:
  - "[[canonical/19_TEMPORAL_EDGES_SPEC]]"
  - "[[context_packages/03_current/41_2026-02-17_TEMPORAL_EDGES_FULL_IMPLEMENTATION]]"
related:
  - "[[core/src/index/file_edges.rs]]"
  - "[[core/src/context_restore.rs]]"
  - "[[core/src/query.rs]]"
  - "[[core/src/storage.rs]]"
tags:
  - tastematter
  - temporal-edges
  - quality
  - canonical
---

# Temporal Edges Quality Refinement

## Executive Summary

Temporal edges (spec #19) are implemented and producing 1,612 edges from 16K
real tool-call events. But zero `work_pattern` fields appear in `context`
output. Three bugs prevent edges from surfacing:

1. **Path format mismatch** — Cluster files use relative paths (`apps/tastematter/core/src/types.rs`), edge files use absolute paths (`C:\Users\dietl\...\types.rs`). They never match in `build_work_patterns`.
2. **No lift metric** — `MIN_SESSION_COUNT=3` is a blunt filter. Rare-but-meaningful patterns (session_count=2, but both files only appear in 3 sessions total) get killed alongside noise.
3. **Threshold too aggressive** — 99.9% of candidates die at >=3 sessions. Lowering to >=2 with a lift guard captures the long tail without flooding with noise.

**Measured data (from diagnostic):**

| Threshold | Surviving Edges | % of Candidates |
|-----------|-----------------|-----------------|
| >= 1      | 260,145         | 100%            |
| >= 2      | 1,927           | 0.7%            |
| >= 3      | 254             | 0.1%            |
| >= 5      | 49              | 0.02%           |

This is the "nines problem" — each increment kills signal exponentially.

---

## Fix 1: Path Normalization

### Problem

`build_work_patterns()` in `context_restore.rs:799` filters edges by checking
if `source_file` and `target_file` are in the cluster's file set:

```rust
let file_set: HashSet<&str> = cluster_files.iter().map(|s| s.as_str()).collect();
// ...
.filter(|e| file_set.contains(e.source_file.as_str())
          && file_set.contains(e.target_file.as_str()))
```

Cluster files come from the flex query (via `claude_sessions.files_read` JSON),
which stores **relative paths**:
```
apps/tastematter/core/src/types.rs
.claude/skills/runway-ops/SKILL.md
```

Edge files come from `file_access_events`, which stores paths from `ToolUse`
structs. These are **mixed format** — some relative, some absolute with
backslashes:
```
C:\Users\dietl\VSCode Projects\...\apps\tastematter\core\src\types.rs
apps\tastematter\core\src\types.rs
/c/Users/dietl/VSCode Projects/.../types.rs
```

**Result:** `file_set.contains(e.source_file.as_str())` always returns false.

### Fix

Normalize both sides to the same format. The cheapest approach: strip the
project root prefix from absolute edge paths to produce relative paths, and
normalize path separators.

**Location:** `query_file_edges()` in `query.rs` — normalize edge paths
at query time (not storage time, preserving raw data).

**Normalization rules:**
1. Replace `\` with `/` (Windows → Unix separators)
2. If path starts with the project root, strip it to get relative path
3. Strip leading `/` or `./`

**Project root detection:** `query_context()` already calls
`std::env::current_dir()` at line 1420. Pass the CWD to `query_file_edges`
as the project root.

Alternatively, normalize in `build_work_patterns()` — normalize both the
cluster file set AND the edge file paths before comparison. This is simpler
and doesn't require changing the `query_file_edges` signature.

**Chosen approach:** Normalize in `build_work_patterns()` — add a helper
function `normalize_path(path: &str) -> String` that both sides pass through.

### Schema Change

None. Raw paths preserved in DB. Normalization is query-time only.

### Files Changed

| File | Change | Lines |
|------|--------|-------|
| `context_restore.rs` | Add `normalize_path()` helper, apply to both file_set and edge paths in `build_work_patterns()` | ~15 |

### Tests (TDD — write FIRST)

```rust
#[test]
fn test_normalize_path_strips_windows_prefix() {
    let p = r"C:\Users\dietl\VSCode Projects\taste_systems\gtm_operating_system\apps\tastematter\core\src\types.rs";
    let normalized = normalize_path(p);
    assert_eq!(normalized, "apps/tastematter/core/src/types.rs");
}

#[test]
fn test_normalize_path_handles_forward_slash_absolute() {
    let p = "/c/Users/dietl/VSCode Projects/taste_systems/gtm_operating_system/apps/foo.rs";
    let normalized = normalize_path(p);
    assert_eq!(normalized, "apps/foo.rs");
}

#[test]
fn test_normalize_path_preserves_relative() {
    let p = "apps/tastematter/core/src/types.rs";
    let normalized = normalize_path(p);
    assert_eq!(normalized, "apps/tastematter/core/src/types.rs");
}

#[test]
fn test_normalize_path_backslash_relative() {
    let p = r"apps\tastematter\core\src\types.rs";
    let normalized = normalize_path(p);
    assert_eq!(normalized, "apps/tastematter/core/src/types.rs");
}

#[test]
fn test_build_work_patterns_with_mixed_path_formats() {
    // Cluster files are relative (from flex query)
    let cluster = vec![
        "apps/tastematter/core/src/types.rs".to_string(),
        "apps/tastematter/core/src/query.rs".to_string(),
    ];
    // Edges have absolute paths (from file_access_events)
    let edges = vec![
        make_edge(
            r"C:\Users\dietl\VSCode Projects\taste_systems\gtm_operating_system\apps\tastematter\core\src\types.rs",
            r"C:\Users\dietl\VSCode Projects\taste_systems\gtm_operating_system\apps\tastematter\core\src\query.rs",
            "read_before", 5, 0.6,
        ),
    ];
    let pattern = build_work_patterns(&cluster, &edges);
    assert!(pattern.is_some(), "Should match after normalization");
    let p = pattern.unwrap();
    assert!(!p.entry_points.is_empty(), "Should find entry points");
}
```

---

## Fix 2: Add Lift Metric

### Problem

`MIN_SESSION_COUNT=3` treats all edges equally. An edge appearing in 3/1526
sessions is noise. An edge appearing in 2/3 sessions where both files only
appear in 4 sessions total is highly significant.

**Lift formula:**

```
lift = P(A→B) / (P(A) × P(B))
     = (sessions_with_edge / total_sessions) / ((sessions_with_source / total_sessions) × (sessions_with_target / total_sessions))

Simplified:
lift = (sessions_with_edge × total_sessions) / (sessions_with_source × sessions_with_target)
```

| Edge | session_count | source_sessions | target_sessions | total | lift |
|------|--------------|-----------------|-----------------|-------|------|
| A→B  | 3            | 500             | 400             | 1000  | 0.015 (noise) |
| C→D  | 2            | 4               | 3               | 1000  | 166.7 (signal) |

### Fix

**Step 1:** Add `lift` column to `file_edges` table (schema migration).

**Step 2:** Compute lift in `aggregate_edge_candidates()` — requires knowing
total session count and per-file session counts.

**Step 3:** Compute `total_sessions_with_target` (currently only source is
tracked). Add to aggregation.

### Schema Change

Add `lift REAL` column to `file_edges`. Use `ALTER TABLE ADD COLUMN` in
`ensure_schema()` — safe for SQLite (column added with NULL default, existing
rows get NULL until next extraction).

```sql
ALTER TABLE file_edges ADD COLUMN lift REAL;
```

### Files Changed

| File | Change | Lines |
|------|--------|-------|
| `storage.rs` | Add `lift REAL` column to schema, add migration in `ensure_schema()` | ~5 |
| `file_edges.rs` | Compute lift in `aggregate_edge_candidates()`, add to `AggregatedEdge` struct, pass total_sessions | ~15 |
| `file_edges.rs` | Store lift in `upsert_edges()` | ~3 |
| `types.rs` | Add `lift: Option<f64>` to `FileEdge` struct | ~2 |

### Tests (TDD — write FIRST)

```rust
#[test]
fn test_lift_calculation_high_significance() {
    // Edge in 2 sessions, source in 3 sessions, target in 4 sessions, 100 total
    // lift = (2 × 100) / (3 × 4) = 16.67
    let candidates = vec![
        make_candidate("a.rs", "b.rs", "read_before", "s1"),
        make_candidate("a.rs", "b.rs", "read_before", "s2"),
        make_candidate("a.rs", "c.rs", "read_before", "s3"),  // source appears in 3 sessions
        make_candidate("x.rs", "b.rs", "read_before", "s2"),  // target appears with other sources
        make_candidate("x.rs", "b.rs", "read_before", "s4"),  // target in 4 sessions total
        make_candidate("x.rs", "b.rs", "read_before", "s5"),
        make_candidate("x.rs", "b.rs", "read_before", "s6"),
    ];
    let edges = aggregate_edge_candidates(&candidates, 100);
    let ab = edges.iter().find(|e| e.source_file == "a.rs" && e.target_file == "b.rs").unwrap();
    assert!(ab.lift > 10.0, "High lift for rare-but-meaningful pair");
}

#[test]
fn test_lift_calculation_low_significance() {
    // Both files appear in many sessions — coincidental overlap
    // Edge in 5 sessions, source in 200 sessions, target in 300 sessions, 1000 total
    // lift = (5 × 1000) / (200 × 300) = 0.083
    // This edge is noise despite session_count=5
    let mut candidates = Vec::new();
    for i in 0..5 {
        candidates.push(make_candidate("common.rs", "popular.rs", "read_before", &format!("s{i}")));
    }
    // Source appears in 200 sessions (simulate by adding other targets)
    for i in 5..205 {
        candidates.push(make_candidate("common.rs", &format!("other{i}.rs"), "read_before", &format!("s{i}")));
    }
    // Target appears in 300 sessions (simulate by adding other sources)
    for i in 205..505 {
        candidates.push(make_candidate(&format!("src{i}.rs"), "popular.rs", "read_before", &format!("s{i}")));
    }
    let edges = aggregate_edge_candidates(&candidates, 1000);
    let cp = edges.iter().find(|e| e.source_file == "common.rs" && e.target_file == "popular.rs").unwrap();
    assert!(cp.lift < 1.0, "Low lift for coincidental pair: {}", cp.lift);
}

#[test]
fn test_schema_has_lift_column() {
    let db = setup_test_db().await;
    let cols: Vec<(String,)> = sqlx::query_as("PRAGMA table_info(file_edges)")
        .fetch_all(db.pool()).await.unwrap();
    let names: Vec<&str> = cols.iter().map(|c| c.0.as_str()).collect();
    assert!(names.contains(&"lift"), "file_edges should have lift column");
}
```

---

## Fix 3: Lower Threshold with Lift Guard

### Problem

`MIN_SESSION_COUNT=3` kills 99.9% of candidates. The long tail (session_count
=2) contains meaningful patterns that are indistinguishable from noise without
lift.

### Fix

Change `apply_noise_filters()`:

**Before:**
```rust
if e.session_count < MIN_SESSION_COUNT {  // MIN_SESSION_COUNT = 3
    return false;
}
```

**After:**
```rust
if e.session_count < 2 {
    return false;  // Absolute minimum: must appear in 2+ sessions
}
if e.session_count < 3 && e.lift < MIN_LIFT_THRESHOLD {
    return false;  // session_count=2 requires high lift
}
```

Where `MIN_LIFT_THRESHOLD = 2.0` (edge appears 2x more often than chance).

**Expected impact:**
- Current: 254 edges survive at >=3
- New: ~1,000+ edges survive at >=2 with lift>2.0
- Quality: noise edges (low lift) still killed

### Files Changed

| File | Change | Lines |
|------|--------|-------|
| `file_edges.rs` | Change `MIN_SESSION_COUNT` to 2, add `MIN_LIFT_THRESHOLD = 2.0`, update `apply_noise_filters` | ~5 |

### Tests (TDD — write FIRST)

```rust
#[test]
fn test_threshold_session_2_high_lift_survives() {
    let edge = AggregatedEdge {
        session_count: 2,
        lift: 8.0,  // Highly significant
        confidence: 0.5,
        ..default_edge()
    };
    let result = apply_noise_filters(&[edge], &HashSet::new());
    assert_eq!(result.len(), 1, "session_count=2 with high lift should survive");
}

#[test]
fn test_threshold_session_2_low_lift_filtered() {
    let edge = AggregatedEdge {
        session_count: 2,
        lift: 0.5,  // Coincidental
        confidence: 0.5,
        ..default_edge()
    };
    let result = apply_noise_filters(&[edge], &HashSet::new());
    assert_eq!(result.len(), 0, "session_count=2 with low lift should be filtered");
}

#[test]
fn test_threshold_session_3_survives_regardless_of_lift() {
    let edge = AggregatedEdge {
        session_count: 3,
        lift: 0.3,  // Low lift but enough sessions
        confidence: 0.5,
        ..default_edge()
    };
    let result = apply_noise_filters(&[edge], &HashSet::new());
    assert_eq!(result.len(), 1, "session_count>=3 always survives");
}

#[test]
fn test_threshold_session_1_always_filtered() {
    let edge = AggregatedEdge {
        session_count: 1,
        lift: 100.0,  // Even high lift can't save session_count=1
        confidence: 1.0,
        ..default_edge()
    };
    let result = apply_noise_filters(&[edge], &HashSet::new());
    assert_eq!(result.len(), 0, "session_count=1 always filtered");
}
```

---

## Implementation Order

```
Fix 1 (path normalization) ←── unlocks work_pattern visibility
        │
Fix 2 (lift metric)        ←── enables quality-aware filtering
        │
Fix 3 (threshold + lift)   ←── unlocks the long tail
```

Fix 1 is independent. Fix 3 depends on Fix 2 (needs lift column).

**Estimated time:**
- Fix 1: 30 min (write tests, add normalize_path, update build_work_patterns)
- Fix 2: 45 min (schema migration, aggregate change, upsert change)
- Fix 3: 15 min (constant change, filter logic)
- Verification: 15 min (rebuild, re-extract, check context output)
- **Total: ~2 hours**

## Verification Plan

After all 3 fixes:

```bash
# 1. Build
cd apps/tastematter/core && cargo build --release

# 2. Run targeted tests (not full suite)
cargo test context_restore::tests -- --test-threads=1 -q
cargo test file_edges -- --test-threads=1 -q

# 3. Re-extract edges
tastematter daemon once

# 4. Verify work_patterns appear
tastematter context "tastematter" --time 30d --format json | \
  python -c "import sys,json; d=json.load(sys.stdin); \
  [print(f'{c[\"name\"]}: {c.get(\"work_pattern\",{}).get(\"entry_points\",[])}') \
   for c in d['work_clusters']]"
```

**Success criteria:**
- At least 1 work_cluster has a non-null `work_pattern`
- `work_pattern.entry_points` contains recognizable entry files
- `work_pattern.typical_sequence` shows a meaningful reading order
- Total edges > 500 (up from current 254 at session_count>=3)

## Risk Assessment (Debugging Skill)

| Check | Answer |
|-------|--------|
| New files created | 0 |
| New dependencies | 0 |
| New failure modes | 0 (lift=NULL for old edges is harmless) |
| Lines changed | ~45 total across 4 files |
| Cognitive load | 3/10 (path normalization, one new column, one constant change) |

**Senior engineer test:** "Why didn't you just normalize the paths?"
Answer: "That's exactly what Fix 1 does."
