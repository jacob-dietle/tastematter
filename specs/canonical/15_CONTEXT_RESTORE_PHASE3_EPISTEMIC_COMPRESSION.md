---
title: "Context Restore Phase 3: Epistemic Compression Architecture"
type: architecture-spec
created: 2026-02-10
last_updated: 2026-02-10
status: draft
foundation:
  - "[[12_CONTEXT_RESTORATION_API_SPEC]]"
  - "[[05_INTELLIGENCE_LAYER_ARCHITECTURE]]"
  - "[[03_CORE_ARCHITECTURE]]"
knowledge_base:
  - "[[04_knowledge_base/technical/core_principles/epistemic-information-hierarchy]]"
  - "[[04_knowledge_base/technical/core_principles/anti-fragile-context-compression]]"
  - "[[04_knowledge_base/technical/core_principles/context-sensitivity]]"
  - "[[04_knowledge_base/technical/core_principles/progressive-context-disclosure]]"
  - "[[04_knowledge_base/technical/core_principles/context-coherence]]"
related:
  - "[[core/src/context_restore.rs]]"
  - "[[core/src/query.rs]]"
  - "[[intel/src/agents/context-synthesis.ts]]"
  - "[[core/src/intelligence/client.rs]]"
tags:
  - tastematter
  - context-restoration
  - intelligence-layer
  - epistemic-compression
  - canonical
---

# Context Restore Phase 3: Epistemic Compression Architecture

## Executive Summary

Phase 2 shipped single-call LLM synthesis for `tastematter context`. E2E testing (2026-02-10) revealed that the deterministic output is already strong at the PATTERNS layer, but the LLM synthesis produces incoherent output — tautological cluster interpretations, ungrounded suggested read reasons, and a narrative field that was null for the primary test case.

The root cause: **we applied a LOW context sensitivity solution (one Haiku call on file paths) to a HIGH context sensitivity problem (agent resuming cold on multi-workstream project).** The LLM lacks the context to compress meaningfully because it only sees the patterns layer, not the raw data or semantic files that give those patterns meaning.

Phase 3 replaces single-call synthesis with **multi-stage epistemic compression** — a pipeline that builds an information hierarchy layer by layer, with coherence grounding at each transition. The architecture is anti-fragile: more workstreams produce more independent compression units, yielding more precise output under increasing complexity.

## Evidence Base

### Phase 2 E2E Results (2026-02-10)

**With intel service running (nickel query):**
- `one_liner`: HIGH value — "Nickel transcript worker processing multi-provider integration..." (instant orientation)
- `cluster.name`: MEDIUM value — "Intercom Pipeline", "Type Contracts" (scannable labels)
- `cluster.interpretation`: LOW value — restates file paths in English ("These files form the core Intercom data flow")
- `suggested_read.reason`: LOW value — generic ("should be reviewed before deployment")
- `current_state.narrative`: NOT TESTED — null because no context packages found in scope

**Without intel service (graceful degradation):**
- All deterministic fields identical: clusters, PMI scores, access patterns, timeline, attention shifts
- Agent can already orient from deterministic output alone
- Intel layer adds value primarily in 2 of 5 enriched fields

**Key observation:** The deterministic output (PMI clustering, access pattern quadrants, Jaccard attention shift, context package discovery) already answers "what files cluster together and what's active?" — a LOW context sensitivity question. The HIGH sensitivity question — "what do I need to know to continue working on X across multiple workstreams?" — requires the NARRATIVE and INTENT layers that single-call synthesis cannot produce.

[VERIFIED: E2E test output captured in session 2026-02-10]
[VERIFIED: context_restore.rs Phase 5 synthesis call]

---

## The Epistemic-Information Hierarchy

### Model

