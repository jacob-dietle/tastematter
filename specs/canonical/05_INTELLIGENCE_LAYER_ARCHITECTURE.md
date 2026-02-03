---
title: "Intelligence Layer Architecture"
type: architecture-spec
created: 2026-01-10
last_updated: 2026-01-25
status: approved
runtime: typescript-bun
foundation:
  - "[[canonical/03_CORE_ARCHITECTURE]]"
  - "[[canonical/00_VISION]]"
  - "[[canonical/01_PRINCIPLES]]"
  - "[[.claude/skills/technical-architecture-engineering/SKILL.md]]"
related:
  - "[[context_packages/20_2026-01-10_QUICK_WINS_COMPLETE]]"
tags:
  - tastematter
  - intelligence-layer
  - claude-agent-sdk
  - architecture
  - canonical
---

# Intelligence Layer Architecture Specification

## Executive Summary

This specification defines the **Intelligence Layer** that transforms Tastematter from a data visualization tool into an intelligent context assistant. The layer integrates Claude Agent SDK to provide:

1. **Intelligent Session Naming** - Chains get meaningful names, not UUIDs
2. **Agent Commit Analysis** - Git commits analyzed for human review
3. **Proactive Insights** - Patterns surfaced before users ask
4. **Session Summaries** - On-demand understanding of any session

**Design Principles:**
- Shared library (works for CLI and Tauri app)
- Lazy evaluation (analyze on first access, cache forever)
- Cost-conscious (model selection based on task complexity)
- Graceful degradation (works without intelligence service)

---

## Problem Statement

### Current State

Tastematter shows raw data without intelligence:

```
Chains Sidebar (Current):
────────────────────────
7f389600  81 sessions  0 files
9fd2c418  23 sessions  0 files
53373094  13 sessions  0 files
```

Users cannot:
- Find work by description ("where's my auth refactor?")
- Understand what agents did (no commit analysis)
- Discover patterns (no proactive insights)
- Get context on sessions (no summaries)

### Vision Gap

From [[00_VISION]]:
> "Effortless (files just appear where they should), Surprising (how did it know to do that?), Trustworthy (but I understand why it did that)"

From [[01_PRINCIPLES]]:
> "STIGMERGIC - Show what agents modified. Enable human response."

**Gap:** No intelligence layer exists to create these experiences.

---

## Target Architecture

### System Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         CONSUMERS                                        │
│                                                                          │
│   ┌─────────────────────┐              ┌─────────────────────┐          │
│   │   Tauri App         │              │   CLI (context-os)  │          │
│   │   (Human UI)        │              │   (Agent Interface) │          │
│   └──────────┬──────────┘              └──────────┬──────────┘          │
│              │ Rust bindings                      │ Direct linking      │
│              └────────────────────┬───────────────┘                     │
└────────────────────────────────────┼────────────────────────────────────┘
                                     │
┌────────────────────────────────────▼────────────────────────────────────┐
│                      context-os-core (Rust)                              │
│                                                                          │
│   ┌────────────────────────────────────────────────────────────────┐    │
│   │                    INTELLIGENCE MODULE (NEW)                    │    │
│   │                                                                 │    │
│   │  ┌─────────────────┐  ┌──────────────────┐  ┌───────────────┐  │    │
│   │  │ IntelClient     │  │ MetadataStore    │  │ CostTracker   │  │    │
│   │  │ (HTTP → Svc)    │  │ (SQLite cache)   │  │ (Budget mgmt) │  │    │
│   │  └────────┬────────┘  └────────┬─────────┘  └───────┬───────┘  │    │
│   │           │                    │                     │          │    │
│   │  ┌────────▼────────────────────▼─────────────────────▼────────┐ │    │
│   │  │                    Intelligence API                         │ │    │
│   │  │                                                             │ │    │
│   │  │  get_chain_metadata(chain_id) -> ChainMetadata             │ │    │
│   │  │  get_commit_analysis(hash) -> CommitAnalysis               │ │    │
│   │  │  get_insights() -> Vec<Insight>                            │ │    │
│   │  │  get_session_summary(id) -> SessionSummary                 │ │    │
│   │  │                                                             │ │    │
│   │  └─────────────────────────────────────────────────────────────┘ │    │
│   └────────────────────────────────────────────────────────────────┘    │
│                                                                          │
│   ┌────────────────────────────────────────────────────────────────┐    │
│   │                    QUERY MODULE (existing)                      │    │
│   │  query_flex, query_timeline, query_sessions, query_chains       │    │
│   └────────────────────────────────────────────────────────────────┘    │
│                                                                          │
│   ┌────────────────────────────────────────────────────────────────┐    │
│   │                    STORAGE MODULE (existing + extended)         │    │
│   │  Database::open() + intelligence metadata tables                │    │
│   └────────────────────────────────────────────────────────────────┘    │
└────────────────────────────────────┬────────────────────────────────────┘
                                     │ HTTP (localhost:3002)
