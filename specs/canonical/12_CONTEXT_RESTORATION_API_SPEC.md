---
title: "Context Restoration API Specification"
type: architecture-spec
created: 2026-02-03
last_updated: 2026-02-03
status: draft
foundation:
  - "[[canonical/00_VISION]]"
  - "[[canonical/05_INTELLIGENCE_LAYER_ARCHITECTURE]]"
  - "[[canonical/07_CLAUDE_CODE_DATA_MODEL]]"
  - "[[.claude/skills/context-query/SKILL.md]]"
related:
  - "[[canonical/03_CORE_ARCHITECTURE]]"
  - "[[canonical/10_MCP_PUBLISHING_ARCHITECTURE]]"
tags:
  - tastematter
  - context-restoration
  - api-design
  - intelligence-layer
  - canonical
---

# Context Restoration API Specification

## Executive Summary

This specification defines the **Context Restoration API** - a high-level interface that combines deterministic hypercube queries with LLM-powered synthesis to answer the question: "What do I need to know to continue working on X?"

**Key insight:** Raw data (file counts, PMI scores, timestamps) becomes actionable through interpretation. The API returns **grounded narratives** - every claim cites a verifiable source.

**Design philosophy:**
- Deterministic data provides the foundation (verifiable, fast, cheap)
- LLM synthesis provides meaning (narrative, interpretation, surprises)
- Every synthesized claim grounds to a deterministic source

---

## Problem Statement

### Current State

The existing CLI returns raw data requiring manual interpretation:

```bash
$ tastematter query flex --files "*nickel*" --agg count,recency,sessions,chains --format json

# Returns:
{
  "results": [
    {"file_path": "...README.md", "access_count": 43, "session_count": 34},
    {"file_path": "...index.ts", "access_count": 41, "session_count": 30},
    ...
  ]
}
```

**Problems:**
1. Agent must decide which query strategy to use (9 strategies in skill doc)
2. Results are facts, not meaning ("43 accesses" vs "reference doc")
3. No narrative connecting files to work
4. No suggested next actions
5. Surprises hidden in data (co-accessed files not surfaced)

### Vision Gap

From [[00_VISION]]:
> "Tastematter enables humans to SEE the git state, RESPOND to agent modifications, and COORDINATE through the stigmergic substrate."

From [[05_INTELLIGENCE_LAYER_ARCHITECTURE]]:
> "The layer integrates Claude Agent SDK to provide... Proactive Insights - Patterns surfaced before users ask"

**Gap:** No single command synthesizes hypercube data into actionable context restoration.

---

## Target Interface

### Command

```bash
tastematter context "<query>" [--depth quick|medium|deep] [--format json|narrative]
```

**Examples:**
```bash
tastematter context "nickel"                    # Default: medium depth, json
tastematter context "what am I working on"      # Natural language query
tastematter context "pixee" --depth deep        # Full analysis
tastematter context "last week" --format narrative  # Human-readable output
```

### Response Structure

```json
{
  "receipt_id": "ctx_7f3a2b",
  "query": "nickel",
  "generated_at": "2026-02-03T03:30:00Z",
  "model_used": "claude-3-5-haiku-latest",

  "executive_summary": { ... },
  "current_state": { ... },
  "continuity": { ... },
  "work_clusters": [ ... ],
  "suggested_reads": [ ... ],
  "timeline": { ... },
  "insights": [ ... ],
  "verification": { ... },
  "quick_start": { ... }
}
```

---

## Response Schema

### 1. Executive Summary

5-second orientation for the query.

```json
{
  "executive_summary": {
    "one_liner": "Nickel transcript worker is production-ready with full provider parity and comprehensive security.",
    "status": "healthy | warning | stale | unknown",
    "confidence": 0.92,
    "work_tempo": "active | cooling | dormant",
    "last_meaningful_session": "2026-02-03T03:30:00Z"
  }
}
```

| Field | Source | Type |
|-------|--------|------|
| `one_liner` | LLM synthesis from file patterns + context packages | Synthesized |
| `status` | Derived from recency + test state + deployment state | Deterministic |
| `confidence` | LLM self-assessment | Synthesized |
| `work_tempo` | Derived from session frequency over time | Deterministic |
| `last_meaningful_session` | Most recent session with >5 file accesses | Deterministic |

**Status derivation:**
- `healthy`: Recent activity, tests passing (if detectable), no warnings
- `warning`: Stale context packages, failing tests, unreviewed agent commits
- `stale`: No activity in >14 days
- `unknown`: Insufficient data

