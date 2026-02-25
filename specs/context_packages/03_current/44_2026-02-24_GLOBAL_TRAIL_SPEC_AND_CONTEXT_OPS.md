---
title: "Tastematter Context Package 44"
package_number: 44
date: 2026-02-24
status: current
previous_package: "[[43_2026-02-24_WAVE2_LAUNCH_DECISION_AND_OUTREACH_WORKER_DESIGN]]"
related:
  - "[[canonical/08_GLOBAL_TRAIL_SPEC]]"
  - "[[06_products/tastematter/strategy/context-ops-offering]]"
  - "[[canonical/07_CLAUDE_CODE_DATA_MODEL_V2]]"
  - "[[canonical/03_CORE_ARCHITECTURE]]"
  - "[[04_knowledge_base/methodology/stigmergy]]"
  - "[[_system/specs/architecture/context_operating_system/04_GIT_STIGMERGY_FOUNDATION]]"
tags:
  - context-package
  - tastematter
  - global-trail
  - context-ops
  - naming
---

# Tastematter - Context Package 44

## Executive Summary

Designed and spec'd the **Global Trail** feature — a Cloudflare D1-backed sync layer that lets tastematter's local SQLite trail (sessions, chains, heat, co-access) sync across machines. This emerged from setting up a Hetzner VPS for parallel Claude Code sessions and realizing context intelligence was trapped on the Windows laptop. Also defined **Context Ops** as a named product/service offering and established **"trail"** as the core naming primitive (with lineage to Bush's 1945 Memex).

## Session Arc

This was a long strategic session that went:
1. VPS setup (Hetzner CCX33, Ubuntu 24.04, Tailscale-only access)
2. Realized context migration problem (months of indexed intelligence on Windows)
3. Evaluated options: raw migration vs sync vs shared VPS
4. Connected to existing stigmergy theory and positioning work
5. Named the concept: **local trail** (per-machine SQLite) vs **global trail** (D1)
6. Wrote Context Ops offering doc
7. Wrote Global Trail technical spec

## Key Decisions Made

### 1. "Trail" as Core Naming Primitive
- **Local trail** = per-machine SQLite database (`~/.context-os/context_os_events.db`)
- **Global trail** = D1 database synced across machines/team
- Lineage: Vannevar Bush's Memex (1945) literally called them "trails" — associative paths through knowledge, shareable between people
- Maps to stigmergy: trails = pheromone trails agents leave behind
- CLI commands: `tastematter trail push`, `trail pull`, `trail status`
- [INFERRED: from naming analysis applying Vignelli "one concept, one word" + Bush Memex connection]

### 2. Context Ops as Product Offering
- Sits between free CLI and $7K+ consulting
- Three components: git automation + team D1 sync + private→published scope
- Resolves dq_002 (monetization): infrastructure value, not consulting value
- Revenue: per-seat/month SaaS + managed service retainer
- [VERIFIED: [[06_products/tastematter/strategy/context-ops-offering.md]]]

### 3. Private→Published Scope Model
- Local trail stays private (experiments, drafts, personal sessions)
- User explicitly publishes to global trail (curated, intentional)
- Maps to git model: local repo vs remote (why devs use GitHub, not shared machines)
- Team members control what enters shared brain
- [INFERRED: from analogy with git collaboration model + stigmergy selective signal emission]

### 4. VPS Architecture
- Hetzner CCX33: 8 vCPU, 32GB RAM, 240GB SSD, Hillsboro OR
- Tailscale-only access (100.109.204.117), public IP fully firewalled
- SSH hardened: no root, no passwords, keys only, ListenAddress on Tailscale
- UFW + Hetzner firewall (belt-and-suspenders)
- fail2ban, unattended-upgrades with 4 AM auto-reboot
- Node.js, Claude Code, tmux, git, gh installed
- Phone access via Termius + Tailscale
- [VERIFIED: VPS setup session output, all verification tests passed]

## Artifacts Created

| File | Type | Description |
|------|------|-------------|
| [[canonical/08_GLOBAL_TRAIL_SPEC]] | Tech spec | Full D1 schema, CF Worker, CLI commands, path normalization, deployment steps |
| [[06_products/tastematter/strategy/context-ops-offering]] | Strategy doc | Offering definition connecting positioning + architecture + revenue model |

## Global Trail Spec Summary

### Architecture
```
PUSH: local SQLite → normalize paths → POST to CF Worker → D1
PULL: GET from CF Worker → D1 query → write to local SQLite
AUTH: CF Access service tokens (same pattern as Nickel)
```

### What Syncs
- claude_sessions, chain_graph, chain_metadata, chain_summaries, chains
- file_access_events, file_edges, git_commits
- All with `source_machine` column for multi-machine tracking

### What Doesn't Sync
- Raw JSONL files (1.2 GB, machine-specific)
- debug/ logs, file-history/ snapshots, tool-results/ overflow

### Key Technical Decisions
- Natural keys (session_id UUID, commit hash) — no collisions across machines
- INSERT OR REPLACE for metadata, INSERT OR IGNORE for events
- Path normalization: Windows backslash → Unix forward slash, strip drive letter
- Incremental sync via `_metadata.last_trail_push` timestamp
- D1 stores Unix paths always

### Estimated Build: ~10 hours
1. Deploy CF Worker + D1 (1h)
2. Build `trail push` in Rust (4h)
3. Build `trail pull` in Rust (2h)
4. Build `trail status` (1h)
5. Test end-to-end (1h)
6. Add incremental sync (1h)

## Context Ops Offering Summary

### The Demand Signal
- Every consulting client asks "how do we make this work across the team?"
- Rula's Austin Pogue explicitly needs internal pitch for sales leadership
- cli.py incident (Dec 2025) proved local-remote drift needs automation, not discipline
- Multi-machine pain felt tonight when VPS had zero context

### Revenue Model
| Tier | What | Price |
|---|---|---|
| Free | CLI + local trail + daemon | $0 |
| Team Sync | D1 per team + git automation + private→published scope | $X/seat/month |
| Managed | We deploy + maintain the stack | Retainer |
| Consulting | Architecture + build from scratch | $7K+ |

### How It Maps to Existing Architecture
- 3-layer model from [[05_PRODUCT_ARCHITECTURE]]: Personal → Team → Company
- Stigmergy: git automation = automated pheromone deposit, publish = selective signal emission
- Roadmap: collapses Phases 2, 4, 5 into single offering

## Connections Established

| Existing Concept | New Connection |
|---|---|
| Stigmergy (pheromone trails) | = "trail" naming + trail push/pull mechanics |
| 3-layer model (05_PRODUCT_ARCHITECTURE) | = local trail / global trail / company trail |
| Private→published scope | = git's local/remote model applied to context |
| "Company brain" (positioning) | = global trail queryable by whole team |
| Master record vs photocopier (positioning) | = local = working copy, global = master record |
| Bush's Memex (1945) | = "trail" is the historically correct word for this |

## For Next Agent

**Context Chain:**
- Previous: [[43_2026-02-24_WAVE2_LAUNCH_DECISION_AND_OUTREACH_WORKER_DESIGN]]
- This package: Global Trail spec + Context Ops offering + VPS setup
- Next action: Build Global Trail (start with CF Worker + D1 deployment)

**Start here:**
1. Read [[canonical/08_GLOBAL_TRAIL_SPEC]] — full implementation spec
2. Read [[06_products/tastematter/strategy/context-ops-offering]] — business context
3. Deploy using steps in spec Section "Deployment Steps"

**Build order:**
1. `cp cf-worker-scaffold → apps/tastematter/trail-worker/`
2. `wrangler d1 create tastematter-trail`
3. Apply migration SQL from spec
4. Deploy Worker
5. Build `trail push` in Rust core (extend main.rs with trail subcommand)
6. Build `trail pull` in Rust core
7. Test: push from Windows → pull on VPS → `tastematter context "tastematter"`

**VPS access:**
- SSH: `ssh jacob@100.109.204.117` (Tailscale, key auth)
- Phone: Termius with imported key + Tailscale active
- tmux for persistent sessions: `tmux new -s work`

**Do NOT:**
- Migrate raw JSONL files (too large, machine-specific paths)
- Build team features yet (single-user prototype first)
- Build git automation yet (separate feature, after trail sync works)

**Key insight:**
The trail captures what git doesn't — the WHY behind changes, the decision context, the work patterns. Git tracks state. The trail tracks intent. Together (git clone + trail pull) = full context restoration on any machine. [INFERRED: from session discussion + Bush Memex connection]