┌────────────────────────────────────▼────────────────────────────────────┐
│                INTELLIGENCE SERVICE (TypeScript + Bun)                   │
│                                                                          │
│   ┌────────────────────────────────────────────────────────────────┐    │
│   │                    Claude Agent SDK                             │    │
│   │  import Anthropic from "@anthropic-ai/sdk";                     │    │
│   └────────────────────────────────────────────────────────────────┘    │
│                                                                          │
│   ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────┐   │
│   │ ChainNaming  │  │ CommitAnalysis│ │ Insights     │  │ Session  │   │
│   │ Agent        │  │ Agent         │ │ Agent        │  │ Summary  │   │
│   │ (haiku)      │  │ (sonnet)      │ │ (sonnet)     │  │ (haiku)  │   │
│   └──────────────┘  └──────────────┘  └──────────────┘  └──────────┘   │
│                                                                          │
│   ┌────────────────────────────────────────────────────────────────┐    │
│   │                    Elysia HTTP Server (Bun)                     │    │
│   │  POST /api/intel/name-chain                                     │    │
│   │  POST /api/intel/analyze-commit                                 │    │
│   │  POST /api/intel/generate-insights                              │    │
│   │  POST /api/intel/summarize-session                              │    │
│   │  GET  /api/intel/health                                         │    │
│   └────────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────────┘
```

### File Structure

```
apps/tastematter/
├── core/                           # Existing Rust CLI
│   └── src/
│       ├── lib.rs                  # + pub mod intelligence
│       ├── intelligence/           # NEW MODULE
│       │   ├── mod.rs              # Module entry
│       │   ├── client.rs           # HTTP client to intel service
│       │   ├── metadata.rs         # SQLite metadata storage
│       │   ├── cost.rs             # Cost tracking
│       │   └── types.rs            # Intelligence types
│       ├── query.rs                # Existing
│       ├── storage.rs              # Extended with metadata tables
│       └── types.rs                # Existing
│
├── intel/                          # NEW SERVICE (TypeScript + Bun)
│   ├── package.json
│   ├── tsconfig.json
│   ├── src/
│   │   ├── index.ts                # Entry + Elysia server
│   │   ├── types/
│   │   │   └── shared.ts           # Zod schemas (match Rust types)
│   │   ├── agents/
│   │   │   ├── chain-naming.ts     # haiku, ~$0.00025/call
│   │   │   ├── commit-analysis.ts  # sonnet, ~$0.003/call
│   │   │   ├── insights.ts         # sonnet, ~$0.003/call
│   │   │   └── session-summary.ts  # haiku, ~$0.00025/call
│   │   ├── client/
│   │   │   └── anthropic.ts        # SDK wrapper + cost tracking
│   │   └── middleware/
│   │       ├── correlation.ts      # X-Correlation-ID propagation
│   │       └── cost-guard.ts       # Budget enforcement
│   └── tests/
│       ├── unit/agents/            # Agent logic tests
│       ├── contract/               # Type parity tests
│       └── integration/            # HTTP endpoint tests
│
├── frontend/                       # Existing Tauri + Svelte
│   ├── src-tauri/
│   │   └── src/
│   │       └── commands.rs         # Extended with intel commands
│   └── src/
│       └── lib/
│           └── stores/
│               └── intelligence.svelte.ts  # NEW store
│
└── specs/                          # Existing specs
```

---

## Design Decisions

### Decision 1: Separate Intelligence Service (TypeScript + Bun)

**Decision:** Run intelligence as a separate TypeScript service using Bun runtime, not embedded in Rust.

**Options Considered:**

| Option | Pros | Cons |
|--------|------|------|
| A. Embed in Rust (FFI) | Single binary | Complex FFI, async issues, no SDK |
| B. Python + FastAPI | Good SDK support | Extra runtime dependency, slower startup |
| C. TypeScript + Bun | Cross-compile to binary (~50MB), fast | Slightly larger binary |
| D. Unix socket IPC | Lower latency | Platform complexity |

**Rationale:**
1. **Bun cross-compile:** Single binary per platform, no runtime required (like Rust)
2. **Agent SDK:** Full `@anthropic-ai/sdk` support with native TypeScript types
3. **HTTP latency:** (~50ms) negligible vs API latency (~2000ms)
4. **Distribution:** Bundle as single executable alongside Rust binary
5. **Development velocity:** TypeScript iteration faster than Rust for AI experiments
6. **Graceful degradation:** Core works without intel service

**Runtime Comparison:**
| Runtime | Startup | Binary Size | Distribution |
|---------|---------|-------------|--------------|
| Python | ~500ms | N/A (needs interpreter) | Complex |
| Bun | ~50ms | ~50MB | Single binary |

**Reference:** [[03_CORE_ARCHITECTURE]] Decision 4 (IPC patterns)

### Decision 2: Lazy Evaluation with Persistent Cache

**Decision:** Analyze on first access, cache forever (with manual refresh option).

**Strategy:**
```
get_chain_metadata(chain_id):
    1. Check SQLite cache
    2. If cached AND not stale → Return cached
    3. If not cached OR forced refresh:
       a. Call intelligence service
       b. Persist to SQLite
       c. Return result
    4. If intel service unavailable → Return None (graceful degradation)
