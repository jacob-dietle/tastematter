---
title: "Tastematter Context Package 43"
package_number: 43
date: 2026-02-24
status: current
previous_package: "[[42_2026-02-18_TEMPORAL_EDGES_QUALITY_REFINEMENT_COMPLETE]]"
related:
  - "[[06_products/tastematter/CLAUDE.md]]"
  - "[[06_products/tastematter/_synthesis/gtm-summary.md]]"
  - "[[06_products/tastematter/launch/beta-launch-plan.md]]"
  - "[[~/.claude/plans/noble-bubbling-plum.md]]"
tags:
  - context-package
  - tastematter
  - gtm
  - outreach-worker
---

# Tastematter - Context Package 43

## Executive Summary

GTM launch decision made: go public this week. Publish-filter analysis completed — open the gates (remove access restrictions), DM targeted outreach list, but save the promotional megaphone (LinkedIn announcement, HN, demo clip) for when analytics visuals + feedback mechanism exist. Architecture designed and approved for a Kondo-webhook-driven outreach tracking worker (CF Worker + D1).

## Session Activity

This was a **strategic planning + architecture design** session, not implementation. No code was written.

### Decisions Made

1. **Launch strategy decided:** Public release NOW + targeted DM outreach this week. Promotional launch (LinkedIn post, demo clip, HN) deferred until Wave 2 features exist (feedback mechanism + analytics visuals).

2. **Publish-filter analysis:** CLI = the chocolate bar (result), not the recipe. The moat is cognitive effects (data depth over time), not CLI code. No NDA concerns. No competitive recipe exposure. Gating a distribution layer is self-defeating. [INFERRED: publish-filter skill framework applied to tastematter GTM context]

3. **Outreach tracker architecture approved:** CF Worker + D1, Kondo webhook integration, labels-as-state-machine pattern. Plan at `~/.claude/plans/noble-bubbling-plum.md`.

### Key Insight

> "The thing you're calling 'launch' is actually two things: opening the gates and turning on the megaphone. Open the gates now. Save the megaphone."

Separating "publicly installable" from "actively promoted" resolves the tension between shipping now and having the full Wave 2 experience.

## Outreach Worker Architecture (APPROVED, NOT IMPLEMENTED)

### Design Summary

- **Location:** `apps/tastematter/outreach-worker/`
- **Stack:** CF Worker + D1 (personal account)
- **Data flow:** Kondo webhook (on label/message/note change) → Worker → D1 upsert
- **Key pattern:** Kondo labels (`tm-wave2`, `tm-contacted`, `tm-installed`, `tm-feedback`) drive pipeline status transitions automatically

### D1 Schema (3 app tables + 2 base tables from scaffold)

| Table | Purpose |
|-------|---------|
| `contacts` | Core contact tracking — linkedin_url (UNIQUE), status state machine, wave, source, kondo metadata |
| `outreach_events` | Append-only event audit trail — contact_id FK, event_type, event_data JSON |
| `webhook_log` | Raw webhook payloads for debugging/replay |
| `flow_logs` | Standard scaffold observability |
| `flow_health` | Standard scaffold health summary |

### API Endpoints

| Method | Path | Purpose |
|---|---|---|
| POST | `/webhook` | Kondo webhook receiver |
| POST | `/contacts/batch` | Batch import (for commenter list) |
| GET | `/dashboard` | Pipeline summary (counts per status per wave) |
| GET | `/contacts` | List with filters |
| PATCH | `/contacts/:id` | Manual status override |

### Kondo Integration

