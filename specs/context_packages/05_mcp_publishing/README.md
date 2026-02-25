# Alerting & Publishing Context Packages

Append-only context packages for Tastematter paid features: Context Alerting + Context Publishing.

## Philosophy

- **Append-only:** Never edit existing packages. New state = new file.
- **Wiki-linked:** Use [[node-name]] for traceable chains.
- **Evidence-based:** Every claim has [VERIFIED/INFERRED/UNVERIFIABLE] attribution.

## Focus

This chain tracks development of:
- Context alerting via Knock (unified: email + Web Push + in-app feed + Slack)
- Context publishing (static pages + queryable MCP sources)
- Web app (`app.tastematter.dev`) for management + notification center + push registration
- Context Worker template (generic CF Worker for all features)
- Engagement-scoped deployment and access (CF Zero Trust)
- Cross-system pattern extraction (intel pipeline, CVI, Nickel PR)

## Canonical Specs

**Primary spec:** [[canonical/17_CONTEXT_ALERTING_AND_PUBLISHING.md]] (v4, 1920 lines)
**Original MCP spec:** [[canonical/10_MCP_PUBLISHING_ARCHITECTURE.md]] (partially superseded)

**Proven patterns from:**
- [[apps/intelligence_pipeline/]] -- HTML dashboards, cron triggers, YAML config
- [[apps/cv_agentic_knowledge/app/deployments/corporate-visions/]] -- MCP server, corpus DO, agentic search
- [[apps/clients/nickel/conference_pr/worker/]] -- CF Access auth, pipeline stages
- [[apps/tastematter/download-alert-worker/]] -- cron + filter + notify pattern (ntfy.sh legacy)

## Timeline

| # | Date | Description |
|---|------|-------------|
| 00 | 2026-01-17 | MCP publishing spec complete, CVI patterns documented |
| 01 | 2026-02-14 | Cross-system synthesis, spec 17 v3 (ntfy.sh + founder interview) |
| 02 | 2026-02-15 | Spec 17 v4: Knock unified notifications, web app, Web Push, clean multi-tenancy seams |
| 03 | 2026-02-16 | Phase 1 deployed: alert worker + Knock + Resend email + web app live |
| 04 | 2026-02-17 | Phase 2 deployed: publishing MVP — ContextDO, MCP, agentic query, R2 corpus |
| 05 | 2026-02-20 | Control plane deployed, infra hardened (CF Access, custom domains, SSR), alert stub fixed, cloudflare-fullstack-engineering skill created |
| 06 | 2026-02-20 | Stigmergic Control Plane v2: system grouping, /status contract, Knock on system transitions, 5 specs, 152 tests. Phases 1-3 via coordinated agent team. |
| 07 | 2026-02-20 | Full registry (11 workers, 3 accounts), dashboard deployed, first cron poll, self-poll + fallback bugs fixed. 5 workers need /health endpoints. |
| 08 | 2026-02-23 | Epistemic grounding: 5→3 workers at 404, root cause (wrong URL + stale deploys), monorepo committed (6d44ee3), agent team created, plan approved. |

## Current State

Latest package: [[08_2026-02-23_STATUS_ENDPOINTS_GROUNDED_TEAM_READY]]
Status: 3 workers need /status endpoints (nickel-conference-pr, resy-finder, linkedin-post-alerting). Agent team `status-endpoints` created with shared task list. Plan approved. Next session: spawn teammates, implement, deploy, verify.

## Implementation Phases

### Phase 1: Alert Worker MVP + Knock Setup + Web App MVP
- [X] Knock account + "new-intel-brief" workflow (email via Resend + in-app feed)
- [X] Worker with cron handler + single Knock trigger call (every 4h)
- [X] Web app MVP: Svelte + CF Pages, Knock feed, Web Push registration scaffold, PWA
- [X] D1 schema: engagements + alert_history + alert_state + activity_log
- [X] OWNER_ID env var (clean seam for multi-tenancy)
- [X] First alert: email lands in inbox via Resend from alerts@tastematter.dev
- [ ] Push notifications (FCM not yet configured)
- [X] CF Access on web app + API + control plane (session 05)
- [ ] Promote Knock workflow to production environment
- [X] Custom domains: app.tastematter.dev, api.tastematter.dev, control.tastematter.dev (session 05)
- [X] Web app dashboard wired to real API data via server-side loading (session 05)
- [X] content_change stub replaced with real corpus SHA comparison (session 05)

### Phase 2: Publishing MVP
- [X] Port CVI worker template — ContextDO, ContextMCP, query-handler, tools (543 lines)
- [X] MCP endpoints: /mcp, /sse, /query, /reload, /query/logs
- [X] R2 bucket + corpus uploaded (34-file Nickel engagement, 394KB)
- [X] D1 query_log table (migration 002)
- [X] Agentic query verified end-to-end (Haiku + grep/read/list)
- [ ] CLI: `tastematter publish context` (manual script + wrangler for now)
- [ ] CF Zero Trust access management via CLI
- [ ] Web app: publishing management views

### Phase 3: Static Pages
- [ ] Extract intel pipeline HTML templates
- [ ] CLI: `tastematter publish pages`
- [ ] Page templates: dashboard, brief, catalog

### Phase 4: Web App Polish + Advanced Triggers
- [ ] Complete management views: Alert Manager, Publish Manager, Access Manager
- [ ] All trigger types (pattern_match, threshold, corpus_drift)
- [ ] Knock notification preferences UI

### Phase 5: Market Features (Future)
- [ ] Multi-tenancy (when 3-5 paying clients)
- [ ] Billing (Stripe), usage metering
- [ ] Multi-channel digest alerts
- [ ] Usage analytics dashboard

## How to Use

1. To continue work: Read latest package, follow "Start here" section
2. To understand history: Read packages in order (00 → latest)
3. To add new package: Increment number, never edit existing

## Related Chains

- [[03_current/]] - General Tastematter development
- [[04_daemon/]] - Indexer/daemon investigation