**Work tempo derivation:**
- `active`: Sessions in last 3 days
- `cooling`: Sessions in last 7 days but not last 3
- `dormant`: No sessions in 7+ days

---

### 2. Current State

Narrative understanding of where things are.

```json
{
  "current_state": {
    "narrative": "You built a multi-provider transcript ingestion system. All 4 providers (Fireflies, Gong, Intercom, HubSpot) are at full parity with pipelines, webhooks, backfill, and observability. Security layer completed yesterday - Cloudflare Access protects admin endpoints, webhooks validate signatures. 220 tests passing, deployed to production.",

    "key_metrics": {
      "corpus_size": {
        "value": 2390,
        "unit": "tickets",
        "breakdown": { "intercom": 2084, "hubspot": 306 }
      },
      "test_coverage": {
        "value": 220,
        "unit": "tests",
        "status": "passing"
      },
      "providers": {
        "value": 4,
        "status": "full_parity"
      }
    },

    "evidence": [
      {
        "claim": "4 providers at parity",
        "source": "10_SECURITY_POSTURE_COMPLETE.md:59-66",
        "verifiable": true
      },
      {
        "claim": "220 tests passing",
        "source": "10_SECURITY_POSTURE_COMPLETE.md:101",
        "verifiable": true
      }
    ]
  }
}
```

| Field | Source | Type |
|-------|--------|------|
| `narrative` | LLM synthesis from context packages + file patterns | Synthesized |
| `key_metrics` | Extracted from context packages (structured data) | Deterministic |
| `evidence` | File:line citations for each claim | Deterministic |

**Narrative generation strategy:**
1. Find context packages via glob (`**/context_packages/*.md`)
2. Read latest package (highest number)
3. Extract: status tables, metrics, completion lists
4. Synthesize into 2-4 sentence narrative
5. Ground each claim to source file:line

---

### 3. Continuity

Where you left off and what's pending.

```json
{
  "continuity": {
    "left_off_at": {
      "description": "Completed security posture implementation",
      "last_context_package": "10_2026-02-02_SECURITY_POSTURE_COMPLETE.md",
      "last_action": "Deployed with Cloudflare Access + JWT/HMAC signatures"
    },

    "pending_items": [
      {
        "item": "Monitor D1 for live wide events",
        "source": "10_SECURITY_POSTURE_COMPLETE.md:129",
        "priority": "next",
        "command": "SELECT * FROM flow_logs ORDER BY timestamp DESC LIMIT 10"
      },
      {
        "item": "Verify Gong JWT auth in production",
        "source": "10_SECURITY_POSTURE_COMPLETE.md:132",
        "priority": "next",
        "expected_signal": "auth: { method: 'jwt' } in logs"
      }
    ],

    "chain_context": {
      "chain_id": "nickel-transcript-worker",
      "total_packages": 10,
      "timeline": "Jan 4 - Feb 2 (29 days)",
      "arc": "Setup -> TDD -> Providers -> Backfill -> Observability -> Security"
    }
  }
}
```

| Field | Source | Type |
|-------|--------|------|
| `left_off_at` | Latest context package "For Next Agent" section | Deterministic |
| `pending_items` | Extracted from "Jobs To Be Done" / TODO sections | Deterministic |
| `chain_context.arc` | LLM synthesis of package titles over time | Synthesized |

**Pending item extraction:**
1. Search for patterns: `- [ ]`, `TODO`, `Next:`, `Jobs To Be Done`
2. Extract with source file:line
3. Prioritize by position (earlier = higher priority)

---

### 4. Work Clusters

Files that move together, with interpretation.

```json
{
  "work_clusters": [
    {
      "name": "Core Implementation",
      "files": [
        "src/index.ts",
        "src/pipeline/intercom.ts",
        "src/providers/intercom.ts",
        "src/providers/hubspot.ts"
      ],
      "pmi_score": 0.73,
      "interpretation": "Main entry point and provider implementations - these files move together as the core pipeline",
      "access_pattern": "high_access_high_session"
    },
    {
      "name": "Type System",
      "files": [
        "src/types/webhook.ts",
        "src/types/ticket.ts",
        "src/types/nickel-ticket.ts"
      ],
      "pmi_score": 0.71,
      "interpretation": "Type contracts - reference docs accessed repeatedly during implementation",
      "access_pattern": "high_access_low_session"
    }
  ]
}
```

