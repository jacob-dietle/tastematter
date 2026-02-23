---
title: "File-Pair Level Edge Extraction Optimization"
type: architecture-spec
created: 2026-02-18
status: draft
foundation:
  - "[[canonical/19_TEMPORAL_EDGES_SPEC]]"
  - "[[core/src/index/file_edges.rs]]"
tags:
  - tastematter
  - temporal-edges
  - performance
  - canonical
---

# File-Pair Level Edge Extraction Optimization

## Executive Summary

Replace the O(events²) per-session edge extraction with O(unique_files²) per-session extraction. The current algorithm compares every event pair; the fix reduces to unique file pairs first, then compares. For a 243K-event session touching ~500 unique files: 250K comparisons instead of 57 billion.

**Root cause:** `extract_session_edges()` (file_edges.rs:400) iterates all read×write pairs and all read×read pairs. This is correct for small sessions but catastrophic for large ones. The aggregation step (`aggregate_edge_candidates`, line 590) only cares about unique (source_file, target_file, edge_type) per session anyway — the event-level granularity is wasted work.

**Empirical data from production DB:**

| Bucket | Sessions | Events | Impact |
|--------|----------|--------|--------|
| 0-100 | 706 | 17,636 | Fine |
| 101-500 | 175 | 41,037 | Fine |
| 501-1000 | 25 | 17,676 | Fine |
| 1001-5000 | 32 | 60,516 | Slow |
| 5000+ | 5 | 294,162 | Hangs (O(n²)) |

The 5 monster sessions (>5K events) account for 68% of all events. One session alone has 243K events (239K reads, 2.5K writes).

---

## Design Principle

**Operate at the right abstraction level.** We care about file-to-file relationships, not event-to-event pairs. The edge "types.rs is read before query.rs is edited" is the same whether it happened once or 47 times in a session. The aggregation already counts sessions, not occurrences.

**From the debugging skill:** "If you're fighting the same error repeatedly, you're solving at the wrong level." The N+1 query fix and the batching fix were at the wrong level. The real issue is algorithmic — O(events²) when the output space is O(unique_files²).

---

## What Changes

**Single function rewrite:** `extract_session_edges()` in `core/src/index/file_edges.rs` (lines 400-512).

**No changes to:**
- `extract_file_edges()` (public API) — unchanged
- `EdgeCandidate` struct — unchanged
- `aggregate_edge_candidates()` — unchanged
- `apply_noise_filters()` — unchanged
- `filter_explore_bursts()` — unchanged (still runs before extraction)
- `extract_reference_anchors()` — unchanged (cross-session SQL, separate path)
- `upsert_edges()` — unchanged
- Batched session loading (lines 120-155) — unchanged

**Blast radius:** 1 function, ~60 lines replaced. All downstream logic identical.

---

## Algorithm: Before vs After

### Before (O(events²))

```
Input: events[] sorted by sequence_position

1. reads = events.filter(read)      // e.g. 239K items
2. writes = events.filter(write)    // e.g. 2.5K items

3. read_then_edit:
   for r in reads:                  // 239K
     for w in writes:               // × 2.5K = 600M iterations
       if w.seq > r.seq && delta < 5min:
         emit candidate(r.file, w.file)

4. read_before:
   for (i, a) in reads.enumerate(): // 239K
     for b in reads[i+1..]:         // × 239K = 57B iterations
       if delta < 5min:
         emit candidate(a.file, b.file)

5. co_edited:
   unique_writes = deduplicate(writes)  // Already O(unique_files²) ✓
   for pair in unique_writes.combinations():
     emit candidate(a, b)
```

### After (O(unique_files²))

```
Input: events[] sorted by sequence_position

1. Reduce to unique files:
   For each unique (file_path, access_type):
     read_files[file] = earliest_timestamp, earliest_seq_position
     write_files[file] = earliest_timestamp, earliest_seq_position

   Typically: ~200-500 unique files (vs 239K events)

2. read_then_edit:
   for r_file in read_files:           // ~400
     for w_file in write_files:        // × ~100 = 40K iterations
       if w_file.seq > r_file.seq && r_file != w_file && delta < 5min:
         emit candidate(r_file, w_file)

3. read_before:
   sorted_reads = read_files.sort_by(seq_position)
   for (i, a) in sorted_reads:        // ~400
     for b in sorted_reads[i+1..]:    // × ~400 = 160K iterations
       if delta < 5min:
         emit candidate(a, b)

4. co_edited:
   [unchanged — already operates on unique files]

5. debug_chain:
   [unchanged — already O(n) linear scan]
```

