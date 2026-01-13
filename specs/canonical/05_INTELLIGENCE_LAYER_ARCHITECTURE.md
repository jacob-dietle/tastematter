---
title: "Intelligence Layer Architecture"
type: architecture-spec
created: 2026-01-10
last_updated: 2026-01-10
status: draft
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
│                    INTELLIGENCE SERVICE (Python)                         │
│                                                                          │
│   ┌────────────────────────────────────────────────────────────────┐    │
│   │                    Claude Agent SDK                             │    │
│   │  from claude_agent_sdk import query, ClaudeAgentOptions         │    │
│   └────────────────────────────────────────────────────────────────┘    │
│                                                                          │
│   ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────┐   │
│   │ ChainNaming  │  │ CommitAnalysis│ │ Insights     │  │ Session  │   │
│   │ Agent        │  │ Agent         │ │ Agent        │  │ Summary  │   │
│   │ (haiku)      │  │ (sonnet)      │ │ (sonnet)     │  │ (haiku)  │   │
│   └──────────────┘  └──────────────┘  └──────────────┘  └──────────┘   │
│                                                                          │
│   ┌────────────────────────────────────────────────────────────────┐    │
│   │                    FastAPI HTTP Server                          │    │
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
apps/
├── context-os/
│   └── core/
│       └── src/
│           ├── lib.rs              # + pub mod intelligence
│           ├── intelligence/       # NEW MODULE
│           │   ├── mod.rs          # Module entry
│           │   ├── client.rs       # HTTP client to intel service
│           │   ├── metadata.rs     # SQLite metadata storage
│           │   ├── cost.rs         # Cost tracking
│           │   └── types.rs        # Intelligence types
│           ├── query.rs            # Existing
│           ├── storage.rs          # Extended with metadata tables
│           └── types.rs            # Existing
│
├── context-os-intel/               # NEW SERVICE
│   ├── pyproject.toml
│   ├── src/
│   │   └── context_os_intel/
│   │       ├── __init__.py
│   │       ├── server.py           # FastAPI server
│   │       ├── agents/
│   │       │   ├── __init__.py
│   │       │   ├── chain_naming.py
│   │       │   ├── commit_analysis.py
│   │       │   ├── insights.py
│   │       │   └── session_summary.py
│   │       └── types.py            # Pydantic models
│   └── tests/
│       └── test_agents.py
│
└── tastematter/
    ├── src-tauri/
    │   └── src/
    │       └── commands.rs         # Extended with intel commands
    └── src/
        └── lib/
            └── stores/
                └── intelligence.svelte.ts  # NEW store
```

---

## Design Decisions

### Decision 1: Separate Intelligence Service (Python)

**Decision:** Run intelligence as a separate Python service, not embedded in Rust.

**Options Considered:**

| Option | Pros | Cons |
|--------|------|------|
| A. Embed in Rust (PyO3) | Single binary | Complex FFI, async issues |
| B. Separate service (HTTP) | Clean separation, easy to develop | Network latency (~50ms) |
| C. Unix socket IPC | Lower latency | Platform complexity |

**Rationale:**
1. Claude Agent SDK is Python/TypeScript - FFI to Rust is complex
2. HTTP latency (~50ms) is negligible vs API latency (~2000ms)
3. Separate service can be developed/deployed independently
4. Graceful degradation: core works without intel service

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

### Rust Types (context-os-core)

```rust
// apps/context-os/core/src/intelligence/types.rs

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

### Python Types (Intelligence Service)