```

**Cache Policy:**

| Data Type | TTL | Rationale |
|-----------|-----|-----------|
| Chain names | ∞ | Chains are immutable once named |
| Commit analysis | ∞ | Commits are immutable |
| Session summaries | ∞ | Sessions are immutable |
| Insights | 6 hours | Data evolves, patterns change |

**Reference:** [[technical-architecture-engineering]] Pattern 3 (Five-Minute Rule)

### Decision 3: Model Selection by Task

**Decision:** Use cheapest model that achieves quality threshold.

**Model Allocation:**

| Task | Model | Cost/Call | Latency | Rationale |
|------|-------|-----------|---------|-----------|
| Chain naming | haiku | ~$0.00025 | ~1s | Simple pattern matching |
| Session summary | haiku | ~$0.00025 | ~1s | Summarization is straightforward |
| Commit analysis | sonnet | ~$0.003 | ~3s | Needs code understanding |
| Insights generation | sonnet | ~$0.003 | ~5s | Complex pattern detection |

**Cost Controls:**
- Daily budget limit (configurable, default $1/day)
- Per-operation cost tracking
- Fallback to cached/none when budget exceeded

### Decision 4: Graceful Degradation

**Decision:** Core functionality works without intelligence service.

**Degradation Levels:**

| Scenario | Behavior |
|----------|----------|
| Intel service running | Full intelligence features |
| Intel service not running | Return cached data or None |
| Budget exceeded | Return cached data or None |
| API rate limited | Queue requests, retry with backoff |

**UI Handling:**
- Show chain ID if no name available
- Show "Analysis pending" if commit not yet analyzed
- Hide insights panel if no insights available

---

## Type Contracts

### Rust Types (tastematter core)

```rust
// apps/tastematter/core/src/intelligence/types.rs

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