**Reduction for worst-case session:**
- read_then_edit: 600M → 40K (15,000x faster)
- read_before: 57B → 160K (356,000x faster)

---

## Semantic Equivalence

The current code emits redundant candidates that the aggregation discards:

```
Current: read(types.rs, t=1), read(types.rs, t=5), write(query.rs, t=10)
→ Emits 2 candidates: (types.rs→query.rs, delta=9), (types.rs→query.rs, delta=5)
→ Aggregation: 1 unique edge, avg_delta=7, session_count=1

Proposed: read_files={types.rs: t=1}, write_files={query.rs: t=10}
→ Emits 1 candidate: (types.rs→query.rs, delta=9)
→ Aggregation: 1 unique edge, avg_delta=9, session_count=1
```

**Difference:** Time delta uses earliest occurrence instead of averaging all occurrences. This is arguably more meaningful — "when did you FIRST read this file before editing that one?" The aggregation across sessions still averages, which is the useful signal.

**For session counting (the primary use):** Identical output. A file pair either occurs in a session or it doesn't.

---

## Implementation Spec

### Step 1: Add `FileOccurrence` helper struct

Inside `extract_session_edges`, create a local struct:

```rust
struct FileOccurrence {
    earliest_timestamp: DateTime<Utc>,
    earliest_seq: i32,
}
```

No need to make this public — it's internal to the function.

### Step 2: Build file occurrence maps

Replace the `reads` and `writes` Vec construction (lines 408-418) with:

```rust
let mut read_files: HashMap<&str, FileOccurrence> = HashMap::new();
let mut write_files: HashMap<&str, FileOccurrence> = HashMap::new();

for event in events {
    let map = match event.access_type.as_str() {
        "read" => &mut read_files,
        "write" => &mut write_files,
        _ => continue,
    };
    map.entry(event.file_path.as_str())
        .and_modify(|occ| {
            if event.sequence_position < occ.earliest_seq {
                occ.earliest_timestamp = event.timestamp;
                occ.earliest_seq = event.sequence_position;
            }
        })
        .or_insert(FileOccurrence {
            earliest_timestamp: event.timestamp,
            earliest_seq: event.sequence_position,
        });
}
```

This is O(n) — single pass through events.

### Step 3: Replace read_then_edit loop (lines 420-437)

```rust
for (r_file, r_occ) in &read_files {
    for (w_file, w_occ) in &write_files {
        if w_occ.earliest_seq > r_occ.earliest_seq && *r_file != *w_file {
            let delta = (w_occ.earliest_timestamp - r_occ.earliest_timestamp)
                .num_milliseconds() as f64 / 1000.0;
            if delta >= 0.0 && delta <= MAX_TIME_DELTA_SECONDS {
                candidates.push(EdgeCandidate {
                    source_file: r_file.to_string(),
                    target_file: w_file.to_string(),
                    edge_type: "read_then_edit".to_string(),
                    session_id: session_id.clone(),
                    time_delta_seconds: Some(delta),
                });
            }
        }
    }
}
```

### Step 4: Replace read_before loop (lines 439-456)

```rust
let mut sorted_reads: Vec<(&str, &FileOccurrence)> = read_files.iter()
    .map(|(k, v)| (*k, v))
    .collect();
sorted_reads.sort_by_key(|(_, occ)| occ.earliest_seq);

for (i, (a_file, a_occ)) in sorted_reads.iter().enumerate() {
    for (b_file, b_occ) in sorted_reads.iter().skip(i + 1) {
        if *a_file != *b_file {
            let delta = (b_occ.earliest_timestamp - a_occ.earliest_timestamp)
                .num_milliseconds() as f64 / 1000.0;
            if delta >= 0.0 && delta <= MAX_TIME_DELTA_SECONDS {
                candidates.push(EdgeCandidate {
                    source_file: a_file.to_string(),
                    target_file: b_file.to_string(),
                    edge_type: "read_before".to_string(),
                    session_id: session_id.clone(),
                    time_delta_seconds: Some(delta),
                });
            }
        }
    }
}
```