```python
# apps/context-os-intel/src/context_os_intel/types.py

from dataclasses import dataclass
from enum import Enum
from typing import Optional, List
from datetime import datetime

class ChainCategory(str, Enum):
    BUG_FIX = "bug-fix"
    FEATURE = "feature"
    REFACTOR = "refactor"
    RESEARCH = "research"
    CLEANUP = "cleanup"
    DOCUMENTATION = "documentation"
    TESTING = "testing"
    UNKNOWN = "unknown"

class RiskLevel(str, Enum):
    LOW = "low"
    MEDIUM = "medium"
    HIGH = "high"

class InsightType(str, Enum):
    FOCUS_SHIFT = "focus-shift"
    CO_OCCURRENCE = "co-occurrence"
    PENDING_REVIEW = "pending-review"
    ANOMALY = "anomaly"
    CONTINUITY = "continuity"

class ActionType(str, Enum):
    NAVIGATE = "navigate"
    FILTER = "filter"
    EXTERNAL = "external"

# Request/Response models (Pydantic for FastAPI)
from pydantic import BaseModel

class ChainNamingRequest(BaseModel):
    chain_id: str
    files_touched: List[str]
    session_count: int
    recent_sessions: List[str]

class ChainNamingResponse(BaseModel):
    chain_id: str
    generated_name: str
    category: ChainCategory
    confidence: float
    model_used: str

class CommitAnalysisRequest(BaseModel):
    commit_hash: str
    message: str
    author: str
    diff: str
    files_changed: List[str]

class CommitAnalysisResponse(BaseModel):
    commit_hash: str
    is_agent_commit: bool
    summary: str
    risk_level: RiskLevel
    review_focus: str
    related_files: List[str]
    model_used: str

class InsightAction(BaseModel):
    label: str
    action_type: ActionType
    payload: dict

class Insight(BaseModel):
    id: str
    insight_type: InsightType
    title: str
    description: str
    evidence: List[str]
    action: Optional[InsightAction]

class InsightsResponse(BaseModel):
    insights: List[Insight]
    model_used: str

class SessionSummaryRequest(BaseModel):
    session_id: str
    files: List[str]
    duration_seconds: Optional[int]
    chain_id: Optional[str]

class SessionSummaryResponse(BaseModel):
    session_id: str
    summary: str
    key_files: List[str]
    focus_area: Optional[str]
    model_used: str
```

---

## Database Schema Extensions

```sql
-- Add to existing context-os SQLite database
-- apps/context-os/core/migrations/002_intelligence_metadata.sql

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

```python
# apps/context-os-intel/src/context_os_intel/agents/chain_naming.py

from claude_agent_sdk import AgentDefinition

CHAIN_NAMING_AGENT = AgentDefinition(
    description="Analyzes Claude Code sessions and generates meaningful chain names",
    prompt="""You are a session naming specialist. Given information about a conversation chain:

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
- Files: [many disparate files] → "General codebase work" (unknown, 0.3)
""",
    tools=[],  # No tools needed - pure analysis
    model="haiku"  # Fast and cheap
)
```

### Commit Analysis Agent

```python
# apps/context-os-intel/src/context_os_intel/agents/commit_analysis.py

from claude_agent_sdk import AgentDefinition

COMMIT_ANALYSIS_AGENT = AgentDefinition(
    description="Analyzes git commits for human-readable summaries and risk assessment",
    prompt="""You are a code review assistant. Analyze git commits for humans.

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
- If changing schema, often need migration
""",
    tools=["Read"],  # Can read files for context
    model="sonnet"  # Better code understanding
)
```

### Insights Agent

```python
# apps/context-os-intel/src/context_os_intel/agents/insights.py

from claude_agent_sdk import AgentDefinition

INSIGHTS_AGENT = AgentDefinition(
    description="Analyzes context data for patterns and actionable insights",
    prompt="""You are a work pattern analyst. Surface surprising and actionable insights.

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
- Every insight should be actionable
""",
    tools=["Read", "Bash"],  # Can read files and run queries
    model="sonnet"
)
```

### Session Summary Agent

```python
# apps/context-os-intel/src/context_os_intel/agents/session_summary.py

from claude_agent_sdk import AgentDefinition

