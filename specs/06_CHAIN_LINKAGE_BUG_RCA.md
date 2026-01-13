# Tastematter Visual Debug Report

**Date:** 2026-01-11
**Method:** Chrome automation + CLI cross-reference
**Status:** Issues documented, ready for prioritization

---

## Executive Summary

Visual testing revealed **critical data architecture issues** causing broken functionality across all views. The root cause is **missing chain-to-file linkage** in the database layer.

| View | Status | Primary Issue |
|------|--------|---------------|
| Files | Partially Working | Chain filtering broken (0 files per chain) |
| Timeline | **Broken** | Shows individual files, not meaningful clusters |
| Sessions | Partially Working | All sessions show "No chain" - linkage broken |
| Chains Sidebar | **Data Bug** | All chains show "0 files" |

---

## Critical Bugs (P0)

### BUG-001: Chain-to-File Linkage Broken

**Status:** ✅ FIXED (2026-01-11)

**Symptom:** All chains show "0 files" in sidebar and CLI
**Evidence:**
```json
// CLI: query chains --format json
{
  "chain_id": "7f389600",
  "session_count": 81,
  "file_count": 0  // <-- Should NOT be 0
}
```

**UI Screenshot Evidence:** Chains sidebar shows "81 sessions 0 files" for all chains

**Impact:**
- Chain filtering does nothing (no files to filter to)
- Sessions show "No chain" badge for ALL sessions
- Cannot trace work threads

**Root Cause Analysis:**
1. Chain graph exists (50 chains with session counts)
2. Sessions have files (verified via `query sessions`)
3. Missing link: session-to-chain relationship not propagated to file queries

**Location to Fix:** `apps/context-os/core/src/` - chain query logic

---

### BUG-002: Session-Chain Linkage Not Exposed

**Status:** ✅ RESOLVED (Not a bug - working as designed)

**Original Symptom:** Sessions display "No chain" badge
**RCA Investigation (2026-01-11):**
```json
// CLI: query sessions --time 7d --format json
// Sessions WITH chains correctly return chain_id:
{
  "session_id": "agent-a74a25a",
  "chain_id": "ed7d532b",  // <-- Present when session has chain
  "file_count": 2
}

// Sessions WITHOUT chains correctly omit field:
{
  "session_id": "agent-afbb5b7",
  "file_count": 9
  // chain_id omitted (serde skip_serializing_if = "Option::is_none")
}
```

**Finding:** Backend correctly returns chain_id via LEFT JOIN to chain_graph.
- 556 sessions have chain_id in 30d window
- 123 sessions have chain_id in 7d window
- Sessions showing "No chain" genuinely have no chain linkage

**Root Cause:** Not a bug. Recent sessions (agent-*) may not have been linked to chains yet by the indexer.

---

## High Priority Issues (P1)

### ISSUE-003: Timeline View Shows Individual Files (Useless)

**Symptom:** Timeline displays individual file rows instead of meaningful work units
**User Quote:** "Timeline view is basically worthless... showing individual files... not useful"

**Current State:**
- Shows: env.ts, corpus-do.ts, C:\Users\dietl\VSCode Project... (truncated)
- Shows: Only 1 day column (Jan 4)
- Shows: File-level granularity with no grouping

**What User Needs:**
- Project/directory clusters (what was I working on?)
- Session groupings (when did I work on it?)
- Chain-level view (conversation threads over time)
- Meaningful time distribution

**Evidence from Roadmap:**
> "Timeline View | 40% | Structure exists, **per-day data is simulated**"

**Root Cause:** Timeline architecture designed for file-level view, not cluster/session view

---

### ISSUE-004: Session Names Are Meaningless Hashes

**Symptom:** Sessions show "agent-af", "agent-a8" instead of meaningful names
**Evidence:** CLI returns `session_id: "agent-afbb5b7"`, UI truncates to "agent-af"

**Impact:**
- Cannot identify sessions at a glance
- No context about what work was done
- Forces user to expand every session to understand it

**Design Improvement Needed:**
- Intelligent session naming (from Intelligence Layer spec)
- Show primary project/file as session identifier
- Or show session purpose derived from files touched

---

### ISSUE-005: Timeline Buckets Empty