| Field | Source | Type |
|-------|--------|------|
| `files` | Co-access query results | Deterministic |
| `pmi_score` | Pointwise Mutual Information calculation | Deterministic |
| `name` | LLM inference from file paths | Synthesized |
| `interpretation` | LLM explanation of what PMI means | Synthesized |
| `access_pattern` | Derived from access_count vs session_count ratio | Deterministic |

**Access pattern derivation:**
- `high_access_high_session`: Active development (many accesses across many sessions)
- `high_access_low_session`: Reference doc (read repeatedly in few sessions)
- `low_access_high_session`: Touched lightly across contexts
- `low_access_low_session`: One-off reference

**Cluster detection algorithm:**
1. Start with highest-access file as seed
2. Run co-access query (PMI > 0.6)
3. Group files with similar PMI into cluster
4. Name cluster via LLM from file paths
5. Repeat for files not yet clustered

---

### 5. Suggested Reads

Prioritized files to load into context.

```json
{
  "suggested_reads": [
    {
      "path": "context_packages/10_2026-02-02_SECURITY_POSTURE_COMPLETE.md",
      "reason": "Latest context package - start here to resume",
      "priority": 1,
      "time_estimate": "3 min read",
      "surprise": false
    },
    {
      "path": "context_packages/README.md",
      "reason": "Timeline of all 10 packages - see the full arc",
      "priority": 2,
      "time_estimate": "1 min read",
      "surprise": false
    },
    {
      "path": ".claude/plans/rippling-snacking-zephyr.md",
      "reason": "Related Claude plan discovered via co-access",
      "priority": 3,
      "time_estimate": "2 min read",
      "surprise": true
    }
  ]
}
```

| Field | Source | Type |
|-------|--------|------|
| `path` | Derived from access patterns + co-access | Deterministic |
| `reason` | LLM explanation of why this file matters | Synthesized |
| `priority` | Ranked by recency + access count + co-access surprise | Deterministic |
| `time_estimate` | Derived from file size (lines / 50 wpm) | Deterministic |
| `surprise` | True if file appears in co-access but not in primary query | Deterministic |

**Prioritization algorithm:**
1. Latest context package = priority 1 (always)
2. Context package README = priority 2 (if exists)
3. Surprise co-access files = priority 3+ (valuable discoveries)
4. High-access files not in packages = priority 4+

---

### 6. Timeline

Recent attention patterns.

```json
{
  "timeline": {
    "recent_focus": [
      { "period": "Feb 1-2", "focus": "Security posture", "files_touched": 12 },
      { "period": "Jan 28-29", "focus": "HubSpot webhooks + backfill", "files_touched": 23 },
      { "period": "Jan 27", "focus": "HubSpot provider implementation", "files_touched": 18 }
    ],

    "attention_shift": {
      "detected": true,
      "description": "Shifted from feature work (HubSpot) to hardening (security) on Feb 1",
      "interpretation": "Normal progression: build -> secure -> ship"
    }
  }
}
```

| Field | Source | Type |
|-------|--------|------|
| `recent_focus[].period` | Temporal bucketing of sessions | Deterministic |
| `recent_focus[].focus` | LLM inference from file paths in period | Synthesized |
| `recent_focus[].files_touched` | Count from temporal query | Deterministic |
| `attention_shift.detected` | Compare focus areas across periods | Deterministic |
| `attention_shift.description` | LLM narrative of shift | Synthesized |
| `attention_shift.interpretation` | LLM assessment of shift meaning | Synthesized |

---

### 7. Insights

Proactive discoveries and observations.

```json
{
  "insights": [
    {
      "type": "completion",
      "title": "Major milestone reached",
      "description": "Full provider parity achieved - all 4 providers have pipeline, webhook, backfill, observability, and security. This is a natural pause point.",
      "evidence": ["Provider matrix shows all checkmarks", "Security layer complete"],
      "action": {
        "label": "Consider next phase",
        "suggestion": "Good time to plan next capability or move to maintenance mode"
      }
    },
    {
      "type": "surprise",
      "title": "Claude plan file in co-access",
      "description": "rippling-snacking-zephyr.md appears in your work cluster but isn't in context packages. May contain planning context worth reading.",
      "evidence": ["PMI 0.73 with index.ts"],
      "action": {
        "label": "Read plan file",
        "path": ".claude/plans/rippling-snacking-zephyr.md"
      }
    }
  ]
}
```

**Insight types:**