SESSION_SUMMARY_AGENT = AgentDefinition(
    description="Summarizes what happened in a Claude Code session",
    prompt="""You are a session summarizer. Create brief, useful summaries.

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
- Files: [50+ files] → "Large-scale refactoring"
""",
    tools=[],  # No tools needed
    model="haiku"  # Fast for simple summarization
)
```

---

## API Endpoints

### Intelligence Service API

```python
# apps/context-os-intel/src/context_os_intel/server.py

from fastapi import FastAPI, HTTPException
from .types import *
from .agents import (
    chain_naming,
    commit_analysis,
    insights,
    session_summary
)

app = FastAPI(title="Context OS Intelligence Service")

@app.get("/api/intel/health")
async def health():
    return {"status": "ok", "version": "0.1.0"}

@app.post("/api/intel/name-chain", response_model=ChainNamingResponse)
async def name_chain(request: ChainNamingRequest):
    """Generate a meaningful name for a conversation chain."""
    result = await chain_naming.analyze(request)
    return result

@app.post("/api/intel/analyze-commit", response_model=CommitAnalysisResponse)
async def analyze_commit(request: CommitAnalysisRequest):
    """Analyze a git commit for summary and risk assessment."""
    result = await commit_analysis.analyze(request)
    return result

@app.post("/api/intel/generate-insights", response_model=InsightsResponse)
async def generate_insights(request: InsightsRequest):
    """Generate proactive insights from context data."""
    result = await insights.analyze(request)
    return result

@app.post("/api/intel/summarize-session", response_model=SessionSummaryResponse)
async def summarize_session(request: SessionSummaryRequest):
    """Generate a summary of a session."""
    result = await session_summary.analyze(request)
    return result
```

---

## Implementation Phases

### Phase 1: Foundation (4-6 hours)

**Deliverables:**
1. `intelligence/` module structure in context-os-core
2. SQLite schema extensions (metadata tables)
3. Basic types and traits
4. Graceful degradation (works without intel service)

**Success Criteria:**
- `cargo build` succeeds with new module
- Schema migration runs on startup
- Existing functionality unaffected
- Tests pass

### Phase 2: Intelligence Service (6-8 hours)

**Deliverables:**
1. `context-os-intel/` Python project
2. FastAPI server with health endpoint
3. Chain Naming Agent implementation
4. Session Summary Agent implementation

**Success Criteria:**
- `uvicorn context_os_intel.server:app --port 3002` starts
- `/api/intel/health` returns OK
- `/api/intel/name-chain` generates names
- Unit tests for agents pass

### Phase 3: Rust Integration (4-6 hours)

**Deliverables:**
1. HTTP client in Rust (reqwest)
2. Metadata storage layer
3. Cost tracking
4. Integration with query_chains

**Success Criteria:**
- `get_chain_metadata(chain_id)` works end-to-end
- Names cached in SQLite
- Cost tracked per operation
- CLI: `context-os intel name-chain <id>` works

### Phase 4: Commit Analysis (4-6 hours)

**Deliverables:**
1. Commit Analysis Agent
2. Git integration (git2 crate in Rust)
3. Agent commit detection
4. CLI and Tauri commands

**Success Criteria:**
- `/api/intel/analyze-commit` works
- Agent commits detected correctly
- Risk levels assigned appropriately
- Cache works correctly

### Phase 5: Insights & Frontend (6-8 hours)

**Deliverables:**
1. Insights Agent
2. Frontend intelligence store
3. Sidebar with chain names
4. Insights panel component

**Success Criteria:**
- Insights generated on demand
- Sidebar shows chain names (not IDs)
- Insights panel displays patterns
- Full end-to-end flow working

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
- Claude Agent SDK docs - https://platform.claude.com/docs/en/agent-sdk/overview

---

**Specification Status:** DRAFT
**Created:** 2026-01-10
**Author:** Architecture planning session
**Next Action:** Review with user, then begin Phase 1 implementation
