---
title: "Context Package 06 — Stigmergic Control Plane v2"
package_number: 6
date: 2026-02-20
status: current
previous_package: "[[05_2026-02-20_CONTROL_PLANE_AND_INFRA_HARDENING]]"
related:
  - "[[21_STIGMERGIC_CONTROL_PLANE_V2]]"
  - "[[21a_PHASE1_FOUNDATION]]"
  - "[[21b_PHASE2_ALERTING]]"
  - "[[21c_PHASE3_STATUS_IMPLEMENTATION]]"
  - "[[21d_PHASE4_DASHBOARD]]"
  - "[[stigmergy]]"
  - "[[context-worker-agent-pattern]]"
tags:
  - context-package
  - tastematter
  - control-plane
  - stigmergy
---

# Stigmergic Control Plane v2 — Context Package 06

## Executive Summary

Designed and implemented the v2 control plane architecture grounded in stigmergic coordination theory. Two load-bearing primitives: PUSH (GitHub Action webhook after corpus sync) and PULL (hourly /status polling). Everything else — system health, corpus freshness, trail health, error intelligence — emerges from the delta between push and pull. Phases 1-3 complete and deployed. Phase 4 (dashboard) next.

## Global Context

### Architecture: Two Primitives

```
GitHub Actions ──POST /sync──► Control Plane ──GET /status──► Workers
                (push)          │  D1:                         (pull)
                                │  worker_registry
                                │  system_registry (NEW)
                                │  health_log
                                │  sync_log (NEW)
                                │
                                ├── Cron: hourly poll
                                ├── Knock: on system transitions (NEW)
                                │
                    Dashboard ◄──┘ (SSR +page.server.ts)
```

**Intelligence = delta(push, pull)** — Compare what SHOULD be (push) vs what IS (pull). The gap is the signal.

### Stigmergic Foundation

The control plane's D1 data is itself a pheromone trail. Future meta-workers read `health_log`, `sync_log`, and `system_registry` to make coordination decisions without direct messaging. Grounded in [[stigmergy]] and [[context-worker-agent-pattern]] from knowledge base.

### Key Design Decisions

- **System grouping**: Workers belong to systems (colonies). System health is holistic — aggregated from member worker statuses. [VERIFIED: [[21_STIGMERGIC_CONTROL_PLANE_V2]]]
- **`/status` contract enforced**: All workers must implement `/status` returning identity, vitals, corpus, trail, d1_health, schedule. Fallback to `/health` for robustness. [VERIFIED: [[21c_PHASE3_STATUS_IMPLEMENTATION]]]
- **Full control surface in dashboard**: Not just visibility — force check, reload corpus, toggle enabled. [VERIFIED: user decision in architecture session]
- **Knock on system transitions**: Alert on HEALTHY→BROKEN and BROKEN→HEALTHY, not continued failures. [VERIFIED: [[21b_PHASE2_ALERTING]]]

### Worker Ecosystem (from Cloudflare MCP discovery)

**Personal Account (4c8353a2...):** 16 workers
**Pixee Account (b5055b58...):** 3 workers
**D1 Databases:** 8 total
**Systems:** 5 (intel-pipeline, tastematter-platform, client-deployments, internal-tools, pixee-intel)

## Local Problem Set

### Completed This Session

**Architecture & Planning:**
- [X] Epistemic grounding: audited all existing data (control plane D1, alert worker D1, dashboard rendering) [VERIFIED: session activity]
- [X] Identified gap: rich data collected but not surfaced (9 columns in alert_history, dashboard uses 2)
- [X] Designed two-primitive architecture (push + pull) grounded in [[stigmergy]] and [[04_GIT_STIGMERGY_FOUNDATION]]
- [X] Created architecture visualization at `_system/reports/control_plane_architecture.html` [VERIFIED: file exists]
- [X] Wrote 5 specs: [[21_STIGMERGIC_CONTROL_PLANE_V2]], [[21a_PHASE1_FOUNDATION]], [[21b_PHASE2_ALERTING]], [[21c_PHASE3_STATUS_IMPLEMENTATION]], [[21d_PHASE4_DASHBOARD]]