| Type | Trigger | Example |
|------|---------|---------|
| `completion` | Context package with "complete" in title + no pending items | "Major milestone reached" |
| `surprise` | High-PMI file not in expected paths | "Claude plan file in co-access" |
| `stale` | Context package >7 days old | "Context may be outdated" |
| `abandoned` | Files with old last_access, no recent sessions | "Potentially abandoned work" |
| `risk` | Agent commits not reviewed (from git analysis) | "3 agent commits pending review" |

---

### 8. Verification

Audit trail and source citations.

```json
{
  "verification": {
    "receipt_id": "ctx_7f3a2b",
    "command": "tastematter context verify ctx_7f3a2b",
    "sources_cited": 8,
    "deterministic_data": {
      "files_analyzed": 30,
      "sessions_analyzed": 34,
      "co_access_pairs": 145
    },
    "synthesis_data": {
      "model": "claude-3-5-haiku-latest",
      "tokens_used": 1247,
      "cost_usd": 0.00031
    }
  }
}
```

**Verification command:**
```bash
$ tastematter context verify ctx_7f3a2b

Receipt: ctx_7f3a2b
Status: MATCH (data unchanged since generation)
Sources verified: 8/8
```

---

### 9. Quick Start

Copy-paste commands to resume work.

```json
{
  "quick_start": {
    "commands": [
      {
        "description": "Verify tests still pass",
        "command": "cd apps/clients/nickel/transcript_worker && pnpm test"
      },
      {
        "description": "Check D1 for live events",
        "command": "wrangler d1 execute nickel-transcript-logs --command=\"SELECT * FROM flow_logs ORDER BY timestamp DESC LIMIT 10\""
      }
    ],
    "expected_results": {
      "tests": "220 passing",
      "access_check": "302 redirect to Cloudflare login"
    }
  }
}
```

| Field | Source | Type |
|-------|--------|------|
| `commands` | Extracted from "Test Commands" sections in context packages | Deterministic |
| `expected_results` | Extracted from verification sections | Deterministic |

---

## Depth Levels

### Quick (< 2 seconds)

Cache-only, no LLM calls.

**Returns:**
- `executive_summary` (cached or derived from last access)
- `work_clusters` (from cached co-access)
- `suggested_reads` (top 3 by access count)

**Use case:** Fast orientation, checking if context is stale

### Medium (< 5 seconds) - Default

One LLM call for synthesis.

**Returns:** All sections

**LLM input:**
- Latest context package content
- File list with access counts
- Top co-access pairs

**Use case:** Standard context restoration

### Deep (< 15 seconds)

Multiple LLM calls, full analysis.

**Additional processing:**
- Read multiple context packages for arc synthesis
- Analyze git commits for agent work
- Generate richer insights

**Use case:** Starting major work session, comprehensive orientation

---

## Implementation Architecture

### Data Flow

```
tastematter context "nickel"
         │
         ▼
┌─────────────────────────────────────────────────────────────┐
│                    RUST CORE (context-os)                    │
│                                                              │
│  1. Parse query                                              │
│  2. Run deterministic queries:                               │
│     - query flex --files "*nickel*"                          │
│     - query co-access (for each hot file)                    │
│     - glob for context packages                              │
│  3. Extract structured data from context packages            │
│  4. Package deterministic data                               │
│                                                              │
└─────────────────────────────────────────────────────────────┘
         │
         │ HTTP POST /api/intel/synthesize-context
         ▼
┌─────────────────────────────────────────────────────────────┐
│              INTELLIGENCE SERVICE (TypeScript + Bun)         │
│                                                              │
│  1. Receive deterministic data                               │
│  2. Call Claude (haiku for medium, sonnet for deep):         │
│     - Generate narrative                                     │
│     - Name work clusters                                     │
│     - Interpret patterns                                     │
│     - Generate insights                                      │
│  3. Return synthesized fields                                │
│                                                              │
└─────────────────────────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────────────────┐
│                    RUST CORE (assembly)                      │
│                                                              │
│  1. Merge deterministic + synthesized data                   │
│  2. Generate receipt_id                                      │
│  3. Cache result                                             │
│  4. Return JSON                                              │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### New Intelligence Service Endpoint

```typescript
// POST /api/intel/synthesize-context

interface SynthesizeContextRequest {
  query: string;
  depth: "quick" | "medium" | "deep";

