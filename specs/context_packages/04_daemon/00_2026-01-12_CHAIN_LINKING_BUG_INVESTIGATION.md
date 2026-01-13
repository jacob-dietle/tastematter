---
title: "Tastematter Chain Linking Bug Investigation"
package_number: 0

migrated_from: "apps/context-os/specs/event_capture/context_packages/00_2026-01-12_CHAIN_LINKING_BUG_INVESTIGATION.md"
status: current
previous_package: null
related:
  - "[[08_CHAIN_LINKING_CANONICAL_REFERENCE.md]]"
  - "[[apps/context-os/core/src/]]"
tags:
  - context-package
  - tastematter
  - chain-linking
  - bug-investigation
---

# Tastematter Chain Linking Bug Investigation

## Executive Summary

**BUG DISCOVERED:** The tastematter CLI's chain linking is fundamentally broken. All sessions are being grouped into a single chain (`fa6b4bf6`) with identical timestamps and `file_count: 0`. The Rust implementation appears to not be following the spec's four-pass algorithm for `leafUuid` extraction from summary records.

## The Problem

### Expected Behavior (Per Spec)

From [[08_CHAIN_LINKING_CANONICAL_REFERENCE.md]]:

1. `leafUuid` exists in `type: "summary"` records in JSONL files
2. This `leafUuid` points to a message UUID in the PARENT session
3. Sessions should be linked into chains via this explicit reference
4. Each chain should have distinct sessions with real timestamps

### Actual Behavior (Observed)

Query: `tastematter query chains --limit 20 --format json`

Returns:
```json
{
  "chains": [
    {
      "chain_id": "fa6b4bf6",
      "session_count": 149,
      "file_count": 717
    },
    // ... other chains with 0 files
  ]
}
```

Drilling into chain `fa6b4bf6`:
```json
{
  "session_id": "1777ab8d-8524-48e7-9d67-7e544a2782ee",
  "chain_id": "fa6b4bf6",
  "started_at": "2026-01-11T20:13:24.619104",  // ← All same timestamp!
  "file_count": 0,                              // ← All zero!
}
```

**Red flags:**
- All sessions have IDENTICAL timestamps (same second, slight microsecond variance)
- All sessions show `file_count: 0, total_accesses: 0`
- This indicates batch import, not actual conversation chain traversal

## Ground Truth: JSONL Structure

### Location
```
C:\Users\dietl\.claude\projects\C--Users-dietl-VSCode-Projects-taste-systems-gtm-operating-system\
```

### Key Finding: leafUuid Location

**leafUuid ONLY exists in `type: "summary"` records:**

```json
{"type":"summary","summary":"Context OS Instrumentation Layer...","leafUuid":"22288505-04d9-49bb-ba7a-eb99efbd94e7"}
```

**Regular messages have `uuid` and `parentUuid` but NOT `leafUuid`:**

```json
{
  "parentUuid": "e655dd02-2f3a-41f3-9345-7e3c11035e17",
  "sessionId": "846b76ee-3534-49ac-8555-cff4745c4a41",
  "type": "user",
  "uuid": "f3afe860-0a21-464d-b7fa-ca8bdcc713d6",
  ...
}
```

### Verified via Grep
```bash
# leafUuid only in summary records
grep -l "leafUuid" ~/.claude/projects/C--Users-.../*.jsonl
# Returns: Only files with type:"summary" entries
```

## Spec vs Implementation Mismatch

### Spec Says (Python Algorithm)

From [[08_CHAIN_LINKING_CANONICAL_REFERENCE.md]]:

```
Implementation Reference:
See: apps/context-os/cli/src/context_os_events/index/chain_graph.py
```

Four-pass algorithm:
1. Pass 1: Extract `leafUuid` from `type:"summary"` records
2. Pass 2: Extract `uuid` from message records (user/assistant/tool_result)
3. Pass 3: Build parent-child relationships (leafUuid → session ownership)
4. Pass 4: Group into chains via connected components

### Actual Implementation (Rust)

The CLI being used is:
```
apps/context-os/core/target/debug/context-os.exe
```

This is **Rust code**, not Python. The spec references Python files that may:
1. Not exist
2. Not be what the Rust code actually does
3. Be outdated relative to Rust implementation

## Hypothesis: Root Cause

The Rust indexer is likely:

1. **NOT parsing `type: "summary"` records** to extract `leafUuid`
2. OR using a different chain derivation method (hash of something?)
3. OR batch-importing sessions without actually traversing the leafUuid graph
4. OR the database schema differs from spec

Evidence: Chain ID `fa6b4bf6` appears to be a truncated hash, not derived from root session UUID as spec suggests.

## Files to Investigate

### Rust Source (PRIMARY)
| Path | Purpose |
|------|---------|
| `apps/context-os/core/src/` | Main Rust implementation |
| Look for: `chain`, `leaf`, `summary` | Chain linking logic |

