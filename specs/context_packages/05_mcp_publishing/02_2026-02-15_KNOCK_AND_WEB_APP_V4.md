---
title: "Alerting & Publishing Context Package 02"
package_number: 2
date: 2026-02-15
status: current
previous_package: "[[01_2026-02-14_ALERTING_AND_PUBLISHING_V3]]"
related:
  - "[[canonical/17_CONTEXT_ALERTING_AND_PUBLISHING.md]]"
  - "[[canonical/10_MCP_PUBLISHING_ARCHITECTURE.md]]"
  - "[[apps/tastematter/download-alert-worker/src/index.ts]]"
tags:
  - context-package
  - alerting-publishing
  - knock
  - web-app
  - multi-tenancy
---

# Alerting & Publishing - Context Package 02

## Executive Summary

Replaced ntfy.sh + Resend + Slack with Knock as unified notification infrastructure. Added web app (`app.tastematter.dev`) as management UI + notification center + Web Push registration surface. Added Design Decision #8: clean seams for multi-tenancy (D1 config, OWNER_ID variable, engagement-scoped data) — ~1 day extra upfront to avoid ~10+ day retrofit later. Spec 17 now at v4 (1,920 lines), ready for Phase 1 implementation.

## Global Context

### System Architecture (v4)

```
app.tastematter.dev (Svelte + CF Pages)
  - Publishing management
  - Knock notification center (bell icon)
  - Web Push registration (FCM → Knock)
  - PWA for iOS/Android mobile push
          |
          | CLI / Cloudflare API
          v
CF Worker (tm-{engagement})
  - Cron triggers evaluate watch rules
  - Single fetch() → Knock API when alert fires
  - MCP + pages endpoints for publishing
  - D1/R2/DO storage per worker
          |
          | trigger workflow
          v
Knock (notification infrastructure)
  - Workflow: email → push → in-app → slack
  - Templates, batching, delivery retry
  - MCP server for Claude Code configuration
```

### Key Design Decisions (cumulative, v1-v4)

1. One Worker per context (isolation) [VERIFIED: [[17_CONTEXT_ALERTING_AND_PUBLISHING.md]]:Part 7.1]
2. **Knock as unified notification layer** (replaces DIY Resend+ntfy+Slack) [VERIFIED: [[17_CONTEXT_ALERTING_AND_PUBLISHING.md]]:Part 7.2]
3. Corpus snapshot pattern (not live filesystem) [VERIFIED: [[17_CONTEXT_ALERTING_AND_PUBLISHING.md]]:Part 7.3]
4. Single `query` MCP tool (agent handles internals) [VERIFIED: [[17_CONTEXT_ALERTING_AND_PUBLISHING.md]]:Part 7.4]
5. Templates not frameworks for pages [VERIFIED: [[17_CONTEXT_ALERTING_AND_PUBLISHING.md]]:Part 7.5]
6. CF Zero Trust for auth (zero worker code) [VERIFIED: [[17_CONTEXT_ALERTING_AND_PUBLISHING.md]]:Part 7.6]
7. Engagement as top-level noun [VERIFIED: [[17_CONTEXT_ALERTING_AND_PUBLISHING.md]]:Part 7.7]
8. **Clean seams for multi-tenancy** (not multi-tenant, not hardcoded) [VERIFIED: [[17_CONTEXT_ALERTING_AND_PUBLISHING.md]]:Part 7.8]

## Local Problem Set

### Completed This Session

- [X] Researched notification infrastructure: Knock, Novu, Courier, OneSignal [VERIFIED: web search + Context7 docs]
- [X] Discovered Knock MCP server (`@knocklabs/agent-toolkit`) — enables workflow config from Claude Code [VERIFIED: Context7 knock docs]
- [X] Analyzed Web Push capabilities across platforms (desktop + Android + iOS PWA) [VERIFIED: web research]
- [X] Updated spec 17 v3→v4: replaced ntfy.sh+Resend+Slack with Knock (12+ targeted edits) [VERIFIED: [[17_CONTEXT_ALERTING_AND_PUBLISHING.md]]:1920 lines]
- [X] Added web app section (`app.tastematter.dev`): Svelte management UI + Knock feed + Web Push registration + PWA manifest + service worker [VERIFIED: [[17_CONTEXT_ALERTING_AND_PUBLISHING.md]]:Part 3 web app section]
- [X] Analyzed single-tenant vs multi-tenant tradeoffs: upfront ~10-17 days vs retrofit ~12-19 days vs clean seams ~1 day [VERIFIED: cost analysis in conversation]
- [X] Added Design Decision #8: clean seams — D1 config, OWNER_ID variable, engagement-scoped data, CF Access on web app [VERIFIED: [[17_CONTEXT_ALERTING_AND_PUBLISHING.md]]:Part 7.8]
- [X] Updated D1 schema: added `engagements` table, `engagement_id` columns, `knock_workflow_run_id` [VERIFIED: [[17_CONTEXT_ALERTING_AND_PUBLISHING.md]]:Part 4 D1 schema]
- [X] Updated type contracts: `AlertingConfig` → Knock model, `KnockTriggerPayload`, `KnockRecipient`, `OWNER_ID` in Env [VERIFIED: [[17_CONTEXT_ALERTING_AND_PUBLISHING.md]]:Part 9]
- [X] Updated all 5 implementation phases for Knock + web app [VERIFIED: [[17_CONTEXT_ALERTING_AND_PUBLISHING.md]]:Part 5]

### In Progress

Nothing in progress — spec update complete, no implementation started.

### Jobs To Be Done (Next Session)

