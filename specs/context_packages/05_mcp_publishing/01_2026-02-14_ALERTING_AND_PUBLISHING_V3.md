---
title: "Tastematter Context Package 01 - Alerting & Publishing v3"
package_number: 1
date: 2026-02-14
status: current
previous_package: "[[00_2026-01-17_MCP_PUBLISHING_SPEC_COMPLETE]]"
chain: "05_mcp_publishing"
related:
  - "[[canonical/17_CONTEXT_ALERTING_AND_PUBLISHING.md]]"
  - "[[canonical/10_MCP_PUBLISHING_ARCHITECTURE.md]]"
  - "[[apps/intelligence_pipeline/]]"
  - "[[apps/cv_agentic_knowledge/app/deployments/corporate-visions/]]"
  - "[[apps/clients/nickel/conference_pr/worker/]]"
  - "[[apps/tastematter/download-alert-worker/]]"
tags:
  - context-package
  - tastematter
  - alerting
  - publishing
  - ntfy
  - resend
  - context-worker
---

# Tastematter - Context Package 01: Alerting & Publishing v3

## Executive Summary

Coordinated 5-agent team exploration of three production systems (intelligence pipeline, CVI knowledge graph, Nickel conference PR) to synthesize patterns into two productized tastematter paid features: **Context Alerting** (email via Resend + push via ntfy.sh) and **Context Publishing** (static pages + queryable MCP sources). Updated canonical spec 17 from v2 to v3 incorporating founder interview findings and ntfy.sh switch.

## Global Context

### The Cross-System "Context Worker" Primitive

Every system built follows the same five-stage pattern:

```
INGEST --> PROCESS --> STORE --> SERVE --> NOTIFY
```

| System | Ingest | Process | Store | Serve | Notify |
|--------|--------|---------|-------|-------|--------|
| Intelligence Pipeline | Readwise, Parallel, Twitter | Claude classify + brief gen | D1 + R2 | HTML dashboards | Slack (legacy) |
| CVI Knowledge Graph | Git repo -> corpus JSON | Agentic search (Claude) | R2 -> DO in-memory | MCP protocol | (none) |
| Nickel Conference PR | Press list import | Web research + scoring | D1 + R2 + DO | JSON API | (none) |
| Download Alert Worker | CF GraphQL API | Regex filter | (none) | (none) | ntfy.sh (v3) |

### Key Design Decisions (v3)

1. **ntfy.sh replaces Web Push** for push notifications -- proven in download-alert-worker, single HTTP POST, zero infrastructure [VERIFIED: [[download-alert-worker/src/index.ts]] -- user switched from Slack to ntfy.sh during this session]
2. **Resend for email** -- single fetch call, free tier 100/day [VERIFIED: Resend API docs]
3. **Cloudflare Zero Trust for auth** -- zero auth code in worker, edge validation [VERIFIED: [[canonical/17_CONTEXT_ALERTING_AND_PUBLISHING.md]]:496-528]
4. **Engagement as top-level noun** -- scopes publishing, access, alerting per client [VERIFIED: [[canonical/17_CONTEXT_ALERTING_AND_PUBLISHING.md]]:992-998]
5. **Port CVI code, don't rewrite** -- ~800 lines proven TypeScript [VERIFIED: [[00_2026-01-17_MCP_PUBLISHING_SPEC_COMPLETE]]:199-201]

## Local Problem Set

### Completed This Session

- [X] Explored intelligence pipeline architecture via agent team [VERIFIED: 3 parallel Explore agents + lead reads of ~15 key source files]
- [X] Explored CVI knowledge graph query patterns [VERIFIED: Read full `index.ts`, `query-handler.ts`, `mcp-wrapper.ts`, `knowledge-graph-do.ts`, `generate-corpus.ts`]
- [X] Explored Nickel conference PR worker patterns [VERIFIED: Read full `index.ts` (263 lines), `wrangler.toml`, auth middleware]
- [X] Conducted founder interview on product direction [VERIFIED: 4 questions answered -- target user, pricing, first alerts, first publishes]
- [X] Updated canonical spec 17 from v2 to v3 with:
  - ntfy.sh replacing Web Push in all type contracts, wireframes, cost estimates
  - Go-to-market progression (me -> clients -> productized service -> product)
  - First use cases from founder interview
  - Reduced Phase 1 effort estimate (~400 lines vs ~500, ntfy.sh simpler than Web Push)
  [VERIFIED: [[canonical/17_CONTEXT_ALERTING_AND_PUBLISHING.md]] -- diff shows 12 edits]

### Founder Interview Findings (2026-02-14)

| Question | Answer |
|----------|--------|
| Target user | All three staged: me -> clients -> market |
| Pricing | Not enough users yet. Productized service first, then product |
| First alert | "Email me when new intel brief is generated with high-relevance articles" |
| First publish | ALL: intel briefs as pages, knowledge base as MCP, client deliverables, content portfolio |
| Push mechanism | ntfy.sh (switched download-alert-worker from Slack during session) |

### In Progress

