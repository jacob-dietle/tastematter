# Heat Command Specification

**Status:** Proposed
**Priority:** Medium
**Estimated Effort:** 4-6 hours

---

## Overview

Add a `tastematter heat` command that computes usage heat metrics for files, enabling visibility into actual vs declared system usage patterns.

**Problem:** Computing heat metrics currently requires multiple manual queries and mental math. The workflow from the heat-metrics-model reference requires 3-6 CLI invocations per analysis.

**Solution:** A single command that rolls up the underlying query primitives and outputs a heat analysis.

---

## User Stories

1. **As a user**, I want to see which files are actively used vs dormant, so I can identify vestigial components.

2. **As a user**, I want to compare 7-day vs 30-day activity patterns, so I can see what's warming/cooling.

3. **As a user**, I want heat metrics for my whole system or a specific directory, so I can focus my analysis.

---

## Command Interface

```bash
tastematter heat [OPTIONS]

OPTIONS:
  -t, --time <TIME>      Base time window: 30d (default), 60d, 90d
  -f, --files <FILES>    Filter by file pattern (glob-style)
  -l, --limit <LIMIT>    Max results [default: 50]
  --format <FORMAT>      Output: table (default), json, csv
  --sort <SORT>          Sort by: heat (default), rcr, velocity, name
  -h, --help             Print help
```

---

## Output Format

### Table (default)

```
HEAT ANALYSIS (30d window, 7d comparison)
=========================================

FILE                                      7D    30D    RCR    VEL    HEAT
--------------------------------------------------------------------------------
apps/tastematter/src/lib.rs               87    94    0.93   4.70   HOT
_system/state/workstreams.yaml            46    59    0.78   2.00   HOT
.claude/skills/context-query/SKILL.md     82    85    0.96   4.10   HOT
_system/state/pipeline.yaml               26    50    0.52   1.70   WARM
.claude/skills/runway-qb/SKILL.md         17    28    0.61   0.90   WARM
_system/knowledge_graph/taxonomy.yaml      1     4    0.25   0.11   COLD
00_foundation/_synthesis/                  0     1    0.00   0.01   DEAD

SUMMARY:
  HOT:  23 files (46%)
  WARM: 15 files (30%)
  COOL:  8 files (16%)
  COLD:  4 files (8%)
```

### JSON

```json
{
  "receipt_id": "heat_abc123",
  "timestamp": "2026-02-03T17:00:00Z",
  "window": {
    "base": "30d",
    "comparison": "7d"
  },
  "results": [
    {
      "file_path": "apps/tastematter/src/lib.rs",
      "count_7d": 87,
      "count_30d": 94,
      "rcr": 0.93,
      "velocity": 4.70,
      "heat_level": "HOT",
      "heat_score": 0.87
    }
  ],
  "summary": {
    "hot": 23,
    "warm": 15,
    "cool": 8,
    "cold": 4,
    "total": 50
  }
}
```

---

## Metric Calculations

### Recency Concentration Ratio (RCR)

```rust
rcr = count_7d / count_30d
// Handle division by zero: if count_30d == 0, rcr = 0.0
```

### Velocity

```rust
// Requires first_access tracking (new field)
days_active = (now - first_access).days()
velocity = count_30d / days_active

// If first_access not available, approximate from oldest access in window
```

### Heat Level Classification

```rust
fn classify_heat(rcr: f64, velocity: f64) -> HeatLevel {
    if rcr > 0.7 && velocity > 1.0 {
        HeatLevel::Hot
    } else if rcr > 0.4 || velocity > 0.5 {
        HeatLevel::Warm
    } else if rcr > 0.1 && velocity > 0.05 {
        HeatLevel::Cool
    } else {
        HeatLevel::Cold
    }
}
```

### Heat Score (for sorting)

```rust
fn heat_score(rcr: f64, velocity: f64, last_access: DateTime) -> f64 {
    let normalized_velocity = (velocity / 5.0).min(1.0);
    let recency_bonus = if last_access > now - 24h { 1.0 }
                        else if last_access > now - 7d { 0.5 }
                        else { 0.0 };

    (normalized_velocity * 0.3) + (rcr * 0.5) + (recency_bonus * 0.2)
}
```

---

## Implementation Approach

### Option A: Build on existing flex query (Recommended)

Reuse the existing `query flex` infrastructure:

```rust
// Pseudocode
fn heat_command(opts: HeatOpts) -> Result<HeatReport> {
    // Run two flex queries internally
    let results_short = query_flex(time: "7d", files: opts.files, agg: "count,recency,sessions");
    let results_long = query_flex(time: opts.time, files: opts.files, agg: "count,recency,sessions");

    // Join results by file_path
    let joined = join_by_path(results_short, results_long);

    // Compute derived metrics
    let heat_results: Vec<HeatResult> = joined.iter().map(|(short, long)| {
        let rcr = short.count as f64 / long.count as f64;
        let velocity = estimate_velocity(long);
        let heat_level = classify_heat(rcr, velocity);
        HeatResult { file_path, rcr, velocity, heat_level, ... }
    }).collect();

    // Sort and limit
    heat_results.sort_by(|a, b| b.heat_score.cmp(&a.heat_score));
    heat_results.truncate(opts.limit);

    Ok(HeatReport { results: heat_results, summary: compute_summary(&heat_results) })
}
```

### Option B: Direct database query

Single SQL query that computes everything:

```sql
WITH
  short_window AS (
    SELECT file_path, COUNT(*) as count_7d
    FROM file_events
    WHERE timestamp > datetime('now', '-7 days')
    GROUP BY file_path
  ),
  long_window AS (
    SELECT file_path, COUNT(*) as count_30d, MIN(timestamp) as first_access
    FROM file_events
    WHERE timestamp > datetime('now', '-30 days')
    GROUP BY file_path
  )
SELECT
  l.file_path,
  COALESCE(s.count_7d, 0) as count_7d,
  l.count_30d,
  CAST(COALESCE(s.count_7d, 0) AS REAL) / l.count_30d as rcr,
  l.count_30d / (julianday('now') - julianday(l.first_access)) as velocity
FROM long_window l
LEFT JOIN short_window s ON l.file_path = s.file_path
ORDER BY rcr DESC
LIMIT ?
```

**Recommendation:** Option A - maintains code reuse and consistency with existing patterns.

---

## Database Enhancement (Optional)

For accurate velocity calculation, consider adding `first_access` tracking:

```sql
-- New column or separate table
ALTER TABLE file_events ADD COLUMN first_access TIMESTAMP;

-- Or: Materialized view
CREATE VIEW file_stats AS
SELECT
  file_path,
  MIN(timestamp) as first_access,
  MAX(timestamp) as last_access,
  COUNT(*) as total_accesses
FROM file_events
GROUP BY file_path;
```

Without this, velocity is approximated from the oldest access in the query window.

---

## Testing

```rust
#[test]
fn test_rcr_calculation() {
    assert_eq!(calculate_rcr(10, 10), 1.0);  // All recent
    assert_eq!(calculate_rcr(5, 10), 0.5);   // Half recent
    assert_eq!(calculate_rcr(0, 10), 0.0);   // None recent
    assert_eq!(calculate_rcr(0, 0), 0.0);    // Edge case
}

#[test]
fn test_heat_classification() {
    assert_eq!(classify_heat(0.95, 5.0), HeatLevel::Hot);
    assert_eq!(classify_heat(0.60, 0.8), HeatLevel::Warm);
    assert_eq!(classify_heat(0.15, 0.08), HeatLevel::Cool);
    assert_eq!(classify_heat(0.05, 0.02), HeatLevel::Cold);
}

#[test]
fn test_heat_command_integration() {
    // Run heat command on test database
    // Verify output format
    // Verify sorting works
}
```

---

## Rollout

1. **Phase 1:** Implement basic command with RCR (no velocity)
2. **Phase 2:** Add velocity calculation (with approximation)
3. **Phase 3:** Add first_access tracking for accurate velocity
4. **Phase 4:** Add `--drift` flag to compare against CLAUDE.md declarations

---

## Future Extensions

### `tastematter heat --drift`

Compare heat analysis against declared usage in CLAUDE.md:

```
DRIFT ANALYSIS
==============

VESTIGIAL (declared important, actually cold):
  - 00_foundation/_synthesis/ - "read first" → DEAD
  - _system/knowledge_graph/taxonomy.yaml - "config" → COLD (expected)

UNDOCUMENTED HOT (heavily used, not documented):
  - apps/tastematter/intel/ - HOT, no CLAUDE.md mention

ALIGNED:
  - _system/state/workstreams.yaml - "ops hub" → HOT ✓
  - 04_knowledge_base/ - "cold storage" → COLD ✓
```

This requires parsing CLAUDE.md for declared file purposes, which is a separate feature.

---

**Created:** 2026-02-03
**Author:** System meta-review analysis
