---
title: "Alerting & Publishing Context Package 03"
package_number: 3
date: 2026-02-16
status: current
previous_package: "[[02_2026-02-15_KNOCK_AND_WEB_APP_V4]]"
related:
  - "[[canonical/17_CONTEXT_ALERTING_AND_PUBLISHING.md]]"
  - "[[alert-worker/src/index.ts]]"
  - "[[alert-worker/src/alerting.ts]]"
  - "[[alert-worker/src/db.ts]]"
  - "[[alert-worker/src/knock.ts]]"
  - "[[alert-worker/src/types.ts]]"
  - "[[alert-worker/src/config.ts]]"
  - "[[web-app/src/routes/+layout.svelte]]"
  - "[[alert-worker/knock/workflows/new-intel-brief/workflow.json]]"
tags:
  - context-package
  - alerting-publishing
  - knock
  - web-app
  - phase-1
  - deployed
---

# Alerting & Publishing - Context Package 03

## Executive Summary

Phase 1 (Alert Worker MVP + Knock + Web App MVP) is fully implemented and deployed to production. 70/70 tests passing. Alert worker fires every 4 hours via cron, triggers Knock workflow, email lands in inbox via Resend from `alerts@tastematter.dev`. Web app live on CF Pages. Knock workflow managed as code via CLI.

## Global Context

### System Architecture (deployed)

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
  - Workflow: new-intel-brief
  - Steps: Resend email + in-app feed
  - User: "founder" → jacob@jdietle.me
          ^
          | Knock API (raw fetch, no SDK)
          |
tastematter-alert-worker.jacob-4c8.workers.dev (CF Worker)
  - Cron: 0 */4 * * *
  - D1: tastematter-alerts (4 tables)
  - Routes: /health, /alert/history, POST /alert/trigger
  - processAlertRules → evaluateRule → triggerKnockWorkflow
```

### Key Design Decisions

- **Raw fetch for Knock API in worker** (not `@knocklabs/node` SDK) — smaller bundle, no Node.js runtime assumptions [VERIFIED: [[alert-worker/src/knock.ts]]:1-40]
- **Dependency injection for triggerFn** — testable without mocking fetch [VERIFIED: [[alert-worker/src/types.ts]]:154-158]
- **Phase 1 simplified trigger** — content_change always fires (no corpus SHA diff yet) [VERIFIED: [[alert-worker/src/alerting.ts]]:29-31]
- **OWNER_ID env var** — clean seam for future multi-tenancy [VERIFIED: [[alert-worker/wrangler.toml]]:15]
- **Knock workflow as code** — managed via `knock` CLI, files in `alert-worker/knock/` [VERIFIED: [[alert-worker/knock/workflows/new-intel-brief/workflow.json]]]

## Local Problem Set

### Completed This Session

- [X] Alert worker full implementation (types, db, knock, config, alerting, index) [VERIFIED: 6 source files in alert-worker/src/]
- [X] All 60 alert-worker tests passing [VERIFIED: vitest run 2026-02-16]
- [X] D1 database created: `tastematter-alerts` (326e6f35-f971-46c9-ad6f-f332ff2dda1a) [VERIFIED: wrangler d1 create output]
- [X] D1 migrations applied remotely (4 tables: engagements, alert_history, alert_state, activity_log) [VERIFIED: wrangler d1 migrations apply --remote]
- [X] Nickel engagement seeded in D1 with alerting config [VERIFIED: wrangler d1 execute INSERT]
- [X] Worker deployed: `https://tastematter-alert-worker.jacob-4c8.workers.dev` [VERIFIED: curl /health → 200]
- [X] KNOCK_API_KEY secret set (printf, no trailing newline) [VERIFIED: wrangler secret put]
- [X] Knock CLI installed globally (`@knocklabs/cli`) [VERIFIED: knock --version → 1.0.0]
- [X] Knock workflow `new-intel-brief` created and committed via CLI [VERIFIED: knock workflow push --commit]
- [X] Knock user "founder" identified (jacob@jdietle.me) [VERIFIED: Knock API PUT /v1/users/founder → 200]
- [X] Email channel switched from knock-email to Resend (channel key: `resend`) [VERIFIED: knock channel list --json]
- [X] Email template upgraded to proper HTML (table layout, branding, footer) [VERIFIED: [[alert-worker/knock/workflows/new-intel-brief/email_1/html_body.html]]]
- [X] Email deliverability fixed — lands in inbox, not spam [VERIFIED: user confirmation 2026-02-16]
- [X] Web app scaffolded (SvelteKit + CF Pages) with all routes [VERIFIED: 14 source files in web-app/src/]
- [X] All 10 web-app tests passing [VERIFIED: vitest run 2026-02-16]
- [X] Web app deployed: `https://tastematter-web-app.pages.dev` [VERIFIED: curl → 200]
- [X] nodejs_compat flag added to web-app wrangler.toml [VERIFIED: [[web-app/wrangler.toml]]:3]
- [X] Fixed wrangler global install (broken bin link → `npm i -g wrangler`) [VERIFIED: which wrangler → ~/.npm-global/wrangler]
- [X] End-to-end trigger verified: `POST /alert/trigger` → fired:1, email in inbox [VERIFIED: curl output + user confirmation]