- **Tier:** Business (confirmed — webhooks available)
- **Webhook payload fields:** LinkedIn URL, headline, location, labels, notes, Kondo URL, latest message, conversation history, timestamp
- **Trigger modes:** Streaming (auto on changes) or Manual (Cmd+K → Sync)
- **Docs:** [docs.trykondo.com/webhooks](https://docs.trykondo.com/webhooks)

### Full Plan Location

`~/.claude/plans/noble-bubbling-plum.md` — contains complete schema SQL, endpoint specs, webhook handler logic, file structure, deployment steps, and verification checklist.

## Epistemic State (What's Verified vs Not)

### VERIFIED
- CLI v0.1.0-alpha.15 shipped, 287 tests passing [VERIFIED: workstreams.yaml:42]
- Wave 1 complete: 3 alpha users, 1 super responsive, Victor posted publicly [VERIFIED: beta-launch-plan.md:43-48]
- Kondo Business tier confirmed by user [VERIFIED: user confirmation this session]
- Kondo webhooks expose: LinkedIn URL, labels, notes, messages, timestamps [VERIFIED: docs.trykondo.com/webhooks]
- Wave 2 features (feedback mechanism, analytics visuals) have NO implementation in core/src [VERIFIED: grep returned empty]
- Recent technical work was temporal edges (file_edges.rs, index/mod.rs), NOT Wave 2 P0s [VERIFIED: heat query q_2c5472]

### INFERRED
- Opening gates without promoting is the right sequencing [INFERRED: publish-filter framework + DIY competitor timing pressure + Wave 1 validation signal]
- Kondo labels as state machine driver is zero-friction [INFERRED: user already uses Kondo daily for DMs, adding labels is natural workflow]

### UNVERIFIABLE
- LinkedIn commenter list size (need to manually scroll posts)
- Actual install conversion rate for Wave 2

## Workstream State Snapshot

| Stream | Temp | Key State |
|---|---|---|
| rula-engagement | HOT | Internal evangelism phase — storytelling for exec buy-in |
| productization-sprint | HOT | P0 — enables scaling strategy, 5% progress |
| tastematter-gtm | WARM→HOT | This session activated it — launch this week |
| tastematter-cli | SHIPPED | v0.1.0-alpha.15, 287 tests |
| nickel-transcript | WARM | All 4 providers complete, 220 tests |
| pixee-linkedin | WARM | Code complete, blocked on Pixee CF credentials |

**Pipeline:** $13K committed MRR (Pixee $6K + Ivan $7K), Rula at 85% ($15K/mo)

## Jobs To Be Done (Next Session)

### P0: Build Outreach Worker
1. [ ] Scaffold from `00_foundation/services/templates/cf-worker-scaffold/`
2. [ ] Write D1 migrations (base + outreach tables)
3. [ ] Implement worker: types.ts, auth.ts, logging.ts, webhook.ts, contacts.ts, dashboard.ts, events.ts, index.ts
4. [ ] Create D1 database: `wrangler d1 create tastematter-outreach`
5. [ ] Apply migrations with `--remote`
6. [ ] Set secrets (CF Access tokens + webhook secret)
7. [ ] Deploy and verify health endpoint
8. [ ] Configure Kondo webhook URL

### P0: Start Wave 2 Outreach
9. [ ] Remove any access gates on install.tastematter.dev
10. [ ] Manually identify LinkedIn commenters from recent posts
11. [ ] Batch import commenter list via POST /contacts/batch
12. [ ] Begin DM outreach (apply `tm-wave2` then `tm-contacted` labels in Kondo)

### P1: Wave 2 Product Features (in parallel with outreach)
13. [ ] Build `tastematter feedback` command (GitHub issue creation)
14. [ ] Build `tastematter stats --visual` (terminal art)
15. [ ] Record 90-second demo clip (blocked on visuals)

## For Next Agent

**Context Chain:**
- Previous: [[42_2026-02-18_TEMPORAL_EDGES_QUALITY_REFINEMENT_COMPLETE]]
- This package: GTM launch decision + outreach worker design (no code written)
- Next action: Build the outreach worker from approved plan

**Start here:**
1. Read this context package
2. Read plan: `~/.claude/plans/noble-bubbling-plum.md` (full architecture + schema + deployment steps)
3. Check scaffold template: `00_foundation/services/templates/cf-worker-scaffold/`
4. Begin scaffolding: copy template to `apps/tastematter/outreach-worker/`

**Do NOT:**
- Skip the scaffold template — it has proven patterns (auth, logging, flow tracking)
- Use `echo` for wrangler secrets — always `printf` (no trailing newline)
- Forget `--remote` flag for D1 migrations (without it = local SQLite only)
- Build a custom domain — workers.dev with CF Access is sufficient for now
- Build cron jobs — this is webhook-driven, not polling

**Key files:**
- Plan: `~/.claude/plans/noble-bubbling-plum.md`
- GTM synthesis: `06_products/tastematter/_synthesis/gtm-summary.md`
- Beta launch plan: `06_products/tastematter/launch/beta-launch-plan.md`
- Workstreams: `_system/state/workstreams.yaml` (tastematter-gtm stream)
- Scaffold template: `00_foundation/services/templates/cf-worker-scaffold/`
