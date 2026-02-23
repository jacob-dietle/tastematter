---
title: "Alerting & Publishing Context Package 05"
package_number: 05
date: 2026-02-20
status: current
previous_package: "[[04_2026-02-17_PHASE2_PUBLISHING_DEPLOYED]]"
related:
  - "[[apps/tastematter/control-plane/]]"
  - "[[apps/tastematter/alert-worker/src/alerting.ts]]"
  - "[[apps/tastematter/web-app/src/routes/+page.server.ts]]"
  - "[[.claude/skills/cloudflare-fullstack-engineering/SKILL.md]]"
tags:
  - context-package
  - tastematter
  - control-plane
  - infrastructure
---

# Alerting & Publishing - Context Package 05

## Executive Summary

Built the **Context Worker Control Plane** (`control.tastematter.dev`) — a dedicated health monitoring worker that polls all context workers hourly and surfaces status in the web app dashboard. Fixed the alert worker `content_change` stub (was firing every 4h about nothing). Hardened infrastructure with CF Access, custom domains, CORS, and SvelteKit server-side loading. Created `cloudflare-fullstack-engineering` skill from lessons learned.

## Global Context

### Architecture (Current)

```
app.tastematter.dev (CF Pages + SvelteKit)
  +page.server.ts fetches via service token from:
  ├── control.tastematter.dev (Control Plane Worker)
  │   ├── D1: tastematter-control (worker_registry + health_log)
  │   ├── Cron: hourly health polls
  │   └── 3 workers registered (ATP, alert-worker, intel-pipeline)
  └── api.tastematter.dev (Alert Worker)
      ├── D1: tastematter-alerts
      ├── R2: tastematter-corpus (34 files)
      ├── Cron: 4h alert evaluation
      ├── Alerting: content_change via corpus SHA comparison
      └── Publishing: ContextDO, ContextMCP, agentic query

All behind CF Access (tastematter.cloudflareaccess.com)
Shared service token across all workers
```

### Key Design Decisions

- **Dedicated control plane** over extending alert worker — the monitor shouldn't monitor itself [VERIFIED: plan approved by user]
- **SvelteKit server-side loading** over client-side cross-origin fetch — CF Access cookies are per-hostname, don't work cross-origin [VERIFIED: debugging session, 302 redirect on cross-origin fetch]
- **`printf |` for ALL secret operations** — `<<<` heredoc and `echo` both corrupt secrets on CF [VERIFIED: empty string values debugged via `__data.json`]
- **`--branch main` for Pages production** — custom domains only serve production environment [VERIFIED: old build served for 45 min before discovery]

## Completed This Session

### Infrastructure Hardening
- [X] Custom domain `api.tastematter.dev` for alert worker [VERIFIED: curl returns 200 with service token]
- [X] Custom domain `app.tastematter.dev` for web app [VERIFIED: CF Access login flow works]
- [X] CF Access protecting all three domains (app, api, control) [VERIFIED: unauthenticated requests return 302]
- [X] `workers_dev = false` on alert worker — no backdoor [VERIFIED: wrangler deploy output shows only custom domain]
- [X] CORS on alert worker for `app.tastematter.dev` origin [VERIFIED: `withCors()` wrapper in index.ts]
- [X] Web app fetches real data from alert worker API [VERIFIED: dashboard shows Nickel engagement with alert count]

### Control Plane Worker (NEW)
- [X] `control.tastematter.dev` deployed [VERIFIED: `/health` returns 200]
- [X] D1 `tastematter-control` with `worker_registry` + `health_log` tables [VERIFIED: migration applied]
- [X] 3 workers seeded: ATP (event/48h), alert-worker (4h/8h), intel-pipeline (daily/48h) [VERIFIED: `/workers` returns 3 entries]
- [X] Hourly cron health polling [VERIFIED: wrangler deploy shows `schedule: 0 * * * *`]
- [X] Health check logic: HTTP reachability + staleness detection via `last_activity` [VERIFIED: 20/20 tests passing]
- [X] Alert dedup: only fires on state transitions, not continued failures [VERIFIED: `shouldAlert()` tests]
- [X] API: `GET /workers`, `GET /workers/:id/health`, `POST /workers`, `DELETE /workers/:id`, `POST /workers/:id/check`

### Alert Worker Fixes
- [X] Replaced `content_change` always-fires stub with real corpus SHA comparison [VERIFIED: [[alerting.ts]]:30-44]
- [X] First check records baseline SHA (no alert), subsequent checks compare [VERIFIED: alerting.test.ts, 7 new tests]
- [X] `processAlertRules` now accepts `currentCorpusSha` from ContextDO health [VERIFIED: [[index.ts]]:208-220]
- [X] 84/84 tests passing (was 81) [VERIFIED: `pnpm test` output]

### Web App Dashboard
- [X] System Status section showing worker health grid [VERIFIED: live at app.tastematter.dev]
- [X] Server-side data loading via `+page.server.ts` [VERIFIED: `__data.json` returns worker + alert data]
- [X] Status dot colors: green (healthy/reachable), yellow (stale), red (down/timeout) [VERIFIED: CSS in +page.svelte]
- [X] ATP correctly shows as DOWN (broken since Feb 13) [VERIFIED: control plane poll result]