- [ ] Spec 17 is draft-v3, awaiting implementation
  - Current state: All type contracts, wireframes, CLI commands, architecture defined
  - Blocker: None -- ready for Phase 1 implementation
  - Evidence: [VERIFIED: [[canonical/17_CONTEXT_ALERTING_AND_PUBLISHING.md]]:1-1603]

### Jobs To Be Done (Next Session)

1. [ ] **Phase 1: Alert Worker MVP** -- Build worker that emails + ntfy-pushes when new intel brief is generated
   - Success criteria: Founder receives email + phone push within 4h of new brief
   - Key files to create: `apps/tastematter/alert-worker/` (new worker)
   - Key files to port: `download-alert-worker/src/index.ts` (ntfy.sh pattern), `intelligence_pipeline/src/generation/notifications.ts` (formatting)
   - Estimated: ~400 lines new TypeScript

2. [ ] **Phase 2: Publishing MVP** -- Publish a directory as queryable MCP source with CF Zero Trust auth
   - Success criteria: Claude Desktop can query published knowledge base
   - Key files to port: CVI `src/` (~800 lines), `scripts/generate-corpus.ts` (133 lines)
   - Key files to create: `apps/tastematter/context-worker-template/` (generic worker template)

3. [ ] **Phase 3: Static Pages** -- Publish intel briefs and client deliverables as shareable HTML pages
   - Key files to port: Intelligence pipeline HTML rendering (~800 lines of templates)

## File Locations

| File | Purpose | Status |
|------|---------|--------|
| [[canonical/17_CONTEXT_ALERTING_AND_PUBLISHING.md]] | Primary spec (v3) | Updated this session |
| [[canonical/10_MCP_PUBLISHING_ARCHITECTURE.md]] | Original MCP publishing spec | Reference (partially superseded by spec 17) |
| [[apps/intelligence_pipeline/src/index.ts]] | HTML rendering, pipeline routing | Reference (~1291 lines) |
| [[apps/intelligence_pipeline/src/generation/notifications.ts]] | Slack notification formatting | To port (160 lines) |
| [[apps/intelligence_pipeline/configs/context_engineering.yaml]] | YAML topic config pattern | Reference (186 lines) |
| [[apps/cv_agentic_knowledge/app/deployments/corporate-visions/src/]] | CVI worker (all 5 source files) | To port (~800 lines) |
| [[apps/cv_agentic_knowledge/app/deployments/corporate-visions/scripts/generate-corpus.ts]] | Corpus generation | To port (133 lines) |
| [[apps/clients/nickel/conference_pr/worker/src/index.ts]] | CF Access auth, pipeline routing | Reference (263 lines) |
| [[apps/tastematter/download-alert-worker/src/index.ts]] | ntfy.sh push pattern (v3) | Reference -- user modified Slack->ntfy.sh |
| [[apps/tastematter/download-alert-worker/wrangler.toml]] | Cron trigger + secrets config | Reference -- user modified |

## Notification Stack (v3)

```
Priority: Email (Resend) > Push (ntfy.sh) > Slack (legacy)

Email:  fetch("https://api.resend.com/emails", { ... })     -- 1 API key
ntfy:   fetch("https://ntfy.sh/${topic}", { body, Title })  -- 0 keys, just topic name
Slack:  fetch(webhook_url, { text, blocks })                 -- legacy, secondary
```

All three are single `fetch()` calls from a CF Worker. Zero SDK dependencies.

## For Next Agent

**Context Chain:**
- Previous: [[00_2026-01-17_MCP_PUBLISHING_SPEC_COMPLETE]] -- MCP publishing spec created, CVI patterns documented
- This package: Cross-system synthesis complete, spec 17 updated to v3 with ntfy.sh + founder interview findings
- Next action: Phase 1 implementation -- build the alert worker MVP

**Start here:**
1. Read this context package
2. Read [[canonical/17_CONTEXT_ALERTING_AND_PUBLISHING.md]] Part 5 (Implementation Path) for Phase 1 spec
3. Read [[apps/tastematter/download-alert-worker/src/index.ts]] for the proven ntfy.sh pattern to port
4. Build `apps/tastematter/alert-worker/` targeting the first use case: "email + push when new intel brief is generated"

**Do NOT:**
- Use Web Push API -- ntfy.sh is simpler and already proven (no VAPID keys, no service workers)
- Build custom API key auth -- use Cloudflare Zero Trust at the edge (zero worker code)
- Rewrite CVI code -- port the ~800 lines as-is, then generalize
- Start with desktop UI -- CLI-first, UI later (matches existing tastematter workflow)

**Key insight:**
The entire alerting feature is ~400 lines of new TypeScript because ntfy.sh push is a single HTTP POST (proven in download-alert-worker) and Resend email is a single fetch call. The publishing feature is ~200 lines new + ~800 lines ported from CVI. This is extraction and composition of proven patterns, not greenfield development.
[VERIFIED: download-alert-worker ntfy.sh integration = 8 lines; CVI worker template = 777 lines total across 5 files]