```
             ┌─────────┐
             │  INTENT  │  WHY — workstream goals, primary focus, relationships
             │          │  Epistemic status: INFERRED (grounded in narrative evidence)
            ┌┴─────────┴┐
            │ NARRATIVE  │  WHAT — state per workstream, complete/pending/blocked
            │            │  Epistemic status: GROUNDED (cites patterns + packages + commits)
           ┌┴───────────┴┐
           │  PATTERNS    │  HOW — clusters, co-access, attention shifts
           │              │  Epistemic status: VERIFIED (deterministic computation)
          ┌┴─────────────┴┐
          │  RAW EVENTS    │  WHERE — file paths, timestamps, sessions, commits
          │                │  Epistemic status: MEASURED (direct observation)
          └───────────────┘
```

### Properties

**Each layer transition is an independent compression step.** You cannot collapse PATTERNS → INTENT in one step and maintain coherence. Phase 2 proved this — the LLM jumped from file paths to interpretations with no narrative intermediary, producing ungrounded claims.

**Each claim at layer N must cite evidence from layer N-1.** This is the context coherence principle applied to information compression. A narrative claim "HubSpot is in progress" must cite a pattern (3 commits in last week) or a package (pending_items in CP#34). An intent claim "primary focus is provider integration" must cite multiple narrative sources.

**Anti-fragile under complexity.** More workstreams → more independent compression units → each unit maintains its own grounding chain → output gets more precise, not vaguer. This is the inverse of single-call synthesis, which degrades as input complexity increases.

[GROUNDED: [[04_knowledge_base/technical/core_principles/epistemic-information-hierarchy]]]
[GROUNDED: [[04_knowledge_base/technical/core_principles/context-coherence]]]

---

## Workstream Detection

### The "What is a Workstream?" Problem

A workstream cannot be defined top-down in the abstract. The atomic unit is contextual — "tastematter" could be one workstream or five (CLI, frontend, intel, daemon, infra). This is an unsolvable definitional problem at the abstract level.

### Solution: Bottom-Up + Top-Down Convergence

**Bottom-up (statistical):** Co-access clusters provide boundaries. Files that move together in sessions form natural groupings. These are computed deterministically by the existing `query_co_access()` pipeline.

**Semantic inference (automatic):** Clusters naturally contain semantically meaningful files — READMEs, CLAUDE.md, specs/, context_packages/. During pre-computation, the daemon reads these files from within each cluster to extract workstream intent WITHOUT requiring user input. A cluster containing `transcript_worker/README.md` and `transcript_worker/specs/*.md` already carries semantic context about what that workstream IS.

**Top-down (user intent):** The user can optionally declare workstream labels ("that cluster is provider integration") which are persisted and override statistical boundaries. This correction improves future compression.

**Convergence flow:**
```
1. Daemon detects clusters from co-access patterns (automatic)
2. Daemon reads semantic files within each cluster (README, specs, CLAUDE.md)
3. Daemon pre-computes narrative per cluster using semantic context
4. Result: auto-detected workstream narratives with reasonable labels
5. User optionally corrects/renames/merges clusters into workstreams
6. Corrections persisted → used in future syncs
```

**Graceful degradation:**
- User provides labels → most precise compression
- No user labels but semantic files exist → reasonable inference
- No semantic files → statistical clusters with LLM-generated names (Phase 2 behavior)

---

## Multi-Stage Compression Pipeline

### Stage 1: MEASURED → PATTERNS (deterministic, already built)

**What exists:** `query_flex()`, `query_co_access()`, `query_timeline()`, `query_sessions()`, `query_chains()`, filesystem discovery for context packages.

**Output:** Clusters with files + PMI scores + access patterns, timeline with attention shifts, context packages with pending items, session metadata.

**Cost:** 0 LLM calls. <1 second. Pre-computed during `query_context()`.

**No changes needed.** Phase 1 deterministic pipeline is solid.

### Stage 2: PATTERNS → NARRATIVE (per-workstream LLM, pre-computable)

**This is the new stage.**

**Input per workstream/cluster:**
- Cluster file list + PMI scores + access patterns (from Stage 1)
- Git commits touching those files (from `git_commits` table — data exists, query method needed)
- Context packages scoped to workstream (from filesystem discovery)
- Semantic files within cluster (README.md, CLAUDE.md, specs/) — read and truncated
- Session summaries from chain cache (from `chain_summaries` table)

**Agent:** New `workstream-compression.ts` agent

**Prompt design principles:**
- "You are compressing raw project data into a workstream narrative."
- "Every claim MUST cite evidence: commit hash, context package number, access count, or file path."
- "Focus on: what's COMPLETE, what's IN PROGRESS, what's BLOCKED, what's the most important PENDING item."
- "Do NOT restate file names in English. The consumer already has the file list."
- Tool schema enforces: `state` (summary), `completed` (array with citations), `pending` (array with citations), `risk` (array with citations), `grounding` (source list)

**Output per workstream:**
```json
{
  "workstream_id": "cluster_0",
  "label": "Intercom Provider Integration",
  "state": "Webhook retry logic recently added, type contracts stabilized",
  "completed": [
    {"item": "Intercom pipeline", "evidence": "commit:a3f2b1 (Jan 28), 14 sessions"}
  ],
  "pending": [
    {"item": "HubSpot observability", "evidence": "context_package_09:pending_items"}
  ],
  "risk": [
    {"item": "Gong pipeline untouched 14 days", "evidence": "last_access: Jan 26"}
  ],
  "semantic_source": "transcript_worker/README.md",
  "grounding": ["commit:a3f2b1", "context_package_09", "cluster_pmi:0.614"],
  "computed_at": "2026-02-10T21:00:00Z"
}
```

**Pre-computation:** Daemon runs this per cluster after each sync. Cached in new `workstream_narratives` table. Invalidated when cluster composition changes or new commits arrive.

**Cost:** N Haiku calls per daemon sync (N = clusters, typically 3-5). ~$0.001-0.002 per sync.

### Stage 3: NARRATIVES → INTENT (orchestrator LLM, query-time)

**Input:**
- Pre-computed workstream narratives from Stage 2 cache (already compressed)
- Overall attention shift data (Jaccard similarity between periods)
- User-declared workstream labels (if any)
- Delta since last computation (new commits, new sessions since `computed_at`)

**Agent:** Evolution of current `context-synthesis.ts` → `context-orchestrator.ts`

**Prompt design principles:**
- "You are synthesizing an information hierarchy from pre-compressed workstream narratives."
- "Determine: which workstream is PRIMARY (most recent activity), how workstreams RELATE, what the agent should FOCUS on first."
- "The consuming agent starts cold with zero project knowledge. This is its compressed index."
- "Output must be structured for progressive disclosure: Level 1 intent (~200 tokens), Level 2 per-workstream detail."

**Output:** The final `ContextRestoreResult` with progressive disclosure structure (see Output Format below).

**Cost:** 1 Haiku call per query. ~$0.0003. Faster than Phase 2 because input is pre-compressed narratives (~1K tokens) not raw cluster data (~3K tokens).

---

## Output Format: Progressive Disclosure

### Level 1: Intent (~200 tokens, always consumed)

```json
{
  "intent": {
    "one_liner": "Multi-provider transcript ingestion — Intercom done, HubSpot in progress, conference scoring pivoting to enrichment",
    "workstreams": [
      {"id": "cluster_0", "label": "Provider Integration", "status": "active"},
      {"id": "cluster_4", "label": "Conference Enrichment", "status": "cooling"}
    ],
    "primary_focus": "cluster_0",
    "attention_shift": "Focus shifted from conference scoring to provider integration this week"
  }
}
```

The agent reads this and immediately knows: there are 2 workstreams, provider integration is primary, conference enrichment is cooling. It can now point its tools (grep, read) at the right files.

### Level 2: Narrative (~1K tokens, load per workstream)

```json
{
  "workstreams": {
    "cluster_0": {
      "label": "Provider Integration",
      "state": "Webhook retry logic recently added, type contracts stabilized",
      "completed": ["Intercom pipeline", "Gong pipeline", "type contracts"],
      "pending": ["HubSpot observability", "backfill collision fix"],
      "risk": ["Gong pipeline untouched 14 days"],
      "grounding": ["commit:a3f2b1", "context_package_09", "cluster_pmi:0.614"]
    },
    "cluster_4": {
      "label": "Conference Enrichment",
      "state": "Scoring pipeline complete, enrichment pipeline in progress",
      "completed": ["Supply search tooling", "Scoring prompts"],
      "pending": ["Enrichment persistence"],
      "risk": [],
      "grounding": ["context_package_33", "cluster_pmi:0.778"]
    }
  }
}
```

The agent loads this when it's about to work on a specific workstream. Every claim is verifiable against the grounding sources.

### Level 3: Patterns (existing deterministic output)

The cluster file lists, suggested reads with surprise flags, timeline with access counts, context package quick_start commands. The agent navigates this with its own tools — grep, glob, read — guided by Level 1+2 understanding.

---

## Pre-Computation Strategy

### Daemon Sync Enhancement

The daemon already runs periodically (`daemon once` or background interval) and syncs sessions, commits, and chains. Phase 3 adds:

**After cluster detection:**
1. For each cluster, identify semantic files (glob for README.md, CLAUDE.md, specs/*.md within cluster file paths)
2. Read semantic file content (truncated to 2K chars per file)
3. Query `git_commits` for commits touching cluster files (last 30 days)
4. Query `chain_summaries` for session summaries related to cluster files
5. Call workstream-compression agent (Stage 2) per cluster
6. Cache result in `workstream_narratives` table

**Invalidation:**
- Recompute when: cluster file composition changes (new files enter/leave cluster)
- Recompute when: new commits arrive touching cluster files
- Recompute when: new context packages discovered
- TTL: 24 hours max (force refresh even if no changes detected)

### New Database Schema

```sql
CREATE TABLE IF NOT EXISTS workstream_narratives (
    cluster_id TEXT PRIMARY KEY,
    label TEXT,                    -- auto-detected or user-provided
    label_source TEXT,             -- 'auto' | 'user' | 'semantic'
    narrative_json TEXT NOT NULL,  -- Stage 2 output (JSON blob)
    semantic_files TEXT,           -- JSON array of semantic files found
    files_hash TEXT,               -- hash of cluster file list (for invalidation)
    computed_at TEXT NOT NULL,
    model_used TEXT
);

CREATE TABLE IF NOT EXISTS workstream_user_labels (
    cluster_id TEXT PRIMARY KEY,
    user_label TEXT NOT NULL,
    created_at TEXT NOT NULL
);
```

### New Query Method

```rust
// New: query git commits for files in a cluster
pub async fn query_commits_for_files(
    &self,
    file_patterns: &[String],
    since_days: i32,
    limit: i32,
) -> Result<Vec<GitCommitRow>, CoreError>
```

This reads from the `git_commits` table (data already synced by daemon) filtering by `files_changed` matching the cluster's file paths.

---

## New Intel Agents

### workstream-compression.ts (Stage 2)

Follows existing agent pattern (`chain-summary.ts` as template).

**System prompt:**
- "You are compressing raw project data into a workstream state narrative."
- "Every claim MUST cite evidence from the provided data."
- "Focus on what's COMPLETE, IN PROGRESS, BLOCKED."
- "Do NOT restate file names. The consumer has the file list."

**Tool schema:** `output_workstream_narrative`
- `label`: string (2-5 word workstream name)
- `state`: string (1-2 sentence current state)
- `completed`: array of {item, evidence}
- `pending`: array of {item, evidence}
- `risk`: array of {item, evidence}

**Model:** Haiku (same as all other intel agents)

### context-orchestrator.ts (Stage 3, evolution of context-synthesis.ts)

**System prompt:**
- "You are synthesizing an information hierarchy from pre-compressed workstream narratives."
- "Determine primary focus, workstream relationships, and recommended first action."
- "The consuming agent starts cold. This is its compressed index."

**Tool schema:** `output_context_intent`
- `one_liner`: string (<120 chars)
- `workstream_summary`: array of {id, label, status}
- `primary_focus`: string (workstream id)
- `attention_shift`: string (1 sentence)
- `recommended_first_action`: string

**Model:** Haiku

---

## Migration Path from Phase 2

Phase 2 code is preserved and becomes the fallback:

1. `context-synthesis.ts` remains as-is (not deleted)
2. New `workstream-compression.ts` and `context-orchestrator.ts` added alongside
3. `IntelClient` gets new methods: `compress_workstream()`, `orchestrate_context()`
4. `query_context()` in query.rs checks for pre-computed narratives:
   - If workstream_narratives exist and are fresh → Stage 3 only (1 call)
   - If no pre-computed narratives → fall back to Phase 2 (1 call, current behavior)
5. Daemon sync enhanced to run Stage 2 after cluster detection
6. Output format extended with `intent` and `workstreams` sections

**No breaking changes.** Phase 2 fields (`one_liner`, `name`, `interpretation`, `reason`) remain in output. Phase 3 adds new sections alongside them. Consumers that don't know about Phase 3 sections ignore them.

---

## Cost Model

| Stage | When | Calls | Cost | Latency |
|-------|------|-------|------|---------|
| Stage 1 | Query time | 0 (deterministic) | $0 | <1s |
| Stage 2 | Daemon sync | N per sync (N=3-5 clusters) | ~$0.001-0.002 | Background |
| Stage 3 | Query time | 1 orchestrator | ~$0.0003 | ~3-5s |
| **Phase 2 fallback** | Query time | 1 synthesis | ~$0.0003 | ~12s |

Phase 3 query-time latency is LOWER than Phase 2 because the orchestrator works with pre-compressed narratives (~1K tokens input) instead of raw cluster data (~3K tokens input).

---

## Success Criteria

1. `tastematter context "nickel"` with pre-computed narratives → output includes `intent` section with workstream hierarchy, grounding citations, and progressive disclosure structure
2. Every narrative claim traceable to deterministic evidence (commit hash, package #, access count)
3. Agent starting cold can determine primary workstream and first action from Level 1 output (~200 tokens)
4. More workstreams → more specific output (not vaguer)
5. No pre-computed narratives → graceful fallback to Phase 2 behavior (no regression)
6. Daemon pre-computation adds <10 seconds to sync time

---

## Open Questions

### Q1: Workstream persistence across syncs

When cluster composition shifts (files enter/leave clusters), should old workstream narratives be versioned or replaced? Versioning enables "what changed since last sync" deltas but adds storage complexity.

**Recommendation:** Replace for now. Delta computation is a Phase 4 concern.

### Q2: Cross-repo workstream detection

Current clustering is per-database (one repo or one context OS). Multi-repo workstreams (e.g., "nickel" spanning transcript_worker and conference_pr) require cross-database queries.

**Recommendation:** Defer. Single-repo workstreams first. Cross-repo is the [[10_MCP_PUBLISHING_ARCHITECTURE]] concern.

### Q3: User correction UX

How does the user declare workstream labels? Options:
1. Config file (`~/.context-os/workstreams.yaml`)
2. CLI command (`tastematter workstream label cluster_0 "Provider Integration"`)
3. Interactive prompt during `tastematter context`

**Recommendation:** Option 2 (CLI command) for explicitness. Config file as persistence backend.

---

## References

- [[12_CONTEXT_RESTORATION_API_SPEC]] — Original Phase 1-3 specification (Phase 3 superseded by this spec)
- [[05_INTELLIGENCE_LAYER_ARCHITECTURE]] — Intel service architecture and agent patterns
- [[04_knowledge_base/technical/core_principles/epistemic-information-hierarchy]] — The pyramid model
- [[04_knowledge_base/technical/core_principles/anti-fragile-context-compression]] — Anti-fragile property
- [[04_knowledge_base/technical/core_principles/context-sensitivity]] — Context sensitivity formula
- [[04_knowledge_base/technical/core_principles/progressive-context-disclosure]] — Three-level disclosure
- [[04_knowledge_base/technical/core_principles/context-coherence]] — Grounding chain requirement

---

**Specification Status:** DRAFT
**Created:** 2026-02-10
**Author:** Design session — context restore Phase 3 rethink
**Evidence:** Phase 2 E2E testing revealed single-call synthesis incoherence; epistemic grounding analysis identified multi-stage compression as solution
**Next Action:** Review, then implement Stage 2 (workstream-compression agent + daemon enhancement)
