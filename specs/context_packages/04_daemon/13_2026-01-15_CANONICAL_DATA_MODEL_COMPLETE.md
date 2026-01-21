---
title: "Canonical Data Model Specification Complete"
package_number: 13
date: 2026-01-15
status: current
previous_package: "[[12_2026-01-15_GLOB_BUG_DISCOVERY]]"
related:
  - "[[canonical/07_CLAUDE_CODE_DATA_MODEL]]"
  - "[[chain_graph.py]]"
tags:
  - context-package
  - tastematter
  - data-model
  - canonical
  - specification
---

# Canonical Data Model Specification Complete - Context Package 13

## Executive Summary

Following the glob bug discovery (Package 12), we conducted a comprehensive investigation of the Claude Code data architecture using three parallel exploration agents. The findings are now documented in a canonical specification: `specs/canonical/07_CLAUDE_CODE_DATA_MODEL.md`.

This package marks the completion of the data model investigation phase and provides a foundation for future chain linking work.

## What Was Done This Session

### 1. Three-Agent Deep Investigation

Spawned three parallel Explore agents to examine different aspects:

| Agent | Focus | Key Findings |
|-------|-------|--------------|
| Agent 1 | Filesystem structure | 18+ directories, 1.7GB total, complete hierarchy map |
| Agent 2 | JSONL data model | 16+ record types, field specifications, nesting patterns |
| Agent 3 | Linking mechanisms | 4 relationship types, parent/child and continuation |

### 2. Canonical Specification Created

**Location:** `specs/canonical/07_CLAUDE_CODE_DATA_MODEL.md`

**Contents:**
- Three-layer abstraction model (JSONL substrate → Context → Meta-context)
- Complete filesystem structure with file counts
- All 16+ JSONL record types with schemas
- Four linking mechanisms explained with diagrams
- Chain building algorithm (correct implementation)
- Common pitfalls and how to avoid them
- Statistics from GTM Operating System project

### 3. Key Architecture Insights Documented

**The Three Layers:**
```
Layer 3: Meta-Context (Tastematter)     ← What we build
Layer 2: Context (Claude Code)          ← Session management
Layer 1: JSONL Substrate (Files)        ← Raw data
```

**Why this matters:** The glob bug was a Layer 1 problem that manifested as Layer 3 symptoms. Understanding which layer you're operating at prevents this class of bugs.

**Four Linking Mechanisms:**
1. `parentUuid` - Message chain within session
2. `sessionId` + directory - Agent spawn linkage
3. `leafUuid` in summary - Session continuation
4. `logicalParentUuid` - Compaction skip links

## Statistics Captured

| Metric | Value |
|--------|-------|
| Total ~/.claude size | 1.7GB |
| projects/ (sessions) | 883MB (52%) |
| debug/ (logs) | 409MB (24%) |
| Total session files | 983 |
| Regular sessions | 324 |
| Top-level agents | 441 |
| Subdirectory agents | 218 |
| Tool result files | 384 |
| Record types documented | 16+ |

## Files Created

| File | Purpose |
|------|---------|
| `specs/canonical/07_CLAUDE_CODE_DATA_MODEL.md` | Authoritative reference for Claude Code data architecture |
| `specs/context_packages/04_daemon/13_...md` | This context package |

## Jobs To Be Done (Next Session)

1. [ ] **Rebuild chain graph database**
   ```bash
   cd apps/tastematter/cli
   tastematter daemon rebuild
   ```

2. [ ] **Verify chain counts match Claude Code UI**
   - Expected: ~356 sessions in largest chain
   - After recursive glob: should now find all

3. [ ] **Consider indexing tool-results/**
   - 384 .txt files not currently indexed
   - May contain important context

4. [ ] **Consider indexing other directories**
   - `todos/` - 2,352 files
   - `plans/` - 76 markdown files
   - `file-history/` - 19,414 versioned snapshots

## For Next Agent

### Context Chain

| # | Date | Title | Key Content |
|---|------|-------|-------------|
| 12 | 2026-01-15 | GLOB_BUG_DISCOVERY | Found 218 missing sessions, one-line fix |
| 13 | 2026-01-15 | CANONICAL_DATA_MODEL_COMPLETE | **This package** - comprehensive spec |

### Start Here

1. Read this package (you're doing it now)
2. Read the canonical spec for detailed reference:
   ```
   specs/canonical/07_CLAUDE_CODE_DATA_MODEL.md
   ```
3. Rebuild database to apply the glob fix:
   ```bash
   cd apps/tastematter/cli && tastematter daemon rebuild
   ```
4. Verify chain counts in UI

### Key References

- **Canonical spec:** `specs/canonical/07_CLAUDE_CODE_DATA_MODEL.md`
- **Implementation:** `cli/src/context_os_events/index/chain_graph.py`
- **Previous packages:** Package 11 (topology), Package 12 (glob bug)

### Do NOT

- Assume you understand the data model without reading the canonical spec
- Use `*.jsonl` glob patterns (must use `**/*.jsonl`)
- Mix abstraction layers when debugging

---

**Document Status:** CURRENT
**Session Duration:** ~30 minutes
**Primary Work:** Three-agent investigation, canonical spec creation
**Deliverable:** `specs/canonical/07_CLAUDE_CODE_DATA_MODEL.md`