1. [ ] **Phase 1: Alert Worker MVP + Knock Setup**
   - Create Knock account + connect Resend email provider + FCM push provider
   - Create "new-intel-brief" workflow via Knock MCP server
   - Build worker template: cron handler + single Knock trigger call
   - Build web app MVP: Svelte + CF Pages, Knock notification feed, Web Push registration, PWA
   - D1 schema: `engagements`, `alert_history`, `alert_state`
   - `OWNER_ID` env var in wrangler.toml
   - Success: email + native push on desktop + phone + in-app bell icon when intel brief fires
   - Estimate: ~250 lines worker + ~200 lines web app + Knock config

2. [ ] **Phase 2: Publishing MVP** — Port CVI worker template, add MCP endpoints
3. [ ] **Phase 3: Static Pages** — Extract intel pipeline HTML templates
4. [ ] **Phase 4: Web App Polish** — Complete management views, Knock preferences
5. [ ] **Phase 5: Advanced Features** — Digest, analytics, team features

## Key Decisions Made This Session

### Why Knock (not DIY Resend + ntfy.sh + Slack)

**Problem:** ntfy.sh requires its own phone app install, looks like a developer tool not a product. "mfs are out here using telegram to message their agents and its just such a bush league ux" — need something elegant.

**Decision:** Knock as unified notification layer.

**Rationale:**
- One API call replaces three integrations (email, push, Slack)
- MCP server for Claude Code workflow configuration
- Web Push via FCM gives native OS notifications on ALL platforms (no native app)
- In-app feed component for notification center
- Workflow engine handles batching/delays/digests
- Free tier: 10K notifications/month
- Paid: $250/month at 50K (steep jump — monitor usage)

### Why Web App (not Tauri desktop only)

**Problem:** Web Push requires a web origin for registration. Tauri desktop can't register for browser push notifications.

**Decision:** Web app at `app.tastematter.dev` serves three roles: publishing management + notification center + push registration.

**Bonus:** Same app works as PWA on iOS (Add to Home Screen) and Android, giving mobile push without a native app. Svelte components shared with future Tauri desktop.

### Why Clean Seams (not multi-tenant, not hardcoded)

**Problem:** Full multi-tenancy costs 10-17 days upfront. But hardcoding "dietl" everywhere costs 12-19 days to retrofit + higher risk.

**Decision:** Four cheap foundations (~1 day total):
1. `OWNER_ID` env var (not hardcoded strings)
2. D1 `engagements` table with `owner_id` column (not local YAML)
3. `engagement_id` on all D1 tables
4. CF Access on web app from day one

**Defer:** User registration, billing (Stripe), usage metering, self-serve onboarding.
**Trigger for multi-tenancy:** 3-5 paying clients, spending more time on setup than product.

## File Locations

| File | Purpose | Status |
|------|---------|--------|
| [[canonical/17_CONTEXT_ALERTING_AND_PUBLISHING.md]] | Primary spec (v4, 1920 lines) | Updated |
| [[context_packages/05_mcp_publishing/README.md]] | Package chain index | Updated |
| [[apps/tastematter/download-alert-worker/src/index.ts]] | Legacy ntfy.sh worker (still in production) | Reference |

## Notification Infrastructure Reference

### Knock MCP Server Config

```json
{
  "mcpServers": {
    "knock": {
      "command": "npx",
      "args": ["-y", "@knocklabs/agent-toolkit", "-p", "local-mcp"],
      "env": { "KNOCK_SERVICE_TOKEN": "YOUR-SERVICE-TOKEN" }
    }
  }
}
```

### Knock Tools Available via MCP

`createWorkflow`, `triggerWorkflow`, `createOrUpdateEmailStepInWorkflow`, `createOrUpdatePushStepInWorkflow`, `createOrUpdateChatStepInWorkflow`, `createOrUpdateBatchStepInWorkflow`, `createOrUpdateInAppFeedStepInWorkflow`, `createOrUpdateDelayStepInWorkflow`, `listWorkflows`, `getWorkflow`

### Web Push Platform Support

| Platform | Works? | Requirement |
|----------|--------|-------------|
| Desktop Chrome/Edge/Firefox | Yes | Just allow notifications |
| Desktop Safari | Yes | Since Ventura (2022) |
| Android Chrome | Yes | Just allow notifications |
| iOS Safari | Yes | Add to Home Screen first (PWA) |

## For Next Agent

**Context Chain:**
- Previous: [[01_2026-02-14_ALERTING_AND_PUBLISHING_V3]] (ntfy.sh + founder interview)
- This package: Knock + web app + clean multi-tenancy seams (2026-02-15)
- Next action: Phase 1 implementation

**Start here:**
1. Read this context package (you're doing it now)
2. Read [[canonical/17_CONTEXT_ALERTING_AND_PUBLISHING.md]] Part 5 Phase 1 for implementation plan
3. Read [[canonical/17_CONTEXT_ALERTING_AND_PUBLISHING.md]] Part 7.8 for multi-tenancy foundations
4. Set up Knock account at https://knock.app, get service token
5. Configure Knock MCP server in Claude Code settings
6. Create "new-intel-brief" workflow via MCP tools

**Do NOT:**
- Build custom email/push/Slack notification code — Knock handles all channel routing
- Store engagement config in local YAML — use D1 `engagements` table
- Hardcode user IDs — use `env.OWNER_ID` everywhere
- Build multi-tenancy infrastructure (user registration, billing, onboarding) — defer to market phase
- Use ntfy.sh for new features — legacy only (download-alert-worker still uses it)

**Key insight:**
The web app IS the push notification registration surface. Without a web origin, Web Push doesn't work. Building `app.tastematter.dev` solves three problems at once: publishing management + notification center + push registration for desktop AND mobile.
[INFERRED: from Web Push API requirements + Knock in-app feed capabilities + PWA support on iOS 16.4+]
