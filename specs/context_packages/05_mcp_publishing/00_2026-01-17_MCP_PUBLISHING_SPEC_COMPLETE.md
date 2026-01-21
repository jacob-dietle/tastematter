---
title: "MCP Publishing Context Package 00"
package_number: 0
date: 2026-01-17
status: current
previous_package: null
chain: "05_mcp_publishing"
origin: "[[03_current/27_2026-01-12_MIGRATION_EXECUTION_GUIDE]]"
related:
  - "[[canonical/10_MCP_PUBLISHING_ARCHITECTURE.md]]"
  - "[[canonical/02_ROADMAP.md]]"
  - "[[apps/cv_agentic_knowledge/app/deployments/corporate-visions/]]"
tags:
  - context-package
  - tastematter
  - mcp-publishing
  - phase-5
---

# Tastematter - Context Package 28

## Executive Summary

Created canonical spec for MCP Publishing (Phase 5) that bridges proven CVI patterns with Tastematter roadmap. Spec includes type contracts, CLI commands, UI wireframes, auth architecture, and implementation phases. ~800 lines of proven CVI code identified for porting.

## Global Context

Tastematter is evolving from a context visualization app to a context publishing platform. The vision (from December 2025) describes:
- Content-to-context pipeline
- Query agent with grep/read/list tools
- Cloudflare Worker + Durable Object architecture
- MCP server with authentication
- Context streaming / context as a service

### Architecture Overview

```
Level 0: Individual File Intelligence  ← Implemented (CLI)
Level 1: Single-Repo Desktop App       ← In Progress (Tauri app)
Level 2: Multi-Repo Dashboard          ← Phase 2 (roadmap)
Level 3: Inter-OS Protocols            ← Phase 5 (THIS SPEC)
        Context as a service, MCP publishing, pay-walling
```

### Key Design Decisions

1. **Use CVI as template** - Proven architecture, ~800 lines to port [VERIFIED: CVI codebase analysis]
2. **Cloudflare stack** - Worker + DO + R2 for stateful MCP [VERIFIED: [[corporate-visions/wrangler.toml]]]
3. **Single `query` tool exposed** - Agent handles tool orchestration internally [VERIFIED: [[mcp-wrapper.ts]]:16-19]
4. **Two-phase implementation** - 5A internal (MVP), 5B external (pay-walling) [VERIFIED: [[10_MCP_PUBLISHING_ARCHITECTURE.md]]:Implementation Phases]

## Local Problem Set

### Completed This Session

- [X] Restored context via tastematter CLI hypercube queries [VERIFIED: receipts q_f06a48, q_72af96, q_bc2fa8]
- [X] Read and analyzed CVI prototype files:
  - [[corporate-visions/src/index.ts]] - Worker entry, 209 lines
  - [[corporate-visions/src/query-handler.ts]] - Agentic loop, ~250 lines
  - [[corporate-visions/src/mcp-wrapper.ts]] - MCP SDK integration, 100 lines
  - [[corporate-visions/src/durable-objects/knowledge-graph-do.ts]] - Corpus holder, 85 lines
  - [[corporate-visions/scripts/generate-corpus.ts]] - Corpus generation, 133 lines
  - [[corporate-visions/wrangler.toml]] - Cloudflare config, 36 lines
- [X] Created canonical spec [[canonical/10_MCP_PUBLISHING_ARCHITECTURE.md]]
- [X] Identified known unknowns for user decisions

### In Progress

- [ ] User review of spec for decision points
  - Current state: Spec complete, awaiting feedback
  - Key decisions needed:
    1. Cloudflare API auth method (OAuth vs API token)
    2. Worker template bundling strategy
    3. Local MCP server priority
    4. Corpus size limits research

### Jobs To Be Done (Next Session)

1. [ ] Resolve known unknowns from spec - Get user decisions on technical/product questions
2. [ ] Implement `tastematter publish corpus` CLI command - Port from generate-corpus.ts
3. [ ] Create deployable Worker template - Package CVI code for Tastematter
4. [ ] Add auth middleware to Worker - API key verification pattern
5. [ ] Design publishers.yaml config schema - Storage for publish configurations

## File Locations

| File | Purpose | Status |
|------|---------|--------|
| [[canonical/10_MCP_PUBLISHING_ARCHITECTURE.md]] | Phase 5 canonical spec | Created |
| [[canonical/02_ROADMAP.md]] | Roadmap with Phase 5 outline | Reference |
| [[corporate-visions/src/index.ts]] | CVI Worker entry | Proven pattern |
| [[corporate-visions/src/query-handler.ts]] | Agentic query loop | Proven pattern |
| [[corporate-visions/src/mcp-wrapper.ts]] | MCP SDK integration | Proven pattern |
| [[corporate-visions/src/durable-objects/knowledge-graph-do.ts]] | DO corpus holder | Proven pattern |
| [[corporate-visions/scripts/generate-corpus.ts]] | Corpus generation | To port |

