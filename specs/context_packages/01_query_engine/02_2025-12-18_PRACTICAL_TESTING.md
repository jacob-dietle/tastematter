---
title: "PRACTICAL TESTING"
package_number: 2
date: 2025-12-18
migrated_from: "apps/context-os/specs/context_os_intelligence/context_packages/02_2025-12-18_PRACTICAL_TESTING.md"
tags:
  - context-package
  - query-engine
  - legacy
---

# Context Package: Practical Testing & Mental Alignment

**Date:** 2025-12-18
**Session:** Index walkthrough + PMI refinement
**Status:** Complete - ready for Layer 2 implementation

---

## Summary

Conducted hands-on practical testing of the Context OS Intelligence index layer using real JSONL data (489 sessions). Walked through each index structure step-by-step to build mental alignment on what the deterministic layer provides vs what the agent layer must add.

**Key outcome:** The index reveals *what* happened (files, times, patterns) but not *why* or *what state* work is in. That semantic understanding requires Layer 2.

---

## Work Completed

### 1. Index Structures Validated

Built and tested all 5 index structures against real Claude Code session data:

| Structure | Result | What It Reveals |
|-----------|--------|-----------------|
| **Chain Graph** | 386 chains from 489 sessions | 2 large chains (33, 30 sessions) = deep work threads |
| **Inverted Index** | 819 files, 1503 accesses | `pipeline.yaml` most touched (16 sessions) |
| **Co-Access Matrix** | 55 files with game trails (PMI) | `agent.ts` ↔ `index.ts` always edited together |
| **Temporal Buckets** | 5 weeks of activity | W49 peak (60 ATP files), W50-51 declining |

### 2. PMI Refinement Implemented

**Problem discovered:** Jaccard similarity produced noise - 1.0 scores for file pairs touched together once.

**Solution implemented:** Replaced with PMI (Pointwise Mutual Information)

```python
# Before (Jaccard) - noisy
matrix = build_co_access_matrix(index)  # 191 files, many 1.0 scores

# After (PMI) - clean
matrix = build_co_access_matrix(index, min_co_occurrence=3)  # 55 files, meaningful scores
```

**Files changed:**
- `apps/context_os_events/src/context_os_events/index/co_access.py` - Added `_compute_pmi()`, updated `build_co_access_matrix()`
- `apps/context_os_events/tests/index/test_co_access.py` - Updated tests for new API

**Why PMI:** Spotify, Google, Netflix use PMI variants for recommendations. Measures whether co-occurrence is *surprising* given baseline popularity, not just raw overlap.

### 3. Practical Query: ATP Resume Context

Queried the index to help resume work on `automated_transcript_processing`:

**Index revealed:**
- Last ATP deploy: 2025-12-18 20:06 UTC
- 31 wrangler commands total for ATP
- Most recently touched: architecture guides (reading, not writing)
- Game trails: `agent.ts` → `index.ts`, `pipeline/index.ts`, `wrangler.toml`

**Index could NOT reveal:**
- "It was working perfectly until API key issue"
- "Deployment couldn't recover"
- "This was finishing touches, not new development"

---

## Key Discovery: The Layer 1 / Layer 2 Gap

### What Layer 1 (Index) Provides

| Capability | Example |
|------------|---------|
| File access history | "agent.ts touched in 7 sessions" |
| Chain detection | "33-session work thread via leafUuid" |
| Co-access patterns | "agent.ts + index.ts: PMI 2.2" |
| Temporal grouping | "W49 had 60 ATP files touched" |
| Fast O(1) lookups | Bloom filters, inverted index |

### What Layer 2 (Agent) Must Add

| Capability | Example |
|------------|---------|
| Intent extraction | "User was debugging, not developing" |
| State inference | "Deployment broken, attempting recovery" |
| Semantic clustering | "These 5 sessions are all about API key fix" |
| Work phase detection | "Finishing touches" vs "New feature" |
| Blockers/friction | "Blocked on X" from conversation content |

### The Gap in Practice

```
Index says:  "wrangler deploy at 20:06"
Agent adds:  "This was an attempt to fix broken deployment after API key expired"

Index says:  "architecture guides read today"
Agent adds:  "User was in 'understand before fixing' mode"

Index says:  "agent.ts + index.ts have PMI 2.2"
Agent adds:  "Load these together when resuming transcript processor work"
```

---

## Architecture Confirmed

```
┌─────────────────────────────────────────┐
│  LAYER 2: Intelligent Agent             │
│  • Intent extraction from conversations │
│  • Work state inference                 │
│  • Semantic clustering                  │
│  • Natural language queries             │
│  • "Why" and "what state" answers       │
└─────────────────────────────────────────┘
                   │
                   ▼ queries
┌─────────────────────────────────────────┐
│  LAYER 1: Deterministic Index           │
│  • Chain graph (leafUuid)               │
│  • Inverted file index                  │
│  • Co-access matrix (PMI)               │
│  • Temporal buckets                     │
│  • Bloom filters                        │
│  • "What" and "when" answers            │
└─────────────────────────────────────────┘
                   │
                   ▼ parses
┌─────────────────────────────────────────┐
│  RAW DATA: JSONL Session Files          │
│  ~/.claude/projects/*/                  │
│  489 sessions, 117 MB                   │
└─────────────────────────────────────────┘
```

---

## Next Steps

### Thread A: Fix ATP Deployment
- Debug API key / token issue
- Redeploy `automated_transcript_processing`
- Verify webhook processing works

### Thread B: Layer 2 Intelligence Extraction
1. **Spec the agent query interface** - What questions should it answer?
2. **Intent extraction** - Parse first user message, voice memos, slash commands
3. **State inference** - Detect "blocked", "debugging", "finishing", "new feature"
4. **Conversation summarization** - Compress session content for agent context
5. **Integration** - Expose index + semantic layer to Claude Code via skill/tool

### Specs to Write
- `05_AGENT_QUERY_INTERFACE.md` - Query primitives for Layer 2
- `06_INTENT_EXTRACTION_SPEC.md` - How to extract semantic signals
- `07_STATE_INFERENCE_SPEC.md` - Work phase detection algorithm

---

## Files Referenced

**Index implementation:**
- `apps/context_os_events/src/context_os_events/index/chain_graph.py`
- `apps/context_os_events/src/context_os_events/index/inverted_index.py`
- `apps/context_os_events/src/context_os_events/index/co_access.py` (PMI added)
- `apps/context_os_events/src/context_os_events/index/temporal.py`

**JSONL data:**
- `~/.claude/projects/C--Users-dietl-VSCode-Projects-taste-systems-gtm-operating-system/`
- 489 session files, 117 MB total

**Plan file:**
- `~/.claude/plans/deep-sparking-platypus.md`

---

## Session Metrics

- Sessions parsed: 489
- Chains detected: 386
- Files indexed: 819
- Game trails (PMI): 55 file pairs
- Tests passing: 21/21 (co_access)
- Lines of code changed: ~40 (PMI implementation)
