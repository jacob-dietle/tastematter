---
title: "Tastematter Context Package 21"
package_number: 21

migrated_from: "apps/tastematter/specs/context_packages/21_2026-01-10_INTELLIGENCE_LAYER_SPEC.md"
status: current
previous_package: "[[20_2026-01-10_QUICK_WINS_COMPLETE]]"
related:
  - "[[specs/canonical/05_INTELLIGENCE_LAYER_ARCHITECTURE.md]]"
  - "[[specs/canonical/03_CORE_ARCHITECTURE.md]]"
  - "[[specs/canonical/00_VISION.md]]"
  - "[[specs/canonical/02_ROADMAP.md]]"
tags:
  - context-package
  - tastematter
  - intelligence-layer
  - architecture
  - claude-agent-sdk
---

# Tastematter - Context Package 21

## Executive Summary

**Intelligence Layer Architecture: SPEC COMPLETE.** Comprehensive specification created for transforming Tastematter from data visualization tool to intelligent context assistant. Researched Claude Agent SDK via Context7, designed two-service architecture (Rust core + Python intelligence service), defined type contracts, agent prompts, and 5-phase implementation plan.

## Global Context

### The Problem

Tastematter shows raw data without intelligence:
- Chain IDs like "7f389600" instead of meaningful names
- No visibility into what agents did (git commits)
- No proactive insights or pattern detection
- App feels like a "toy" - needs to feel "production excellent"

### Vision Gap Analysis

From [[00_VISION]]:
> "Effortless, Surprising, Trustworthy"

| Vision Promise | Current State | Gap |
|----------------|---------------|-----|
| "Effortless" | User scans 50 raw paths | No hierarchy, no grouping |
| "Surprising" | Static data display | No intelligence, no insights |
| "Trustworthy" | Error in sidebar, unexplained IDs | Broken, opaque |

### Solution: Intelligence Layer

Two-service architecture enabling Claude Agent SDK integration:

```
┌─────────────────────────────────────────────────────────────┐
│                  context-os-core (Rust)                      │
│  ┌────────────────────────────────────────────────────────┐ │
│  │  INTELLIGENCE MODULE (NEW)                              │ │
│  │  - IntelClient (HTTP → service)                        │ │
│  │  - MetadataStore (SQLite cache)                        │ │
│  │  - CostTracker (budget mgmt)                           │ │
│  └────────────────────────────────────────────────────────┘ │
└───────────────────────────┬─────────────────────────────────┘
                            │ HTTP :3002
┌───────────────────────────▼─────────────────────────────────┐
│              INTELLIGENCE SERVICE (Python)                   │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐       │
│  │ChainName │ │CommitAnal│ │ Insights │ │SessionSum│       │
│  │(haiku)   │ │(sonnet)  │ │(sonnet)  │ │(haiku)   │       │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘       │
└─────────────────────────────────────────────────────────────┘
```

## Local Problem Set

### Completed This Session

**1. Context Foundation Loading**
- [X] Loaded context from package 20 [VERIFIED: read_page shows app running]
- [X] Verified 246 tests still passing [VERIFIED: pnpm test:unit output]
- [X] Confirmed Timeline/Sessions views work with real data (not simulated) [VERIFIED: curl API endpoints]

**2. Browser Testing Attempt**
- [X] Attempted Chrome testing via MCP tools
- [X] Discovered extension conflict blocking clicks/screenshots
- [X] read_page still works - verified app state via accessibility tree
- [X] Confirmed: Git Status shows error in HTTP mode (expected)

**3. Gap Analysis**
- [X] Reviewed vision docs, principles, roadmap [VERIFIED: [[00_VISION]], [[01_PRINCIPLES]], [[02_ROADMAP]]]
- [X] Identified principle compliance: 2/5 passing (IMMEDIATE, INVESTMENT NOT RENT)
- [X] Mapped what's spec'd vs implemented [VERIFIED: [[02_ROADMAP]] analysis]

**4. Claude Agent SDK Research**
- [X] Used Context7 MCP to research SDK capabilities [VERIFIED: 3 query-docs calls]
- [X] Documented: Session management, hooks, message types, subagents
- [X] Identified 4 intelligence features: Chain Naming, Commit Analysis, Insights, Session Summary

**5. Intelligence Layer Architecture Spec**
- [X] Created comprehensive spec: [[05_INTELLIGENCE_LAYER_ARCHITECTURE.md]] [VERIFIED: file created]
- [X] Defined 4 key design decisions with rationale
- [X] Created type contracts (Rust + Python sides)
- [X] Wrote 4 agent definitions with prompts
- [X] Defined API endpoints (FastAPI)
- [X] Set latency budgets (<3s chain naming, <6s commit analysis, <10s insights)
- [X] Planned 5 implementation phases (24-34 hours total)

### In Progress

None - spec phase complete.

### Jobs To Be Done (Next Session)

**Phase 1: Foundation (4-6 hours)**
1. [ ] Create `intelligence/` module in context-os-core
   - mod.rs, client.rs, metadata.rs, cost.rs, types.rs
   - Success criteria: `cargo build` succeeds