// =============================================================================
// CHAIN METADATA
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainMetadata {
    pub chain_id: String,
    pub generated_name: Option<String>,
    pub category: Option<ChainCategory>,
    pub confidence: Option<f32>,
    pub generated_at: Option<DateTime<Utc>>,
    pub model_used: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum ChainCategory {
    BugFix,
    Feature,
    Refactor,
    Research,
    Cleanup,
    Documentation,
    Testing,
    Unknown,
}

// =============================================================================
// COMMIT ANALYSIS
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitAnalysis {
    pub commit_hash: String,
    pub is_agent_commit: bool,
    pub summary: Option<String>,
    pub risk_level: Option<RiskLevel>,
    pub review_focus: Option<String>,
    pub related_files: Vec<String>,
    pub analyzed_at: Option<DateTime<Utc>>,
    pub model_used: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

// =============================================================================
// INSIGHTS
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Insight {
    pub id: String,
    pub insight_type: InsightType,
    pub title: String,
    pub description: String,
    pub evidence: Vec<String>,
    pub action: Option<InsightAction>,
    pub generated_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum InsightType {
    FocusShift,
    CoOccurrence,
    PendingReview,
    Anomaly,
    Continuity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsightAction {
    pub label: String,
    pub action_type: ActionType,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ActionType {
    Navigate,
    Filter,
    External,
}

// =============================================================================
// SESSION SUMMARY
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummaryData {
    pub session_id: String,
    pub summary: String,
    pub key_files: Vec<String>,
    pub focus_area: Option<String>,
    pub generated_at: DateTime<Utc>,
    pub model_used: Option<String>,
}

// =============================================================================
// INTELLIGENCE SERVICE REQUESTS/RESPONSES
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainNamingRequest {
    pub chain_id: String,
    pub files_touched: Vec<String>,
    pub session_count: u32,
    pub recent_sessions: Vec<String>,  // Session IDs for context
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitAnalysisRequest {
    pub commit_hash: String,
    pub message: String,
    pub author: String,
    pub diff: String,
    pub files_changed: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsightsRequest {
    pub time_range: String,
    pub chain_data: Vec<ChainSummaryForInsights>,
    pub file_patterns: Vec<FilePatternForInsights>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainSummaryForInsights {
    pub chain_id: String,
    pub name: Option<String>,
    pub session_count: u32,
    pub file_count: u32,
    pub recent_activity: String,  // ISO timestamp
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilePatternForInsights {
    pub file_path: String,
    pub access_count: u32,
    pub co_accessed_with: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummaryRequest {
    pub session_id: String,
    pub files: Vec<String>,
    pub duration_seconds: Option<u32>,
    pub chain_id: Option<String>,
}

// =============================================================================
// COST TRACKING
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostRecord {
    pub operation: String,
    pub model: String,
    pub cost_usd: f64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostSummary {
    pub today_usd: f64,
    pub budget_usd: f64,
    pub remaining_usd: f64,
    pub operations_today: u32,
}
```

### TypeScript Types (Intelligence Service - Zod)

```typescript
// apps/tastematter/intel/src/types/shared.ts

import { z } from "zod";

// =============================================================================
// ENUMS (match Rust serde(rename_all = "kebab-case"))
// =============================================================================

export const ChainCategorySchema = z.enum([
  "bug-fix",
  "feature",
  "refactor",
  "research",
  "cleanup",
  "documentation",
  "testing",
  "unknown",
]);
export type ChainCategory = z.infer<typeof ChainCategorySchema>;

export const RiskLevelSchema = z.enum(["low", "medium", "high"]);
export type RiskLevel = z.infer<typeof RiskLevelSchema>;

export const InsightTypeSchema = z.enum([
  "focus-shift",
  "co-occurrence",
  "pending-review",
  "anomaly",
  "continuity",
]);
export type InsightType = z.infer<typeof InsightTypeSchema>;

export const ActionTypeSchema = z.enum(["navigate", "filter", "external"]);
export type ActionType = z.infer<typeof ActionTypeSchema>;

// =============================================================================
// CHAIN NAMING
// =============================================================================

export const ChainNamingRequestSchema = z.object({
  chain_id: z.string(),
  files_touched: z.array(z.string()),
  session_count: z.number().int().positive(),
  recent_sessions: z.array(z.string()),
});
export type ChainNamingRequest = z.infer<typeof ChainNamingRequestSchema>;

export const ChainNamingResponseSchema = z.object({
  chain_id: z.string(),
  generated_name: z.string(),
  category: ChainCategorySchema,
  confidence: z.number().min(0).max(1),
  model_used: z.string(),
});
export type ChainNamingResponse = z.infer<typeof ChainNamingResponseSchema>;

// =============================================================================
// COMMIT ANALYSIS
// =============================================================================

export const CommitAnalysisRequestSchema = z.object({
  commit_hash: z.string(),
  message: z.string(),
  author: z.string(),
  diff: z.string(),
  files_changed: z.array(z.string()),
});
export type CommitAnalysisRequest = z.infer<typeof CommitAnalysisRequestSchema>;

export const CommitAnalysisResponseSchema = z.object({
  commit_hash: z.string(),
  is_agent_commit: z.boolean(),
  summary: z.string(),
  risk_level: RiskLevelSchema,
  review_focus: z.string(),
  related_files: z.array(z.string()),
  model_used: z.string(),
});
export type CommitAnalysisResponse = z.infer<typeof CommitAnalysisResponseSchema>;

// =============================================================================
// INSIGHTS
// =============================================================================

export const InsightActionSchema = z.object({
  label: z.string(),
  action_type: ActionTypeSchema,
  payload: z.record(z.unknown()),
});
export type InsightAction = z.infer<typeof InsightActionSchema>;

export const InsightSchema = z.object({
  id: z.string(),
  insight_type: InsightTypeSchema,
  title: z.string(),
  description: z.string(),
  evidence: z.array(z.string()),
  action: InsightActionSchema.nullable(),
});
export type Insight = z.infer<typeof InsightSchema>;

export const InsightsRequestSchema = z.object({
  time_range: z.string(),
  chain_data: z.array(z.object({
    chain_id: z.string(),
    name: z.string().nullable(),
    session_count: z.number().int(),
    file_count: z.number().int(),
    recent_activity: z.string(), // ISO timestamp
  })),
  file_patterns: z.array(z.object({
    file_path: z.string(),
    access_count: z.number().int(),
    co_accessed_with: z.array(z.string()),
  })),
});
export type InsightsRequest = z.infer<typeof InsightsRequestSchema>;

export const InsightsResponseSchema = z.object({
  insights: z.array(InsightSchema),
  model_used: z.string(),
});
export type InsightsResponse = z.infer<typeof InsightsResponseSchema>;

// =============================================================================
// SESSION SUMMARY
// =============================================================================

export const SessionSummaryRequestSchema = z.object({
  session_id: z.string(),
  files: z.array(z.string()),
  duration_seconds: z.number().int().nullable(),
  chain_id: z.string().nullable(),
});
export type SessionSummaryRequest = z.infer<typeof SessionSummaryRequestSchema>;

export const SessionSummaryResponseSchema = z.object({
  session_id: z.string(),
  summary: z.string(),
  key_files: z.array(z.string()),
  focus_area: z.string().nullable(),
  model_used: z.string(),
});
export type SessionSummaryResponse = z.infer<typeof SessionSummaryResponseSchema>;
```

### Type Parity Testing Strategy

```typescript
// apps/tastematter/intel/tests/contract/parity.test.ts

import { describe, test, expect } from "bun:test";
import { ChainNamingResponseSchema, CommitAnalysisResponseSchema } from "../../src/types/shared";

// Load fixtures generated by Rust: cargo test --lib intelligence -- --nocapture
// Fixtures written to: core/tests/fixtures/intel/

describe("Type Parity: Rust ↔ TypeScript", () => {
  test("ChainNamingResponse matches Rust serialization", async () => {
    const rustJson = await Bun.file("../core/tests/fixtures/intel/chain_naming_response.json").json();
    const result = ChainNamingResponseSchema.safeParse(rustJson);
    expect(result.success).toBe(true);
  });

  test("CommitAnalysisResponse matches Rust serialization", async () => {
    const rustJson = await Bun.file("../core/tests/fixtures/intel/commit_analysis_response.json").json();
    const result = CommitAnalysisResponseSchema.safeParse(rustJson);
    expect(result.success).toBe(true);
  });

  // ... similar tests for all types
});
```

---

## Database Schema Extensions

```sql
-- Add to existing tastematter SQLite database
-- apps/tastematter/core/migrations/002_intelligence_metadata.sql

-- Chain metadata (generated names, categories)
CREATE TABLE IF NOT EXISTS chain_metadata (
    chain_id TEXT PRIMARY KEY,
    generated_name TEXT,
    category TEXT,
    confidence REAL,
    generated_at TEXT,  -- ISO 8601 timestamp
    model_used TEXT,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now'))
);

-- Index for quick lookups
CREATE INDEX IF NOT EXISTS idx_chain_metadata_name ON chain_metadata(generated_name);

-- Commit analysis cache
CREATE TABLE IF NOT EXISTS commit_analysis (
    commit_hash TEXT PRIMARY KEY,
    is_agent_commit INTEGER NOT NULL DEFAULT 0,
    summary TEXT,
    risk_level TEXT,
    review_focus TEXT,
    related_files TEXT,  -- JSON array
    analyzed_at TEXT,
    model_used TEXT,
    created_at TEXT DEFAULT (datetime('now'))
);

-- Index for agent commit filtering
CREATE INDEX IF NOT EXISTS idx_commit_analysis_agent ON commit_analysis(is_agent_commit);

-- Session summaries cache
CREATE TABLE IF NOT EXISTS session_summaries (
    session_id TEXT PRIMARY KEY,
    summary TEXT NOT NULL,
    key_files TEXT,  -- JSON array
    focus_area TEXT,
    generated_at TEXT,
    model_used TEXT,
    created_at TEXT DEFAULT (datetime('now'))
);

-- Insights cache (with expiration)
CREATE TABLE IF NOT EXISTS insights_cache (
    id TEXT PRIMARY KEY,
    insight_type TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    evidence TEXT,  -- JSON array
    action TEXT,  -- JSON object
    generated_at TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    model_used TEXT,
    created_at TEXT DEFAULT (datetime('now'))
);

-- Index for expiration cleanup
CREATE INDEX IF NOT EXISTS idx_insights_expires ON insights_cache(expires_at);

-- Cost tracking
CREATE TABLE IF NOT EXISTS intelligence_costs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    operation TEXT NOT NULL,
    model TEXT NOT NULL,
    cost_usd REAL NOT NULL,
    timestamp TEXT DEFAULT (datetime('now'))
);

-- Index for daily cost queries
CREATE INDEX IF NOT EXISTS idx_costs_timestamp ON intelligence_costs(timestamp);

-- Schema version tracking
INSERT OR IGNORE INTO schema_version (version, applied_at, description)
VALUES (2, datetime('now'), 'Intelligence metadata tables');
```

---

## Latency Budgets

Following [[technical-architecture-engineering]] Pattern 1:

### Chain Naming (Target: <3 seconds)

```
Operation                          Budget      %
──────────────────────────────────────────────────
Rust SQLite cache check            10ms       0.3%
Rust → Intel HTTP request          50ms       1.7%
Intel service processing           100ms      3.3%
Claude API (haiku)                 2000ms     66.7%
Response parsing                   50ms       1.7%
Rust SQLite persist                50ms       1.7%
Buffer                             740ms      24.7%
──────────────────────────────────────────────────
TOTAL                              3000ms     100%
```

### Commit Analysis (Target: <6 seconds)

```
Operation                          Budget      %
──────────────────────────────────────────────────
Rust SQLite cache check            10ms       0.2%
Rust → Intel HTTP request          50ms       0.8%
Git diff retrieval                 100ms      1.7%
Intel service processing           100ms      1.7%
Claude API (sonnet)                4500ms     75.0%
Response parsing                   50ms       0.8%
Rust SQLite persist                50ms       0.8%
Buffer                             1140ms     19.0%
──────────────────────────────────────────────────
TOTAL                              6000ms     100%
```

### Insights Generation (Target: <10 seconds)

```
Operation                          Budget      %
──────────────────────────────────────────────────
Rust data aggregation              200ms      2.0%
Rust → Intel HTTP request          50ms       0.5%
Intel service processing           200ms      2.0%
Claude API (sonnet)                7000ms     70.0%
Response parsing                   100ms      1.0%
Rust SQLite persist                50ms       0.5%
Buffer                             2400ms     24.0%
──────────────────────────────────────────────────
TOTAL                              10000ms    100%
```

---

## Agent Definitions

### Chain Naming Agent

```typescript
// apps/tastematter/intel/src/agents/chain-naming.ts

import Anthropic from "@anthropic-ai/sdk";
import type { ChainNamingRequest, ChainNamingResponse } from "../types/shared";

const CHAIN_NAMING_PROMPT = `You are a session naming specialist. Given information about a conversation chain:

INPUT:
- chain_id: The unique identifier
- files_touched: List of file paths touched across all sessions
- session_count: Number of sessions in this chain
- recent_sessions: IDs of recent sessions for context

OUTPUT (JSON):
{
  "generated_name": "Short descriptive name (3-6 words)",
  "category": "bug-fix|feature|refactor|research|cleanup|documentation|testing|unknown",
  "confidence": 0.0-1.0
}

NAMING RULES:
1. Be SPECIFIC. "Fixed bug" is bad. "Fixed auth redirect loop" is good.
2. Use file paths to infer context:
   - Files in "tests/" → likely testing
   - Files in "docs/" → likely documentation
   - Mix of src and test → likely feature or refactor
3. If files are from multiple unrelated areas, use the dominant theme.
4. If unclear, set confidence < 0.5 and use "unknown" category.

EXAMPLES:
- Files: [auth.py, login.py, tests/test_auth.py] → "Fixed authentication flow" (bug-fix, 0.9)
- Files: [README.md, CHANGELOG.md] → "Updated documentation" (documentation, 0.95)
- Files: [many disparate files] → "General codebase work" (unknown, 0.3)`;

export async function nameChain(
  client: Anthropic,
  request: ChainNamingRequest
): Promise<ChainNamingResponse> {
  const response = await client.messages.create({
    model: "claude-3-5-haiku-latest", // Fast and cheap
    max_tokens: 256,
    messages: [
      {
        role: "user",
        content: `${CHAIN_NAMING_PROMPT}\n\nINPUT:\n${JSON.stringify(request, null, 2)}`,
      },
    ],
  });

  const text = response.content[0].type === "text" ? response.content[0].text : "";
  const parsed = JSON.parse(text);

  return {
    chain_id: request.chain_id,
    generated_name: parsed.generated_name,
    category: parsed.category,
    confidence: parsed.confidence,
    model_used: "claude-3-5-haiku-latest",
  };
}
```

### Commit Analysis Agent

```typescript
// apps/tastematter/intel/src/agents/commit-analysis.ts

import Anthropic from "@anthropic-ai/sdk";
import type { CommitAnalysisRequest, CommitAnalysisResponse } from "../types/shared";

const COMMIT_ANALYSIS_PROMPT = `You are a code review assistant. Analyze git commits for humans.

INPUT:
- commit_hash: The commit SHA
- message: Commit message
- author: Author name/email
- diff: Full git diff
- files_changed: List of changed files

OUTPUT (JSON):
{
  "is_agent_commit": true/false,
  "summary": "Plain English summary (1-2 sentences)",
  "risk_level": "low|medium|high",
  "review_focus": "What a human reviewer should check",
  "related_files": ["files that might also need updates"]
}

AGENT DETECTION:
- Check for "Co-Authored-By: Claude" in commit message or diff
- Check for author containing "claude" or "anthropic"
- If either present → is_agent_commit: true

RISK ASSESSMENT:
- LOW: Documentation, tests, comments, small refactors, config tweaks
- MEDIUM: New features, dependency updates, config changes affecting behavior
- HIGH: Authentication, authorization, payments, data migrations, security

REVIEW FOCUS:
- What's the most important thing to verify?
- Any obvious issues or concerns?
- What tests should be run?

RELATED FILES:
- If modifying X.py, often need to update test_X.py
- If changing API, often need to update docs
- If changing schema, often need migration`;

export async function analyzeCommit(
  client: Anthropic,
  request: CommitAnalysisRequest
): Promise<CommitAnalysisResponse> {
  const response = await client.messages.create({
    model: "claude-sonnet-4-20250514", // Better code understanding
    max_tokens: 1024,
    messages: [
      {
        role: "user",
        content: `${COMMIT_ANALYSIS_PROMPT}\n\nINPUT:\n${JSON.stringify(request, null, 2)}`,
      },
    ],
  });

  const text = response.content[0].type === "text" ? response.content[0].text : "";
  const parsed = JSON.parse(text);

  return {
    commit_hash: request.commit_hash,
    is_agent_commit: parsed.is_agent_commit,
    summary: parsed.summary,
    risk_level: parsed.risk_level,
    review_focus: parsed.review_focus,
    related_files: parsed.related_files,
    model_used: "claude-sonnet-4-20250514",
  };
}
```

### Insights Agent

```typescript
// apps/tastematter/intel/src/agents/insights.ts

import Anthropic from "@anthropic-ai/sdk";
import type { InsightsRequest, InsightsResponse } from "../types/shared";

const INSIGHTS_PROMPT = `You are a work pattern analyst. Surface surprising and actionable insights.

INPUT:
- time_range: Period being analyzed (e.g., "7d")
- chain_data: List of chains with activity stats
- file_patterns: File access patterns with co-occurrence data

OUTPUT (JSON):
{
  "insights": [
    {
      "id": "unique-id",
      "insight_type": "focus-shift|co-occurrence|pending-review|anomaly|continuity",
      "title": "Short title (5-8 words)",
      "description": "Explanation with specific numbers",
      "evidence": ["Specific data points backing this up"],
      "action": {
        "label": "Button text",
        "action_type": "navigate|filter|external",
        "payload": {...}
      }
    }
  ]
}

INSIGHT TYPES:

1. FOCUS-SHIFT: Significant change in where attention is going
   - Compare this period to previous
   - >20% shift is significant
   - Example: "45% of activity on project X (was 20% last week)"

2. CO-OCCURRENCE: Files that always change together
   - >80% co-occurrence is significant
   - Suggests a "module" or tight coupling
   - Example: "config.yaml and deploy.sh change together 95% of the time"

3. PENDING-REVIEW: Agent commits not yet reviewed
   - Agent commits >24 hours old without human commits after
   - Example: "3 agent commits from yesterday pending review"

4. ANOMALY: Unusual patterns
   - >2x normal activity
   - Activity at unusual times
   - Example: "3x normal file access yesterday - investigation?"

5. CONTINUITY: This work continues previous work
   - Same files as recent chain
   - Similar patterns
   - Example: "Looks like continued work on auth refactor (chain X)"

QUALITY RULES:
- Max 5 insights (quality over quantity)
- Only report patterns with strong evidence
- Include specific numbers, not vague claims
- Every insight should be actionable`;

export async function generateInsights(
  client: Anthropic,
  request: InsightsRequest
): Promise<InsightsResponse> {
  const response = await client.messages.create({
    model: "claude-sonnet-4-20250514",
    max_tokens: 2048,
    messages: [
      {
        role: "user",
        content: `${INSIGHTS_PROMPT}\n\nINPUT:\n${JSON.stringify(request, null, 2)}`,
      },
    ],
  });

  const text = response.content[0].type === "text" ? response.content[0].text : "";
  const parsed = JSON.parse(text);

  return {
    insights: parsed.insights,
    model_used: "claude-sonnet-4-20250514",
  };
}
```

### Session Summary Agent

```typescript
// apps/tastematter/intel/src/agents/session-summary.ts

import Anthropic from "@anthropic-ai/sdk";
import type { SessionSummaryRequest, SessionSummaryResponse } from "../types/shared";

const SESSION_SUMMARY_PROMPT = `You are a session summarizer. Create brief, useful summaries.

INPUT:
- session_id: Session identifier
- files: List of files touched in session
- duration_seconds: How long the session lasted
- chain_id: Parent chain (for context)

OUTPUT (JSON):
{
  "summary": "One-line summary of what happened",
  "key_files": ["top 3 most important files"],
  "focus_area": "infrastructure|frontend|backend|testing|docs|unknown"
}

SUMMARIZATION RULES:
1. Be concise - one sentence max
2. Focus on WHAT was done, not technical details
3. Infer from file paths:
   - src/api/* → "API work"
   - tests/* → "Testing"
   - docs/* → "Documentation"
   - config/* → "Configuration"
4. If session is short (<5 min) and few files, might be "Quick edit" or "Review"
5. If many files across areas, might be "Cross-cutting refactor"

EXAMPLES:
- Files: [auth.py, login.vue, tests/test_auth.py] → "Updated authentication flow"
- Files: [README.md] → "Documentation update"
- Files: [50+ files] → "Large-scale refactoring"`;

export async function summarizeSession(
  client: Anthropic,
  request: SessionSummaryRequest
): Promise<SessionSummaryResponse> {
  const response = await client.messages.create({
    model: "claude-3-5-haiku-latest", // Fast for simple summarization
    max_tokens: 256,
    messages: [
      {
        role: "user",
        content: `${SESSION_SUMMARY_PROMPT}\n\nINPUT:\n${JSON.stringify(request, null, 2)}`,
      },
    ],
  });

  const text = response.content[0].type === "text" ? response.content[0].text : "";
  const parsed = JSON.parse(text);

  return {
    session_id: request.session_id,
    summary: parsed.summary,
    key_files: parsed.key_files,
    focus_area: parsed.focus_area,
    model_used: "claude-3-5-haiku-latest",
  };
}
```

---

## API Endpoints

### Intelligence Service API (Elysia + Bun)

```typescript
// apps/tastematter/intel/src/index.ts

import { Elysia, t } from "elysia";
import Anthropic from "@anthropic-ai/sdk";
import { nameChain } from "./agents/chain-naming";
import { analyzeCommit } from "./agents/commit-analysis";
import { generateInsights } from "./agents/insights";
import { summarizeSession } from "./agents/session-summary";
import {
  ChainNamingRequestSchema,
  CommitAnalysisRequestSchema,
  InsightsRequestSchema,
  SessionSummaryRequestSchema,
} from "./types/shared";
import { correlationMiddleware } from "./middleware/correlation";
import { costGuardMiddleware } from "./middleware/cost-guard";

const client = new Anthropic();

const app = new Elysia()
  .use(correlationMiddleware)
  .use(costGuardMiddleware)
  .get("/api/intel/health", () => ({
    status: "ok",
    version: "0.1.0",
  }))
  .post(
    "/api/intel/name-chain",
    async ({ body }) => {
      const request = ChainNamingRequestSchema.parse(body);
      return await nameChain(client, request);
    },
    {
      body: t.Object({
        chain_id: t.String(),
        files_touched: t.Array(t.String()),
        session_count: t.Number(),
        recent_sessions: t.Array(t.String()),
      }),
    }
  )
  .post(
    "/api/intel/analyze-commit",
    async ({ body }) => {
      const request = CommitAnalysisRequestSchema.parse(body);
      return await analyzeCommit(client, request);
    },
    {
      body: t.Object({
        commit_hash: t.String(),
        message: t.String(),
        author: t.String(),
        diff: t.String(),
        files_changed: t.Array(t.String()),
      }),
    }
  )
  .post(
    "/api/intel/generate-insights",
    async ({ body }) => {
      const request = InsightsRequestSchema.parse(body);
      return await generateInsights(client, request);
    },
    {
      body: t.Object({
        time_range: t.String(),
        chain_data: t.Array(t.Any()),
        file_patterns: t.Array(t.Any()),
      }),
    }
  )
  .post(
    "/api/intel/summarize-session",
    async ({ body }) => {
      const request = SessionSummaryRequestSchema.parse(body);
      return await summarizeSession(client, request);
    },
    {
      body: t.Object({
        session_id: t.String(),
        files: t.Array(t.String()),
        duration_seconds: t.Nullable(t.Number()),
        chain_id: t.Nullable(t.String()),
      }),
    }
  )
  .listen(3002);

console.log(`🔮 Intelligence service running on http://localhost:${app.server?.port}`);

export type App = typeof app;
```

### Middleware: Correlation ID

```typescript
// apps/tastematter/intel/src/middleware/correlation.ts

import { Elysia } from "elysia";
import { randomUUID } from "crypto";

export const correlationMiddleware = new Elysia({ name: "correlation" })
  .derive(({ request }) => {
    const correlationId = request.headers.get("x-correlation-id") ?? randomUUID();
    return { correlationId };
  })
  .onAfterHandle(({ correlationId, set }) => {
    set.headers["x-correlation-id"] = correlationId;
  });
```

### Middleware: Cost Guard

```typescript
// apps/tastematter/intel/src/middleware/cost-guard.ts

import { Elysia } from "elysia";

// Simple in-memory cost tracking (persisted via Rust SQLite in production)
let todayCostUsd = 0;
const DAILY_BUDGET_USD = 1.0;

export const costGuardMiddleware = new Elysia({ name: "cost-guard" })
  .derive(() => {
    const remainingBudget = DAILY_BUDGET_USD - todayCostUsd;
    return { remainingBudget };
  })
  .onBeforeHandle(({ remainingBudget, set, path }) => {
    // Skip health endpoint
    if (path === "/api/intel/health") return;

    if (remainingBudget <= 0) {
      set.status = 429;
      return { error: "Daily budget exceeded", remaining_usd: 0 };
    }
  });

export function recordCost(costUsd: number) {
  todayCostUsd += costUsd;
}

export function resetDailyCost() {
  todayCostUsd = 0;
}
```

---

## Implementation Phases

### Phase 1: TypeScript Foundation (4-6 hours)

**Deliverables:**
1. `apps/tastematter/intel/` package structure
2. Elysia server with health endpoint
3. Zod schemas matching Rust types
4. Correlation ID middleware
5. Contract tests with fixtures

**Success Criteria:**
- `bun install && bun run src/index.ts` starts server on :3002
- `/api/intel/health` returns OK
- Contract tests pass against Rust fixtures
- TypeScript types compile cleanly

**Commands:**
```bash
cd apps/tastematter/intel
bun install
bun run src/index.ts  # Dev mode
bun test              # Run tests
```

### Phase 2: Chain Naming Agent (3-4 hours)

**Deliverables:**
1. Chain naming agent with Anthropic SDK
2. Cost tracking wrapper
3. Unit tests for agent logic
4. Integration tests for endpoint

**Success Criteria:**
- `/api/intel/name-chain` generates names
- Costs tracked per operation
- Unit tests pass
- Integration tests pass

### Phase 3: Rust IntelClient (4-6 hours)

**Deliverables:**
1. `intelligence/` module in core
2. `IntelClient` with reqwest
3. SQLite schema migration (5 tables)
4. `MetadataStore` cache layer
5. Graceful degradation

**Success Criteria:**
- `cargo build` succeeds with new module
- `get_chain_metadata(chain_id)` works end-to-end
- Names cached in SQLite
- CLI: `tastematter intel name-chain <id>` works
- Works without intel service (graceful degradation)

### Phase 4: Remaining Agents (4-6 hours)

**Deliverables:**
1. Commit Analysis Agent (sonnet)
2. Insights Agent (sonnet)
3. Session Summary Agent (haiku)
4. Integration tests for all endpoints

**Success Criteria:**
- All 4 endpoints return valid responses
- Cost tracking works for all operations
- Agent commit detection working
- Risk levels assigned correctly

### Phase 5: Build Pipeline (2-3 hours)

**Deliverables:**
1. Bun compile configuration for all platforms
2. GitHub Actions for 4-platform builds
3. Combined installer scripts
4. Release workflow

**Success Criteria:**
- Cross-platform binaries build:
  ```bash
  bun build src/index.ts --compile --target=bun-darwin-x64 --outfile=dist/tastematter-intel-darwin-x64
  bun build src/index.ts --compile --target=bun-darwin-arm64 --outfile=dist/tastematter-intel-darwin-arm64
  bun build src/index.ts --compile --target=bun-linux-x64 --outfile=dist/tastematter-intel-linux-x64
  bun build src/index.ts --compile --target=bun-windows-x64 --outfile=dist/tastematter-intel-win32-x64.exe
  ```
- Combined release package includes both Rust + TypeScript binaries
- Install scripts work on all platforms

### Phase 6: Parity & E2E Tests (2-3 hours)

**Deliverables:**
1. Rust JSON fixtures generated in CI
2. Contract tests validate TypeScript against Rust
3. E2E tests with mocked Claude calls

**Success Criteria:**
- All contract tests pass (Zod validates Rust JSON)
- E2E test suite covers all workflows
- CI runs all test suites

**Total Estimated: 19-28 hours**

---

## Success Metrics

### Performance

| Operation | Target | Rationale |
|-----------|--------|-----------|
| Chain naming | <3s | Interactive feel |
| Commit analysis | <6s | Background acceptable |
| Insights generation | <10s | Background, cached |
| Cache hit | <50ms | Local SQLite |

### Quality

| Metric | Target | Measurement |
|--------|--------|-------------|
| Chain name relevance | >80% accurate | User feedback |
| Risk level accuracy | >90% correct | Manual audit |
| Insight actionability | >70% useful | User engagement |

### Cost

| Metric | Target | Rationale |
|--------|--------|-----------|
| Daily budget | <$1/day | Sustainable for personal use |
| Cost per chain name | ~$0.00025 | Haiku is cheap |
| Cost per commit analysis | ~$0.003 | Sonnet for quality |

---

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Intel service down | Graceful degradation - show raw IDs |
| API rate limits | Retry with backoff, cache aggressively |
| Budget exceeded | Fallback to cached, notify user |
| Model quality issues | Human feedback loop, model versioning |
| Latency spikes | Timeouts, async where possible |

---

## References

- [[03_CORE_ARCHITECTURE]] - Existing architecture this extends
- [[00_VISION]] - "Surprising, Effortless, Trustworthy"
- [[01_PRINCIPLES]] - STIGMERGIC principle
- [[technical-architecture-engineering]] - Latency budgets, Five-Minute Rule
- [[specification-driven-development]] - Spec-first methodology
- Anthropic SDK docs - https://docs.anthropic.com/en/api/client-sdks
- Bun compile docs - https://bun.sh/docs/bundler/executables
- Elysia docs - https://elysiajs.com/
- Zod docs - https://zod.dev/

---

**Specification Status:** APPROVED
**Runtime:** TypeScript + Bun + Elysia
**Created:** 2026-01-10
**Last Updated:** 2026-01-25
**Author:** Architecture planning session
**Decision Record:**
- 2026-01-25: Approved TypeScript + Bun over Python (cross-compile, ~50MB binary)
- 2026-01-25: Approved Anthropic SDK (`@anthropic-ai/sdk`) for agent capabilities
- 2026-01-25: Approved Rust spawns TypeScript service (coordinated daemon lifecycle)
**Next Action:** Begin Phase 1 - TypeScript Foundation