### Not Done (from plan)

- [ ] Push notifications — FCM project not created, no VAPID key
- [ ] CF Access on web app — currently public (no auth)
- [ ] Promote Knock workflow to production environment (currently on `development`)
- [ ] Custom domain for web app (e.g., `app.tastematter.dev`)
- [ ] Real corpus SHA comparison for content_change trigger (Phase 1 always fires)

### Jobs To Be Done (Next Session)

1. [ ] **Promote Knock workflow to production** — Currently on `development` env which uses test keys. Need `knock commit --promote` to production.
2. [ ] **CF Access on web app** — Add Cloudflare Access application for `tastematter-web-app.pages.dev` (founder email only). Currently anyone can access.
3. [ ] **Custom domain** — Point `app.tastematter.dev` to CF Pages project.
4. [ ] **Phase 2: Publishing MVP** — Port CVI worker template, `tastematter publish context` CLI command.

## File Locations

### Alert Worker (`apps/tastematter/alert-worker/`)

| File | Purpose | Status |
|------|---------|--------|
| [[alert-worker/src/types.ts]] | Env, Result<T>, row types, Knock types, TriggerFn | Complete |
| [[alert-worker/src/db.ts]] | createDB closure, 7 CRUD methods | Complete |
| [[alert-worker/src/knock.ts]] | triggerKnockWorkflow via raw fetch | Complete |
| [[alert-worker/src/config.ts]] | parseEngagementConfig with validation | Complete |
| [[alert-worker/src/alerting.ts]] | evaluateRule + processAlertRules orchestrator | Complete |
| [[alert-worker/src/index.ts]] | fetch + scheduled handlers, 4 routes | Complete |
| [[alert-worker/migrations/001_create_tables.sql]] | 4 tables (engagements, alert_history, alert_state, activity_log) | Applied |
| [[alert-worker/wrangler.toml]] | Cron, D1 binding, OWNER_ID, database_id | Deployed |
| [[alert-worker/knock/workflows/new-intel-brief/workflow.json]] | Knock workflow definition (Resend email + in-app feed) | Committed |
| [[alert-worker/knock/workflows/new-intel-brief/email_1/html_body.html]] | Proper HTML email template | Committed |
| [[alert-worker/knock.json]] | Knock CLI project config | Created |
| [[alert-worker/tests/]] | 7 test files, 60 tests total | All passing |

### Web App (`apps/tastematter/web-app/`)

| File | Purpose | Status |
|------|---------|--------|
| [[web-app/src/lib/knock.ts]] | Singleton Knock client init | Complete |
| [[web-app/src/lib/push.ts]] | SW registration, push permission, FCM token | Complete |
| [[web-app/src/lib/components/NotificationBell.svelte]] | Bell SVG + badge | Complete |
| [[web-app/src/lib/components/NotificationFeed.svelte]] | Knock Feed API wrapper | Complete |
| [[web-app/src/routes/+layout.svelte]] | Nav + Knock provider + bell | Complete |
| [[web-app/src/routes/+page.svelte]] | Dashboard with engagement list | Complete |
| [[web-app/src/routes/notifications/+page.svelte]] | Knock feed page | Complete |
| [[web-app/src/routes/settings/+page.svelte]] | Push toggle + FCM registration | Complete |
| [[web-app/static/manifest.json]] | PWA manifest | Complete |
| [[web-app/static/sw.js]] | Service worker (push + notificationclick) | Complete |
| [[web-app/svelte.config.js]] | adapter-cloudflare | Complete |
| [[web-app/wrangler.toml]] | CF Pages config + nodejs_compat | Deployed |
| [[web-app/src/tests/]] | 3 test files, 10 tests total | All passing |