### New Skill Created
- [X] `cloudflare-fullstack-engineering` skill — 4 files, ~800 lines [VERIFIED: SKILL.md + 3 reference docs]
- [X] Covers Workers, Pages, D1, R2, CF Access, SvelteKit, cross-worker comms
- [X] Every foot-gun from this session documented with fix
- [X] Subsumes `cf-worker-deploy` (all content incorporated)

## Lessons Learned (Hard Way)

| Lesson | Time Lost | Now Documented In |
|--------|-----------|-------------------|
| `<<<` heredoc sends empty values for wrangler secrets | 45 min | cloudflare-fullstack-engineering SKILL.md:Rule 1 |
| Pages custom domains only serve production (`--branch main`) | 30 min | cloudflare-fullstack-engineering SKILL.md:Rule 2 |
| CF Access cookies are per-hostname, not cross-origin | 60 min | references/cf-access.md |
| Don't cast `platform.env` to `Record<string,string>` | 20 min | references/pages-sveltekit.md |
| Use `String(env?.KEY ?? '')` for Pages secrets | 20 min | references/pages-sveltekit.md |
| Pages secrets need redeploy to take effect | 10 min | references/pages-sveltekit.md |

## File Locations

### New Files
| File | Purpose |
|------|---------|
| `apps/tastematter/control-plane/` | Control plane worker (entire directory) |
| `apps/tastematter/control-plane/src/index.ts` | Routes + cron handler |
| `apps/tastematter/control-plane/src/health-checker.ts` | Health polling + staleness + alert dedup |
| `apps/tastematter/control-plane/src/db.ts` | D1 operations |
| `apps/tastematter/control-plane/src/types.ts` | Type definitions |
| `apps/tastematter/web-app/src/routes/+page.server.ts` | Server-side data loading |
| `.claude/skills/cloudflare-fullstack-engineering/` | New skill (4 files) |

### Modified Files
| File | Change |
|------|--------|
| `alert-worker/src/index.ts` | CORS, handleRequest refactor, corpus SHA in scheduled |
| `alert-worker/src/alerting.ts` | Real content_change with SHA comparison |
| `alert-worker/wrangler.toml` | Custom domain, workers_dev=false |
| `alert-worker/tests/alerting.test.ts` | Updated for SHA-based content_change |
| `web-app/src/routes/+page.svelte` | System Status section + server data props |
| `web-app/src/app.d.ts` | Platform env types for CF secrets |
| `web-app/wrangler.toml` | Added [vars] for URLs |

## Test State

| Project | Tests | Status |
|---------|-------|--------|
| alert-worker | 84/84 | All passing |
| control-plane | 20/20 | All passing |
| web-app | 3 (knock/push/bell) | Existing, not modified |

## Deployment State

| Service | URL | Version | Status |
|---------|-----|---------|--------|
| Alert Worker | api.tastematter.dev | e0ce0882 | Deployed, CF Access |
| Control Plane | control.tastematter.dev | caf54ef8 | Deployed, CF Access |
| Web App | app.tastematter.dev | 3a3f8e1a | Deployed (production), CF Access |

## Jobs To Be Done

### Immediate
1. [ ] Wire Knock alerts for control plane health transitions (Phase 2 of control plane plan)
2. [ ] Add `last_activity` to existing workers' `/health` endpoints (ATP, alert-worker, intel-pipeline) — currently all show "reachable" not "healthy"
3. [ ] Promote Knock workflow to production environment
4. [ ] Remove debug `console.log` statements if any remain

### Next Phase
5. [ ] Phase 3: Static Pages (extract intel pipeline HTML templates)
6. [ ] Test MCP from Claude Desktop (`/sse` endpoint)
7. [ ] Web app: dedicated `/workers` route with detailed health history per worker

### Strategic (from user vision)
8. [ ] Context worker template system — `tastematter worker init --template=X`
9. [ ] Context OS homepage: "What workers do you have? What's their state?"
10. [ ] Productization: wire control plane into the cf-worker-scaffold template

## For Next Agent

**Context Chain:**
- Previous: [[04_2026-02-17_PHASE2_PUBLISHING_DEPLOYED]] (Phase 2 done)
- This package: Control plane built, infra hardened, skill created
- Next action: Wire Knock alerts for health transitions OR add `last_activity` to worker health endpoints

**Start here:**
1. Read this package
2. Call `/cloudflare-fullstack-engineering` skill before ANY CF work
3. Run: `curl -s https://control.tastematter.dev/workers` to verify state
4. Check `apps/tastematter/control-plane/src/health-checker.ts` for alert integration point

**Do NOT:**
- Make cross-origin browser fetches to CF Access-protected workers (use server-side loading)
- Use `echo` or `<<<` for wrangler secrets (use `printf |`)
- Deploy Pages without `--branch main` (custom domain won't update)
- Cast `platform.env` to `Record<string, string>` (breaks property access)