  // Deterministic data from Rust
  deterministic: {
    files: Array<{
      path: string;
      access_count: number;
      session_count: number;
      last_access: string;
    }>;

    co_access_clusters: Array<{
      seed_file: string;
      related_files: Array<{ path: string; pmi: number }>;
    }>;

    context_packages: Array<{
      path: string;
      title: string;
      content: string;  // Full content for synthesis
    }>;

    timeline_buckets: Array<{
      period: string;
      files: string[];
      session_count: number;
    }>;
  };
}

interface SynthesizeContextResponse {
  executive_summary: { ... };
  current_state: { narrative: string; ... };
  work_clusters: Array<{ name: string; interpretation: string; ... }>;
  timeline: { attention_shift: { ... }; ... };
  insights: Array<{ ... }>;

  synthesis_metadata: {
    model: string;
    tokens_used: number;
    cost_usd: number;
  };
}
```

---

## Cost Model

| Depth | LLM Calls | Estimated Cost | Latency |
|-------|-----------|----------------|---------|
| Quick | 0 | $0 | <500ms |
| Medium | 1 (haiku) | ~$0.0003 | <5s |
| Deep | 2-3 (haiku + sonnet) | ~$0.003 | <15s |

**Budget controls:**
- Daily limit applies (from [[05_INTELLIGENCE_LAYER_ARCHITECTURE]])
- Cache aggressively (context doesn't change frequently)
- Graceful degradation to quick mode if budget exceeded

---

## Quality Metrics

### Actionability

| Metric | Target | Measurement |
|--------|--------|-------------|
| Time to resume work | <2 min | User testing |
| Commands copy-paste ready | 100% | Validation |
| Pending items extracted | >90% | Compare to manual extraction |

### Groundedness

| Metric | Target | Measurement |
|--------|--------|-------------|
| Claims with citations | 100% | Automated check |
| Citations verifiable | >95% | Spot check file:line |
| No hallucinated files | 100% | Validate paths exist |

### Usefulness

| Metric | Target | Measurement |
|--------|--------|-------------|
| Surprise discoveries | >1 per query | Count `surprise: true` |
| Insights actionable | >70% | User feedback |
| Narrative accuracy | >90% | User validation |

---

## Implementation Phases

### Phase 1: Deterministic Foundation (4-6 hours)

Extend Rust core with context restoration command.

**Deliverables:**
1. `tastematter context "<query>"` command
2. Aggregates: flex query + co-access + context package discovery
3. Returns deterministic-only JSON (no synthesis)

**Success criteria:**
- Command returns work_clusters, suggested_reads, quick_start
- All data grounded in hypercube queries

### Phase 2: Intelligence Integration (4-6 hours)

Add synthesis via intelligence service.

**Deliverables:**
1. New `/api/intel/synthesize-context` endpoint
2. Narrative generation for current_state
3. Cluster naming and interpretation
4. Insight generation

**Success criteria:**
- Full JSON schema returned
- Narratives reference specific files/metrics
- Insights surface non-obvious patterns

### Phase 3: Depth Levels + Caching (2-3 hours)

**Deliverables:**
1. `--depth quick|medium|deep` flag
2. Response caching with TTL
3. Budget enforcement

**Success criteria:**
- Quick mode <500ms (no LLM)
- Cache hit <100ms
- Budget exceeded falls back to quick

---

## References

- [[00_VISION]] - "VISIBLE and NAVIGABLE" principle
- [[05_INTELLIGENCE_LAYER_ARCHITECTURE]] - Intelligence service design
- [[07_CLAUDE_CODE_DATA_MODEL]] - JSONL structure for context packages
- [[.claude/skills/context-query/SKILL.md]] - Current query strategies (to be simplified)
- [[.claude/skills/epistemic-context-grounding/SKILL.md]] - Grounding methodology

---

## Expert Perspectives Applied

This specification was developed by applying expert perspectives to CLI/developer tool design:

| Expert | Principle | Application |
|--------|-----------|-------------|
| Charity Majors | "Understand what happened, not just dump data" | Narrative synthesis, interpreted clusters |
| Rich Hickey | "Essential vs accidental complexity" | Hide query strategy, expose meaning |
| Julia Evans | "What's the mental model?" | One command for 80% case |
| Bret Victor | "Immediate feedback" | <5 seconds to insight |
| Dan Abramov | "Beginner understands in 5 minutes" | `tastematter context "X"` is obvious |

---

**Specification Status:** DRAFT
**Created:** 2026-02-03
**Author:** Context restoration design session
**Evidence:** Derived from real CLI usage attempting to restore Nickel context
**Next Action:** Review with stakeholders, then begin Phase 1 implementation
