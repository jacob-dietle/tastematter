---
title: "Alerting & Publishing Context Package 04"
package_number: 4
date: 2026-02-17
status: current
previous_package: "[[03_2026-02-16_PHASE1_ALERT_WORKER_DEPLOYED]]"
related:
  - "[[canonical/17_CONTEXT_ALERTING_AND_PUBLISHING.md]]"
  - "[[alert-worker/src/index.ts]]"
  - "[[alert-worker/src/context-do.ts]]"
  - "[[alert-worker/src/mcp-wrapper.ts]]"
  - "[[alert-worker/src/query-handler.ts]]"
  - "[[alert-worker/src/query-logging.ts]]"
  - "[[alert-worker/src/tools/grep.ts]]"
  - "[[alert-worker/src/tools/list.ts]]"
  - "[[alert-worker/src/tools/read.ts]]"
  - "[[alert-worker/scripts/generate-corpus.ts]]"
tags:
  - context-package
  - alerting-publishing
  - phase-2
  - publishing
  - deployed
---

# Alerting & Publishing - Context Package 04

## Executive Summary

Phase 2 (Context Publishing MVP) is deployed to production. The alert worker now serves both alerting AND publishing from a single worker. 81/81 tests passing. Agentic queries via `/query?q=...` work end-to-end — Haiku searches a 34-file Nickel corpus using grep/read/list tools and returns cited answers. MCP transport endpoints (`/mcp`, `/sse`) are live.

## Global Context

### System Architecture (deployed, updated from package #03)

```
tastematter-web-app.pages.dev (SvelteKit + CF Pages)
  - Dashboard (engagement list)
  - Knock notification center (bell icon + feed)
  - Push settings (FCM registration scaffold)
  - PWA manifest + service worker
          |
          | Knock client SDK (@knocklabs/client)
          v
Knock (knock.app)
  - Workflow: new-intel-brief (development env)
  - Steps: Resend email + in-app feed
  - User: "founder" → jacob@jdietle.me
          ^
          | Knock API (raw fetch)
          |
tastematter-alert-worker.jacob-4c8.workers.dev (CF Worker)
  ┌─ ALERTING (Phase 1) ─────────────────────────────┐
  │  Cron: 0 */4 * * *                               │
  │  D1: tastematter-alerts (5 tables)                │
  │  Routes: /health, /alert/history, POST /alert/trigger │
  │  processAlertRules → evaluateRule → triggerKnockWorkflow │
  └───────────────────────────────────────────────────┘
  ┌─ PUBLISHING (Phase 2) ───────────────────────────┐
  │  R2: tastematter-corpus (corpus-snapshot.json)    │
  │  DO: ContextDO (lazy R2 load, grep/read/list)    │
  │  DO: ContextMCP (McpAgent, query tool)            │
  │  Routes: /query, /mcp, /sse, /reload, /query/logs│
  │  Agentic: Haiku + betaTool (grep/read/list)      │
  │  Logging: D1 query_log table                     │
  └───────────────────────────────────────────────────┘
```

### Key Design Decisions

- **Unified worker** — Alerting + Publishing in one worker, not separate [VERIFIED: [[alert-worker/src/index.ts]]:1-202]
- **Cloudflare `agents` package** for McpAgent — official CF MCP framework, requires `ai` (Vercel) as transitive build dep [VERIFIED: [[alert-worker/package.json]]:19]
- **Raw fetch for Knock API** — no `@knocklabs/node` SDK on worker side [VERIFIED: [[alert-worker/src/knock.ts]]:1-40]
- **`@knocklabs/client`** on web app side — for notification feed + push [VERIFIED: [[web-app/package.json]]]
- **ContextDO + ContextMCP** — two Durable Objects, both use `new_sqlite_classes` migration [VERIFIED: [[alert-worker/wrangler.toml]]:31-37]
- **Corpus from R2** — generate-corpus.ts creates JSON snapshot, upload via `wrangler r2 object put --remote` [VERIFIED: [[alert-worker/scripts/generate-corpus.ts]]:1-131]

## Local Problem Set

### Completed This Session

- [X] Fixed index.test.ts `cloudflare:workers` import error — added `server.deps.inline` to vitest config + `agents/mcp` mock [VERIFIED: [[alert-worker/vitest.config.ts]]:8-12, [[alert-worker/tests/mocks/agents-mcp.ts]]]
- [X] All tests passing: 81/81 (was 75 passing + 1 suite failing) [VERIFIED: vitest run 2026-02-17]
- [X] Created R2 bucket `tastematter-corpus` [VERIFIED: wrangler r2 bucket create output]
- [X] Applied migration 002 (query_log table) to remote D1 [VERIFIED: wrangler d1 migrations apply --remote]
- [X] Set ANTHROPIC_API_KEY secret from `apps/automated_transcript_processing/.dev.vars` [VERIFIED: wrangler secret put]
  - **NOTE:** Initially used Nickel key by mistake, immediately overwritten with correct key