## Proven Patterns Extracted

### 1. Corpus Snapshot Schema

```typescript
interface CorpusSnapshot {
  version: string;
  commit: string;           // Git SHA for versioning
  fileCount: number;
  totalSize: number;
  generatedAt: string;
  files: Record<string, FileEntry>;
  allPaths: string[];       // Pre-computed path index
}
```
[VERIFIED: [[generate-corpus.ts]]:7-15]

### 2. Tool Definition Pattern (betaTool SDK)

```typescript
const grepTool = betaTool({
  name: 'grep',
  description: 'Search for patterns in the knowledge base',
  inputSchema: { pattern: string, caseInsensitive?: boolean, maxResults?: number },
  run: async (input) => { /* query DO */ }
});
```
[VERIFIED: [[query-handler.ts]]:53-98]

### 3. MCP Wrapper Pattern

```typescript
export class KnowledgeGraphMCP extends McpAgent {
  server = new McpServer({ name: 'context-name', version: '1.0.0' });

  async init() {
    this.server.tool('query', { question: z.string() }, async ({ question }) => {
      const result = await executeAgenticQueryStreaming(question, this.env);
      return { content: [{ type: 'text', text: result.response }] };
    });
  }
}
```
[VERIFIED: [[mcp-wrapper.ts]]:8-99]

### 4. Durable Object Lazy Loading

```typescript
if (!this.corpus && !this.loadPromise) {
  this.loadPromise = this.loadCorpusFromR2();
}
if (this.loadPromise) {
  await this.loadPromise;
  this.loadPromise = null;
}
```
[VERIFIED: [[knowledge-graph-do.ts]]:21-27]

## Context Sources Synthesized

| Source | Key Insight | Verification |
|--------|-------------|--------------|
| CVI Prototype | Full working MCP server architecture | [VERIFIED: codebase read] |
| Pixee Metaworker Spec | Two-layer model (internal + external) | [VERIFIED: [[00_CONTEXT_METAWORKER_SPEC.md]]:18-32] |
| December 2025 Voice Memo | Original vision for context streaming | [VERIFIED: user transcript] |
| Roadmap Phase 5 | Task breakdown for MCP publishing | [VERIFIED: [[02_ROADMAP.md]]:501-558] |

## Test State

No tests for Phase 5 yet (spec-only session).

### Verification Commands for Next Agent

```bash
# Verify spec exists
cat apps/tastematter/specs/canonical/10_MCP_PUBLISHING_ARCHITECTURE.md | head -50

# Check CVI prototype files still exist
ls apps/cv_agentic_knowledge/app/deployments/corporate-visions/src/

# Verify tastematter CLI works
tastematter --help
```

## For Next Agent

**Context Chain:**
- Previous: [[27_2026-01-12_MIGRATION_EXECUTION_GUIDE]] - Repository consolidation
- This package: MCP Publishing spec complete (Phase 5 canonical)
- Next action: Get user decisions on known unknowns, then implement CLI

**Start here:**
1. Read this context package
2. Read [[canonical/10_MCP_PUBLISHING_ARCHITECTURE.md]] for full spec
3. Review "Known Unknowns" section with user
4. Begin Phase 5A implementation starting with corpus CLI

**Do NOT:**
- Skip reading the CVI prototype code before implementing (proven patterns exist)
- Implement pay-walling before internal publishing works (Phase 5A before 5B)
- Create new Worker architecture - port the CVI patterns

**Key insight:**
The entire MCP publishing feature is ~800 lines of proven TypeScript to port from CVI. This is not greenfield development - it's adaptation of a working system.
[VERIFIED: CVI file analysis showing index.ts(209) + query-handler.ts(250) + mcp-wrapper.ts(100) + knowledge-graph-do.ts(85) + generate-corpus.ts(133) = ~777 lines]

## Known Unknowns (Decisions Needed)

### Technical

1. **Cloudflare API Authentication** - How does Tastematter auth to deploy workers?
   - Options: OAuth flow, API token input, Wrangler CLI delegation

2. **Worker Template Bundling** - Bundle in binary or fetch from GitHub?

3. **Local MCP Server Option** - Priority for non-Cloudflare users?

4. **Corpus Size Limits** - Practical limits for DO memory, R2 free tier?

### Product

1. **Pricing Model** - Per-query? Monthly? Free tier limits?

2. **Multi-Tenant Architecture** - Separate worker per publisher or shared?

3. **Context Updates** - Auto-sync with git or manual trigger?

---

**Package Created:** 2026-01-17
**Session Duration:** ~45 minutes
**Primary Output:** [[canonical/10_MCP_PUBLISHING_ARCHITECTURE.md]]