### Python Source (SPEC REFERENCE)
| Path | Purpose |
|------|---------|
| `apps/context-os/cli/src/context_os_events/index/chain_graph.py` | Spec says this exists |
| May not exist or be outdated | Need to verify |

### Database
| Path | Purpose |
|------|---------|
| `~/.context-os/context_os.db` or similar | SQLite database |
| Check `chain_graph` and `chains` tables | Schema may differ from spec |

### Spec
| Path | Purpose |
|------|---------|
| [[08_CHAIN_LINKING_CANONICAL_REFERENCE.md]] | What SHOULD happen |

## Test Commands for Verification

### Check if Python implementation exists
```bash
ls -la apps/context-os/cli/src/context_os_events/index/chain_graph.py
```

### Check Rust source for chain logic
```bash
grep -r "chain" apps/context-os/core/src/ --include="*.rs"
grep -r "leaf" apps/context-os/core/src/ --include="*.rs"
grep -r "summary" apps/context-os/core/src/ --include="*.rs"
```

### Find the database location
```bash
find ~ -name "*.db" -path "*context*" 2>/dev/null
```

### Verify leafUuid in JSONL
```bash
grep "leafUuid" ~/.claude/projects/C--Users-dietl-VSCode-Projects-taste-systems-gtm-operating-system/*.jsonl | head -10
```

### Check chain_id derivation
```bash
# Get a chain and trace back to see how ID was generated
tastematter query flex --chain fa6b4bf6 --limit 5 --format json
```

## Resolution (2026-01-12)

### Bug Root Cause

**The Python indexer extracted ALL leafUuids from ALL summary records, but only the FIRST summary record indicates session resumption.**

Key insight: JSONL files contain two types of summary records:
1. **Session resumption** (first record): `leafUuid` points to parent session's message
2. **Compaction markers** (subsequent records): `leafUuid` points to message in THIS session

The original code in `extract_leaf_uuids()` iterated through ALL lines and collected every `leafUuid`, causing:
- Self-linking (sessions linked to themselves via compaction markers)
- Incorrect chain grouping

### Fix Applied

**File:** `apps/tastematter/cli/src/context_os_events/index/chain_graph.py`

Changed `extract_leaf_uuids()` to only read the FIRST record and return its `leafUuid` if it's a summary type.

### Verification Results

After fix:
- **150 of 153 sessions successfully linked** (98% match rate)
- **10 multi-session chains** identified
- **Top chain: 92 sessions** (was incorrectly showing as 149 in one chain)
- **5 branching parents** (sessions with multiple children)

### Chain Structure (Correct)

```
Sessions with resumption: 153
Message UUIDs indexed: 63,591
Linking: 150 matched, 3 orphaned (98.0%)

Top chains:
- 846b76ee: 92 sessions (main development chain)
- 2e826939: 39 sessions (secondary chain)
- 5083f8a5: 7 sessions
- 7fab4726: 7 sessions
- 1a424326: 5 sessions
```

### Why Some Sessions Are Orphans

3 sessions have `leafUuid` values that don't match any message UUID:
- Parent session was fully deleted (not just compacted)
- Or data corruption in JSONL files

### logicalParentUuid Note

Investigated `logicalParentUuid` in `type: "system", subtype: "compact_boundary"` records. These are for **within-session** continuity tracking (linking across compaction boundaries), NOT cross-session linking. Not needed for chain graph building.

## For Next Agent

**Status:** BUG FIXED

**What was done:**
1. Identified root cause: ALL summary leafUuids extracted instead of just FIRST
2. Fixed `chain_graph.py` to only use first record's leafUuid
3. Verified fix: 98% link rate, proper chain structure

**Do NOT:**
- Revert the `extract_leaf_uuids()` fix
- Modify the JSONL files (they're Claude Code's data)

## Evidence Collection

### Receipt IDs from CLI queries
- `[eecce25c]` - Initial Pixee file query
- `[485107e9]` - jan_2026 files query
- `[dbd2c157]` - Chain fa6b4bf6 Pixee files
- `[d4f9219c]` - Supabase files (0 results)

### Files examined
- `C:\Users\dietl\.claude\projects\C--Users-dietl-VSCode-Projects-taste-systems-gtm-operating-system\846b76ee-3534-49ac-8555-cff4745c4a41.jsonl`
- Tool results file showing chain data: `846b76ee-3534-49ac-8555-cff4745c4a41\tool-results\toolu_013LmjuhzXj86ojtQ2SVfkNC.txt`

### Grep results
- `leafUuid` found in 11 files, all in `type:"summary"` records
- Chain ID `fa6b4bf6` found in tool result caches from previous queries

---

**Document Status:** RESOLVED
**Created:** 2026-01-12
**Resolved:** 2026-01-12
**Author:** Context restoration session discovering and fixing chain linking bug