- [X] Deployed Phase 2 worker (v0.2.0) with all publishing routes [VERIFIED: wrangler deploy → ef6e9eb9]
- [X] Generated Nickel corpus (34 files, 394KB) and uploaded to R2 [VERIFIED: wrangler r2 object put --remote]
- [X] Verified `/health` shows `{ alerting: true, publishing: true, corpus: { loaded: true, fileCount: 34 } }` [VERIFIED: curl output]
- [X] Verified `/query?q=...` returns agentic answer with [VERIFIED:] citations from corpus [VERIFIED: curl output, 31.9KB response]
- [X] Verified `/mcp` endpoint responds with proper MCP transport negotiation [VERIFIED: curl output]
- [X] Ran epistemic context grounding skill — verified API contracts, identified frontend/ as Tauri (not usable for web app) [VERIFIED: session activity]
- [X] Generated implementation tracker status view [VERIFIED: session activity]

### Not Done (from plan)

- [ ] Push notifications — FCM project not created, no VAPID key
- [ ] CF Access on web app — currently public (no auth)
- [ ] Promote Knock workflow to production environment (session expired)
- [ ] Custom domain for web app (app.tastematter.dev) — skipped per user
- [ ] CLI commands (`tastematter publish context`, `tastematter access`) — manual workflow only
- [ ] Web app publishing management views — dashboard is placeholder data
- [ ] Connect web app dashboard to real alert-worker API data

### Jobs To Be Done (Next Session)

1. [ ] **Promote Knock to production** — `knock login` → `knock commit --promote`. Then set production KNOCK_API_KEY secret.
2. [ ] **CF Access on web app** — CF Zero Trust API: create application + allow policy for founder email.
3. [ ] **Wire web app to real API** — `+page.svelte` currently has hardcoded data. Fetch from alert-worker `/alert/history`.
4. [ ] **Phase 3: Static Pages** (~400 lines) — Extract intel pipeline HTML templates, add `/pages/*` route.
5. [ ] **Test MCP from Claude Desktop** — Add worker URL to Claude Desktop MCP config, verify agentic search works.

## File Locations

### Alert Worker (`apps/tastematter/alert-worker/`)

| File | Purpose | Lines | Status |
|------|---------|-------|--------|
| [[alert-worker/src/index.ts]] | fetch + scheduled handlers, all routes | 202 | Phase 2 complete |
| [[alert-worker/src/types.ts]] | Env, Result<T>, all types | 267 | Phase 2 complete |
| [[alert-worker/src/alerting.ts]] | evaluateRule + processAlertRules | 179 | Phase 1 |
| [[alert-worker/src/db.ts]] | createDB closure, 7 CRUD methods | 202 | Phase 1 |
| [[alert-worker/src/knock.ts]] | triggerKnockWorkflow via raw fetch | 40 | Phase 1 |
| [[alert-worker/src/config.ts]] | parseEngagementConfig | 57 | Phase 1 |
| [[alert-worker/src/context-do.ts]] | ContextDO: R2 corpus, grep/read/list | 79 | Phase 2 NEW |
| [[alert-worker/src/mcp-wrapper.ts]] | ContextMCP: McpAgent + query tool | 65 | Phase 2 NEW |
| [[alert-worker/src/query-handler.ts]] | Agentic loop: Haiku + betaTool | 312 | Phase 2 NEW |
| [[alert-worker/src/query-logging.ts]] | D1 query_log insert + query | 65 | Phase 2 NEW |
| [[alert-worker/src/tools/grep.ts]] | Regex search over corpus | 75 | Phase 2 NEW |
| [[alert-worker/src/tools/list.ts]] | Glob matching over paths | 60 | Phase 2 NEW |
| [[alert-worker/src/tools/read.ts]] | File content retrieval | 22 | Phase 2 NEW |
| [[alert-worker/scripts/generate-corpus.ts]] | Git-based corpus snapshot generator | 131 | Phase 2 NEW |
| [[alert-worker/migrations/002_add_query_log.sql]] | query_log table | 13 | Applied |
| [[alert-worker/vitest.config.ts]] | Vitest config with cloudflare: + agents/ mocks | 15 | Fixed this session |
| [[alert-worker/tests/mocks/agents-mcp.ts]] | Mock McpAgent for tests | 25 | NEW this session |

### Web App (`apps/tastematter/web-app/`)

No changes this session. 10 tests, 799 lines. Deployed at `tastematter-web-app.pages.dev`.