**Symptom:** CLI returns `buckets: {}` for each file in timeline query
**Evidence:**
```json
{
  "file_path": "env.ts",
  "total_accesses": 2,
  "buckets": {},  // <-- Empty! Should have per-day counts
}
```

**Impact:** Cannot show per-day activity distribution for files

---

## Medium Priority Issues (P2)

### ISSUE-006: Git Status Error in HTTP Mode

**Symptom:** Git Status panel shows "Error: Cannot read properties of undefined (reading 'invoke')"
**Context:** Expected in HTTP transport mode (no Tauri)
**Fix:** Graceful degradation - show "Git status unavailable in browser mode"

---

### ISSUE-007: File Paths Truncated Beyond Usefulness

**Symptom:** Paths show as "C:\Users\dietl\VSCode Project..."
**Impact:** Cannot identify which project or directory a file belongs to
**Fix:** Show relative paths from repo root, or use intelligent truncation (show filename + parent)

---

### ISSUE-008: Chain Click Doesn't Filter Views

**Symptom:** Clicking chain in sidebar highlights it but doesn't filter Files/Timeline view
**Root Cause:** Chain filtering depends on chain-file linkage (BUG-001)
**Fix:** Once BUG-001 fixed, verify filtering works

---

### ISSUE-009: Inconsistent File Counts

**Symptom:**
- Files view: "46 files"
- CLI query flex: 20 files (with --limit 20)
- Timeline: "30 files"
- Sessions: "48 files"

**Impact:** Confusing - which count is correct?
**Root Cause:** Different queries, different aggregations, no single source of truth display

---

## UX Issues (P3)

### ISSUE-010: No Loading States

**Symptom:** Views don't show loading indicators during data fetch
**Impact:** User doesn't know if app is working or frozen

---

### ISSUE-011: No Empty States

**Symptom:** When chain has 0 files, view just shows all files instead of "No files in this chain"
**Impact:** User doesn't understand filter is active but empty

---

### ISSUE-012: Heat Map Legend Not Clear

**Symptom:** "ACTIVITY: Low [colors] High" legend doesn't explain what the colors mean
**Impact:** User has to guess what intensity represents (access count? recency?)

---

## Data Architecture RCA

```
                  CHAIN GRAPH (exists)
                       │
                       │ session_count: 81
                       │ file_count: 0  <-- BROKEN LINK
                       │
                       ▼
              ┌────────────────┐
              │   SESSIONS     │
              │ (files exist)  │
              └───────┬────────┘
                      │
                      │ No chain_id exposed
                      │
                      ▼
              ┌────────────────┐
              │    FILES       │
              │ (data exists)  │
              └────────────────┘

PROBLEM: Middle layer (session-chain link) broken
         Chain → Session link exists (session_count > 0)
         Session → File link exists (files in session)
         Chain → File link MISSING (file_count = 0)
```

---

## Recommended Fix Order

### Phase 1: Data Layer Fixes (Unblocks Everything)
1. **BUG-001:** Fix chain-file linkage in Rust core
2. **BUG-002:** Add chain_id to session query response

### Phase 2: View Improvements
3. **ISSUE-003:** Redesign Timeline to show sessions/projects, not files
4. **ISSUE-004:** Implement session naming (basic: show primary directory)

### Phase 3: Polish
5. **ISSUE-006:** Graceful Git Status in HTTP mode
6. **ISSUE-007:** Intelligent path truncation
7. **ISSUE-010/011:** Loading and empty states

---

## Verification Commands

```bash
# Verify chain-file linkage after fix
context-os.exe query chains --format json
# Expect: file_count > 0 for chains with sessions

# Verify chain filtering
context-os.exe query flex --chain 7f389600 --format json
# Expect: Results with files from that chain

# Verify session-chain linkage
context-os.exe query sessions --format json
# Expect: chain_id field in each session
```

---

## Related Specs

- [[05_INTELLIGENCE_LAYER_ARCHITECTURE.md]] - Session naming via LLM
- [[02_ROADMAP.md]] - Phase dependencies
- [[03_CORE_ARCHITECTURE.md]] - Data layer design

---

**Report Generated:** 2026-01-11T21:30:00Z
**Method:** Chrome MCP automation + CLI cross-reference
**Tab ID:** 1286546403