**Phase 1 — Foundation (me):**
- [X] D1 migration 002: system_registry, worker_registry extensions (system_id, account_id, status_url), sync_log [VERIFIED: applied to production D1]
- [X] 5 systems seeded, 3 workers assigned [VERIFIED: D1 query via CF MCP]
- [X] /status polling with /health fallback in health-checker.ts [VERIFIED: 46 tests]
- [X] System health computation (computeSystemHealth) with transition detection [VERIFIED: 9 tests]
- [X] New routes: GET/POST /systems, PATCH /workers/:id, POST /sync, GET /sync/:id, proxy /reload + /trigger [VERIFIED: deployed]
- [X] parseStatusResponse validator for /status contract [VERIFIED: 6 tests]
- [X] Deployed to control.tastematter.dev [VERIFIED: wrangler deploy output]

**Phase 2 — Knock Alerting (knock-alerting agent):**
- [X] Created knock.ts with triggerSystemAlert + buildAlertSummary [VERIFIED: 13 tests]
- [X] Wired into cron handler's system transition block [VERIFIED: tests pass]
- [X] Added Result<T> type and KNOCK_WORKFLOW_KEY to Env [VERIFIED: types.ts]
- [X] Deployed [VERIFIED: wrangler deploy output]

**Phase 3 — /status Endpoints (status-endpoints agent):**
- [X] Added GET /status to alert-worker with corpus, trail, d1_health, schedule [VERIFIED: 6 tests]
- [X] Added GET /status to control-plane for self-monitoring [VERIFIED: 3 tests]
- [X] Added WorkerStatusResponse type to alert-worker [VERIFIED: types.ts]
- [X] Both deployed [VERIFIED: wrangler deploy output]

**Skill & Tooling:**
- [X] Updated cloudflare-fullstack-engineering skill with Cloudflare MCP API safety rules [VERIFIED: SKILL.md]
- [X] Created references/cloudflare-mcp-api.md with account registry, safe/dangerous classification, common patterns [VERIFIED: file exists]
- [X] Cloudflare MCP connected — can list workers, query D1, inspect infrastructure across both accounts [VERIFIED: API calls returned data]

### Completed (Phase 4 — Dashboard)

- [X] Expanded +page.server.ts: fetches /systems + /workers + /alert/history, form actions for forceCheck, reloadCorpus, toggleWorker [VERIFIED: deployed to app.tastematter.dev]
- [X] Rewrote +page.svelte: system cards with holistic health badges, member workers with status dots, error messages, trail info, corpus info from raw_response parsing [VERIFIED: deployed]
- [X] Control actions: Check button per worker via SvelteKit form actions + use:enhance [VERIFIED: deployed]
- [X] Enriched alert history: shows rule_name, trigger_type, failure count per engagement [VERIFIED: deployed]
- [X] Ungrouped workers section for workers without system_id [VERIFIED: deployed]
- [X] Built and deployed to production with `--branch main` [VERIFIED: wrangler pages deploy output]

### Spec 21 Status: COMPLETE (4/4 phases)

| Phase | Name | Tests | Status |
|-------|------|-------|--------|
| 1 | Foundation | 46 | ✅ COMPLETE |
| 2 | Knock Alerting | +16 → 62 | ✅ COMPLETE |
| 3 | /status Endpoints | +9 → 152 total | ✅ COMPLETE |
| 4 | Dashboard | manual | ✅ COMPLETE |

### Jobs To Be Done (Post-v1 — Populating the System)

1. [ ] **Register remaining workers** — 16 of 19 workers unregistered. POST /workers for each with system_id, account_id, health_url, auth_type.
2. [ ] **Create Knock workflow + set secret** — Create "system-health-alert" workflow in Knock dashboard, then `printf "system-health-alert" | wrangler secret put KNOCK_WORKFLOW_KEY` on control plane.
3. [ ] **Add /status to nickel-conference-pr** — ~50 lines, has corpus + D1. Medium priority.
4. [ ] **Add /status to transcript-processing** — ~50 lines, has D1 flow_logs. Medium priority.
5. [ ] **Add /status to intelligence-pipeline** — ~50 lines, has D1. Medium priority.
6. [ ] **Wire GitHub Actions to POST /sync** — Add webhook call to sync-nickel-corpus.yml and sync-state-to-r2.yml after R2 upload. ~10 lines each.
7. [ ] **Add /status to Pixee workers** — Cross-account (b5055b58...). Low priority.
8. [ ] **Decide on legacy workers** — 6 workers from Nov 2025 untouched. Register or decommission.