## Test State

### Alert Worker: 81/81 passing
```bash
cd apps/tastematter/alert-worker && pnpm test
# 8 test files, 81 tests, ~15s
```

### Web App: 10/10 passing
```bash
cd apps/tastematter/web-app && pnpm test
# 3 test files, 10 tests, ~33s
```

## Deployment Details

### Alert Worker
- **URL:** `https://tastematter-alert-worker.jacob-4c8.workers.dev`
- **Account:** `4c8353a21e0bfc69a1e036e223cba4d8`
- **Version:** ef6e9eb9 (deployed 2026-02-17)
- **D1:** `tastematter-alerts` / `326e6f35-f971-46c9-ad6f-f332ff2dda1a` (5 tables)
- **R2:** `tastematter-corpus` (34-file Nickel corpus, 394KB)
- **DOs:** ContextDO (corpus holder), ContextMCP (MCP server)
- **Cron:** `0 */4 * * *`
- **Secrets:** `KNOCK_API_KEY`, `ANTHROPIC_API_KEY` (from automated_transcript_processing)
- **Var:** `OWNER_ID = "founder"`

### Web App
- **URL:** `https://tastematter-web-app.pages.dev`
- No changes this session

### Knock
- **Workflow:** `new-intel-brief` (development environment — NOT promoted to prod yet)
- **CLI session:** Expired (need `knock login` to re-auth)

## Codebase Summary

| Component | Source Lines | Test Lines | Tests |
|-----------|-------------|------------|-------|
| Alert Worker | 1,625 | 1,498 | 81 |
| Web App | 799 | 100 | 10 |
| Migrations | 57 | — | — |
| Scripts | 131 | — | — |
| **Total** | **2,612** | **1,598** | **91** |

## Implementation Status

| Phase | Name | Lines | Status | Tests | Package |
|-------|------|-------|--------|-------|---------|
| 1 | Alert Worker + Knock + Web App | 1,625 + 799 | ✅ COMPLETE (90%) | 91 | #03 |
| 2 | Publishing MVP | 543 new | ✅ COMPLETE | incl. above | #04 (this) |
| 3 | Static Pages | ~400 est. | ⬜ READY | 0 | — |
| 4 | Web App Polish + Advanced Triggers | ~1,200 est. | ⬜ READY | 0 | — |
| 5 | Advanced Features (Market) | TBD | ⬜ BLOCKED | 0 | — |

**Overall Progress:** 2/5 phases complete (~40%), core product loop working

## Lessons Learned

### ANTHROPIC_API_KEY sourcing
- Do NOT use Nickel client's `.dev.vars` for tastematter secrets
- Use `apps/automated_transcript_processing/.dev.vars` for shared Anthropic key
- Each project should have its own key eventually

### Vitest + Cloudflare Workers
- `cloudflare:workers` protocol not recognized by Node ESM loader
- Fix: `server.deps.inline: [/cloudflare:/, /^agents/]` in vitest config forces resolution through Vite pipeline where aliases work
- Also need mock for `agents/mcp` (McpAgent class)

### `agents` package (Cloudflare MCP)
- `agents` is Cloudflare's official MCP agent framework — legitimate dependency
- Transitively requires `ai` (Vercel AI SDK) for `getAITools()` method we don't use
- Must install `ai` as dependency to satisfy esbuild at bundle time
- At runtime, the `ai` code path is never hit

### R2 uploads
- `wrangler r2 object put` defaults to LOCAL — always add `--remote` flag for production

## For Next Agent

**Context Chain:**
- Previous: [[03_2026-02-16_PHASE1_ALERT_WORKER_DEPLOYED]] (Phase 1 deployed)
- This package: Phase 2 deployed — publishing works end-to-end
- Next action: Promote Knock to prod, add CF Access, wire web app to real API

**Start here:**
1. Read this context package
2. Read [[canonical/17_CONTEXT_ALERTING_AND_PUBLISHING.md]] for full spec (Phase 3+ details)
3. Run `cd apps/tastematter/alert-worker && pnpm test` to verify 81/81
4. Run `curl https://tastematter-alert-worker.jacob-4c8.workers.dev/health` to verify deployment

**Do NOT:**
- Use Nickel's `.dev.vars` for API keys — use `apps/automated_transcript_processing/.dev.vars`
- Use `echo` for wrangler secrets — always `printf` (no trailing newline)
- Put `account_id` in Pages wrangler.toml — use `CLOUDFLARE_ACCOUNT_ID` env var
- Forget `--remote` on `wrangler r2 object put`
- Assume Knock CLI sessions persist — re-auth with `knock login` if commands fail
- Install `@knocklabs/node` on the worker — raw fetch is used deliberately (smaller bundle)