### Step 5: co_edited (lines 458-483) — SIMPLIFY

The existing co_edited code already deduplicates to unique files. With `write_files` HashMap, simplify to:

```rust
let mut write_list: Vec<&str> = write_files.keys().copied().collect();
write_list.sort();
for (i, a) in write_list.iter().enumerate() {
    for b in write_list.iter().skip(i + 1) {
        candidates.push(EdgeCandidate {
            source_file: a.to_string(),
            target_file: b.to_string(),
            edge_type: "co_edited".to_string(),
            session_id: session_id.clone(),
            time_delta_seconds: None,
        });
    }
}
```

### Step 6: debug_chain (lines 485-508) — UNCHANGED

Keep the existing linear scan. It's already O(n) and only looks at the first read after each Bash call. No change needed.

---

## Test Plan

### Existing tests that MUST still pass (no behavior change expected)

| Test | What it validates | Expected outcome |
|------|-------------------|-----------------|
| `test_extract_session_edges_finds_read_then_edit` | Basic R→W edge | PASS (same) |
| `test_extract_session_edges_finds_read_before` | Basic R→R edge | PASS (same) |
| `test_extract_session_edges_finds_co_edited` | W+W pair edge | PASS (same) |
| `test_extract_session_edges_read_then_edit_respects_time_window` | 5-min cap | PASS (same) |
| `test_extract_session_edges_no_self_read_then_edit` | No self-edges | PASS (same) |
| `test_extract_session_edges_empty_input` | Empty handling | PASS (same) |
| `test_aggregate_edge_candidates_counts_sessions` | Session counting | PASS (same) |
| `test_aggregate_edge_candidates_deduplicates_same_session` | Dedup | PASS (same — but fewer candidates emitted, same result) |
| `test_extract_file_edges_end_to_end` | Full pipeline | PASS (same) |
| `test_extract_file_edges_incremental` | Incremental | PASS (same) |
| `test_universal_anchor_dampening` | Noise filter | PASS (same) |

### New test to add

```rust
#[test]
fn test_extract_session_edges_deduplicates_repeated_file_access() {
    // A session reads types.rs 100 times and writes query.rs 50 times.
    // Should produce exactly 1 read_then_edit candidate, not 5000.
    let events: Vec<FileAccessEvent> = ...;
    let refs: Vec<&FileAccessEvent> = events.iter().collect();
    let candidates = extract_session_edges(&refs);
    let rte: Vec<_> = candidates.iter()
        .filter(|c| c.edge_type == "read_then_edit")
        .collect();
    assert_eq!(rte.len(), 1, "Should deduplicate to 1 candidate per file pair");
}
```

### Performance validation

After implementation, run against real DB:

```bash
tastematter daemon once
```

**Success criteria:**
- Completes in <60 seconds (was: hangs indefinitely)
- Produces >0 `read_then_edit` edges
- Produces >0 `read_before` edges
- `reference_anchor` count unchanged (~1116)
- All 23 existing file_edges tests pass
- `tastematter context "tastematter"` shows `work_pattern` data in clusters

---

## Files Modified

| File | Change | Lines |
|------|--------|-------|
| `core/src/index/file_edges.rs` | Rewrite `extract_session_edges()` | ~60 lines replaced |

**No other files change.** The function signature and return type are identical.

---

## Risk Assessment

- **Blast radius:** 1 function in 1 file
- **New dependencies:** 0
- **New failure modes:** 0
- **Backward compatibility:** Output is semantically equivalent (unique file pairs, session counts)
- **Reversibility:** Full — revert single function if needed

## Verification Sequence

1. Rewrite `extract_session_edges()`
2. Run `cargo test file_edges -- --test-threads=1 -q` → 23/23 pass
3. Run `cargo build --release`
4. Run `tastematter daemon once` → completes <60s, 0 errors
5. Run Python query: `SELECT edge_type, COUNT(*) FROM file_edges GROUP BY edge_type` → non-zero read_then_edit
6. Run `tastematter context "tastematter"` → work_patterns visible