## Test State

| Component | Tests | Status |
|-----------|-------|--------|
| Control Plane | 62 passing | All green |
| Alert Worker | 90 passing | All green |
| **Total** | **152** | **All green** |

```bash
# Verify
cd apps/tastematter/control-plane && npx vitest run
cd apps/tastematter/alert-worker && npx vitest run
```

## Test State

| Component | Tests | Status |
|-----------|-------|--------|
| Control Plane | 62 passing | All green |
| Alert Worker | 90 passing | All green |
| **Total** | **152** | **All green** |

```bash
cd apps/tastematter/control-plane && npx vitest run  # expect 62
cd apps/tastematter/alert-worker && npx vitest run   # expect 90
```

## File Locations

### New Files This Session

| File | Purpose |
|------|---------|
| `control-plane/migrations/002_control_plane_v2.sql` | Schema: system_registry, worker_registry extensions, sync_log |
| `control-plane/src/knock.ts` | Knock integration for system health alerts |
| `control-plane/tests/knock.test.ts` | 13 tests for Knock integration |
| `control-plane/tests/status.test.ts` | 3 tests for /status self-monitoring |
| `alert-worker/tests/status.test.ts` | 6 tests for /status endpoint |
| `specs/canonical/21_STIGMERGIC_CONTROL_PLANE_V2.md` | Architecture guide |
| `specs/canonical/21a_PHASE1_FOUNDATION.md` | Phase 1 spec |
| `specs/canonical/21b_PHASE2_ALERTING.md` | Phase 2 spec |
| `specs/canonical/21c_PHASE3_STATUS_IMPLEMENTATION.md` | Phase 3 spec |
| `specs/canonical/21d_PHASE4_DASHBOARD.md` | Phase 4 spec |
| `_system/reports/control_plane_architecture.html` | Architecture visualization |
| `.claude/skills/cloudflare-fullstack-engineering/references/cloudflare-mcp-api.md` | CF MCP safety guide |

### Modified Files

| File | Changes |
|------|---------|
| `control-plane/src/types.ts` | Added system types, /status contract, sync types, Result<T> |
| `control-plane/src/db.ts` | Added system CRUD, sync CRUD, updateWorker, getSystemsWithMembers |
| `control-plane/src/health-checker.ts` | /status-first polling, parseStatusResponse, computeSystemHealth |
| `control-plane/src/index.ts` | All new routes, system health in cron, Knock on transitions, GET /status |
| `alert-worker/src/index.ts` | Added GET /status route |
| `alert-worker/src/types.ts` | Added WorkerStatusResponse |
| `web-app/src/routes/+page.server.ts` | Fetch /systems + /workers, form actions (forceCheck, reloadCorpus, toggleWorker) |
| `web-app/src/routes/+page.svelte` | Full rewrite: system cards, worker detail, controls, enriched alerts |
| `.claude/skills/cloudflare-fullstack-engineering/SKILL.md` | Added CF MCP API section |

## Team Execution Note

Phases 2 and 3 were executed by a coordinated agent team (TeamCreate):
- **knock-alerting** agent: Phase 2 (Knock integration) — completed in ~5 min
- **status-endpoints** agent: Phase 3 (/status endpoints) — completed in ~7 min
- Zero file conflicts despite both touching control-plane/src/index.ts (different sections)
- Team created, tasks assigned, agents spawned, work verified, team deleted — clean lifecycle

## For Next Agent

**Context Chain:**
- Previous: [[05_2026-02-20_CONTROL_PLANE_AND_INFRA_HARDENING]] (control plane v1, infra hardening)
- This package: v2 architecture with system grouping, Knock, /status contract
- Next action: Phase 4 dashboard implementation

**Start here:**
1. Read this context package
2. Read [[21d_PHASE4_DASHBOARD]] for Phase 4 spec
3. Read current `web-app/src/routes/+page.server.ts` and `+page.svelte`
4. Run: `cd apps/tastematter/control-plane && npx vitest run` (expect 62 passing)
5. Implement Phase 4 per spec

**Key insight:**
The dashboard is purely a rendering layer. It computes nothing — it displays what the control plane provides via GET /systems and GET /workers. All data loading happens server-side in +page.server.ts using CF Access service tokens. No client-side fetches to workers. [VERIFIED: [[21d_PHASE4_DASHBOARD]]]