## Test State

### Alert Worker: 60/60 passing
```bash
cd apps/tastematter/alert-worker && pnpm test
# 6 test files, 60 tests, ~3.5s
```

### Web App: 10/10 passing
```bash
cd apps/tastematter/web-app && pnpm test
# 3 test files, 10 tests, ~33s (happy-dom environment setup)
```

## Deployment Details

### Alert Worker
- **URL:** `https://tastematter-alert-worker.jacob-4c8.workers.dev`
- **Account:** `4c8353a21e0bfc69a1e036e223cba4d8` (personal)
- **D1:** `tastematter-alerts` / `326e6f35-f971-46c9-ad6f-f332ff2dda1a`
- **Cron:** `0 */4 * * *` (every 4 hours)
- **Secret:** `KNOCK_API_KEY` (set via printf)
- **Var:** `OWNER_ID = "founder"`

### Web App
- **URL:** `https://tastematter-web-app.pages.dev`
- **Account:** `4c8353a21e0bfc69a1e036e223cba4d8` (personal)
- **Env vars in .env:** `PUBLIC_KNOCK_PUBLIC_API_KEY`, `PUBLIC_KNOCK_FCM_CHANNEL_ID`
- **Compat flags:** `nodejs_compat`

### Knock
- **Workflow:** `new-intel-brief` (development environment)
- **Email channel:** `resend` (provider: email_resend, from: alerts@tastematter.dev)
- **In-app channel:** `in-app` (provider: in_app_feed_knock)
- **User:** `founder` → jacob@jdietle.me
- **CLI auth:** `knock login` (sessions expire quickly, need re-auth)

## Lessons Learned

### Wrangler PATH issue
- Global `wrangler` was installed via npm but had broken bin link (showed `wrangler@` with no version)
- `npx wrangler` runs ephemeral copy — still reads `~/.wrangler/config/default.toml` for auth
- Fix: `npm install -g wrangler` to recreate bin symlink
- [VERIFIED: which wrangler → /c/Users/dietl/.npm-global/wrangler after reinstall]

### Knock email deliverability
- Knock's built-in email (`knock-email` / `email_knock_limited`) → lands in spam (shared IP, no DKIM for your domain)
- Resend channel with `tastematter.dev` domain (DKIM + SPF + DMARC configured) → still landed in spam with bare `<p>{{ body }}</p>` template
- Proper HTML email template (table layout, branding, footer) → lands in inbox
- Root cause was email content quality, not infrastructure [VERIFIED: user confirmation after template upgrade]

### Knock CLI auth
- `knock login` sessions expire quickly (< 30 min observed)
- Secret API key (`sk_test_...`) is NOT a service token — CLI rejects it
- Service tokens are separate credentials generated in Knock dashboard
- For CI/CD, use service tokens. For interactive use, `knock login` and re-auth as needed.

### CF Pages deployment
- `wrangler pages deploy` needs `--branch main` to deploy to production URL
- Pages projects don't support `account_id` in wrangler.toml — use `CLOUDFLARE_ACCOUNT_ID` env var
- First deploy requires `wrangler pages project create` before `pages deploy`
- Svelte 5 on CF Pages needs `nodejs_compat` compatibility flag (node:async_hooks)

## For Next Agent

**Context Chain:**
- Previous: [[02_2026-02-15_KNOCK_AND_WEB_APP_V4]] (spec v4 complete, ready for implementation)
- This package: Phase 1 fully deployed and working
- Next action: Promote Knock to production, add CF Access, custom domain

**Start here:**
1. Read this context package
2. Read [[canonical/17_CONTEXT_ALERTING_AND_PUBLISHING.md]] for full spec (Phase 2+ details)
3. Run `cd apps/tastematter/alert-worker && pnpm test` to verify state
4. Run `curl https://tastematter-alert-worker.jacob-4c8.workers.dev/health` to verify deployment

**Do NOT:**
- Use `npx wrangler` — use global `wrangler` (auth is in `~/.wrangler/config/default.toml`)
- Use `echo` for wrangler secrets — always `printf` (no trailing newline)
- Put `account_id` in Pages wrangler.toml — use `CLOUDFLARE_ACCOUNT_ID` env var
- Assume Knock CLI sessions persist — re-auth with `knock login` if commands fail with bearer_token_invalid