2. [ ] Add SQLite schema extensions
   - chain_metadata, commit_analysis, session_summaries, insights_cache, intelligence_costs tables
   - Success criteria: Migration runs on startup
3. [ ] Implement graceful degradation
   - Works without intel service running
   - Success criteria: Existing tests still pass

**Phase 2: Intelligence Service (6-8 hours)**
1. [ ] Create `context-os-intel/` Python project
   - FastAPI server, health endpoint
   - Success criteria: `uvicorn` starts on port 3002
2. [ ] Implement Chain Naming Agent
   - Using Claude Agent SDK with haiku model
   - Success criteria: /api/intel/name-chain returns meaningful names
3. [ ] Implement Session Summary Agent
   - Success criteria: /api/intel/summarize-session works

**Alternative: Quick Polish Sprint First**
If wanting visible progress before intelligence layer:
1. [ ] Fix Git Status error in HTTP mode (30 min)
2. [ ] Add keyboard shortcuts (1 hr)
3. [ ] Smart path truncation in file list (30 min)
4. [ ] Loading skeletons (1 hr)

## File Locations

| File | Purpose | Status |
|------|---------|--------|
| [[specs/canonical/05_INTELLIGENCE_LAYER_ARCHITECTURE.md]] | Intelligence Layer spec | **CREATED** |
| [[specs/canonical/03_CORE_ARCHITECTURE.md]] | Existing core architecture | Reference |
| [[specs/canonical/00_VISION.md]] | Vision document | Reference |
| [[specs/canonical/01_PRINCIPLES.md]] | Design principles | Reference |
| [[specs/canonical/02_ROADMAP.md]] | Development roadmap | Reference (stale - needs update) |
| [[apps/context-os/core/src/lib.rs]] | Core library entry | To be extended |
| [[apps/context-os/core/src/types.rs]] | Type definitions | To be extended |

## Test State

**Frontend:** 246 tests passing, 0 failing [VERIFIED: pnpm test:unit 2026-01-10]
**Backend:** 15 tests passing (Rust core) [VERIFIED: cargo test]

### Test Commands for Next Agent

```bash
# Frontend tests
cd apps/tastematter && pnpm test:unit

# Rust core tests
cd apps/context-os/core && cargo test

# Start servers for browser testing
# Terminal 1: Rust HTTP server
cd apps/context-os/core && cargo run --bin context-os -- serve --port 3001 --cors

# Terminal 2: Vite dev server
cd apps/tastematter && pnpm dev
```

## Key Research Findings

### Claude Agent SDK Capabilities

From Context7 research:

1. **Session Management**
   - Create/resume sessions with full history
   - Session ID in first system message
   - `resume: sessionId` to continue

2. **Hooks System**
   - PreToolUse, PostToolUse - intercept tool calls
   - SessionStart, SessionEnd - lifecycle
   - Can log, modify, or block actions

3. **Agent Definitions**
   ```python
   AgentDefinition(
       description="When to use this agent",
       prompt="System prompt with instructions",
       tools=["Read", "Grep"],  # Optional tool list
       model="haiku"  # or sonnet, opus
   )
   ```

4. **Model Selection**
   - haiku: ~$0.00025/call, ~1s latency (simple tasks)
   - sonnet: ~$0.003/call, ~3s latency (code analysis)

## For Next Agent

**Context Chain:**
- Previous: [[20_2026-01-10_QUICK_WINS_COMPLETE]] (10 quick wins done, 246 tests passing)
- This package: Intelligence Layer architecture spec complete
- Next action: Choose implementation path (Phase 1 Foundation or Quick Polish)

**Start here:**
1. Read this context package (you're doing it now)
2. Read [[05_INTELLIGENCE_LAYER_ARCHITECTURE.md]] for full spec details
3. Decide: Start Phase 1 (intelligence foundation) or Quick Polish sprint
4. If Phase 1: Create `apps/context-os/core/src/intelligence/` module
5. If Polish: Start with Git Status error fix

**Do NOT:**
- Edit existing context packages (append-only)
- Skip reading the Intelligence Layer spec before implementing
- Assume Tauri IPC works in browser mode (use HTTP transport)
- Forget to run tests after changes

**Key Architectural Decisions:**
1. **Separate Python service** - Claude SDK is Python; HTTP latency negligible vs API latency
2. **Lazy eval + persistent cache** - Analyze on first access, cache forever (chains/commits immutable)
3. **Model selection by task** - haiku for simple ($0.00025), sonnet for code ($0.003)
4. **Graceful degradation** - Core works without intel service; show raw IDs if unavailable

**Key insight:**
The Intelligence Layer transforms Tastematter from "data dump" to "intelligent assistant" by having Claude agents analyze sessions, commits, and patterns - then caching results in SQLite for instant retrieval. The two-service architecture (Rust + Python) is intentional: Claude SDK is Python, core is Rust, HTTP bridges them cleanly.

[VERIFIED: Architecture spec [[05_INTELLIGENCE_LAYER_ARCHITECTURE.md]] created 2026-01-10]
