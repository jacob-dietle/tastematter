---
title: "Tastematter Context Package 05 - Vision Foundation"
package_number: 05

migrated_from: "apps/tastematter/specs/context_packages/05_2026-01-07_VISION_FOUNDATION.md"
status: current
previous_package: "[[04_2026-01-06_PERF_OPTIMIZATION_COMPLETE]]"
foundation:
  - "[[_system/specs/architecture/context_operating_system/04_GIT_STIGMERGY_FOUNDATION.md]]"
  - "[[_system/specs/architecture/context_operating_system/05_PRODUCT_ARCHITECTURE.md]]"
  - "[[_system/specs/architecture/context_operating_system/06_INTELLIGENT_GITOPS_SPEC.md]]"
related:
  - "[[04_knowledge_base/methodology/stigmergy.md]]"
  - "[[04_knowledge_base/technical/context-worker-agent-pattern.md]]"
  - "[[04_knowledge_base/methodology/evidence-attribution-system.md]]"
  - "[[.claude/skills/context-operating-system/skill.md]]"
  - "[[specs/07_CHAIN_INTEGRATION_SPEC.md]]"
  - "[[specs/08_UNIFIED_DATA_ARCHITECTURE.md]]"
  - "[[specs/09_LOGGING_SERVICE_SPEC.md]]"
  - "[[specs/10_PERF_OPTIMIZATION_SPEC.md]]"
tags:
  - context-package
  - tastematter
  - vision
  - architecture
  - mega-package
---

# Tastematter - Context Package 05: Vision Foundation

## Executive Summary

This mega context package establishes the **complete vision** for Tastematter by synthesizing foundational specs and knowledge base concepts. Tastematter is NOT a visualization tool - it is **Level 2 of the Context Operating System architecture**: the Tauri desktop app that enables human-agent coordination through stigmergic git infrastructure. This package serves as both context preservation and the foundation for creating canonical vision/principles/roadmap documents.

**Session accomplishments:**
- Consolidated specs directory (commit `628ab07`)
- Read and synthesized 6 foundational documents
- Established complete vision framework
- Created plan for canonical docs (`specs/canonical/`)

**Next agent job:** Create `specs/canonical/` directory with `00_VISION.md`, `01_PRINCIPLES.md`, `02_ROADMAP.md`

---

## The Breakthrough: Tastematter's True Identity

### What Tastematter IS

> **Purpose Statement:** Tastematter is the human interface to Context Operating Systems - enabling immediate visibility into attention patterns, seamless resumption of context, agent-augmented exploration, and stigmergic coordination with intelligent agents.

[INFERRED: From synthesis of [[04_GIT_STIGMERGY_FOUNDATION.md]], [[05_PRODUCT_ARCHITECTURE.md]], [[06_INTELLIGENT_GITOPS_SPEC.md]]]

### What Tastematter IS NOT

- NOT a standalone visualization tool [VERIFIED: [[06_INTELLIGENT_GITOPS_SPEC.md]]:73-79 - Level 2 is multi-repo + desktop app]
- NOT just for viewing file access patterns [INFERRED: Vision includes agent-controllable UI, git ops integration]
- NOT a replacement for git [VERIFIED: [[04_GIT_STIGMERGY_FOUNDATION.md]]:174 - "Git is coordination infrastructure"]

---

## Position in Context OS Stack

```
┌─────────────────────────────────────────────────────────────────┐
│  LEVEL 3: Inter-OS Protocols (FUTURE)                          │
│           Context as a service, MCP publishing, pay-walling    │
│           [VERIFIED: [[06_INTELLIGENT_GITOPS_SPEC.md]]:86-96]  │
├─────────────────────────────────────────────────────────────────┤
│  LEVEL 2: Multi-Repo Desktop App ← TASTEMATTER IS HERE         │
│           Tauri app, multi-repo management, agent UI control   │
│           [VERIFIED: [[06_INTELLIGENT_GITOPS_SPEC.md]]:73-79]  │
├─────────────────────────────────────────────────────────────────┤
│  LEVEL 1: Team Coordination                                    │
│           GitHub webhooks, Cloudflare Workers, conflict detect │
│           [VERIFIED: [[06_INTELLIGENT_GITOPS_SPEC.md]]:52-66]  │
├─────────────────────────────────────────────────────────────────┤
│  LEVEL 0: Personal CLI ← ALREADY BUILT (context-os)            │
│           CLI + daemon, promptable rules, intelligent commits  │
│           [VERIFIED: [[06_INTELLIGENT_GITOPS_SPEC.md]]:35-49]  │
│           [VERIFIED: apps/context_os_events/ exists]           │
└─────────────────────────────────────────────────────────────────┘
```

---

## The Git-Stigmergy Foundation

### Core Insight

> **Git is not version control. Git is coordination infrastructure.**
> [VERIFIED: [[04_GIT_STIGMERGY_FOUNDATION.md]]:174]

### The Isomorphism

| Git Primitive | Stigmergy Concept | Source |
|---------------|-------------------|--------|
| Commit (snapshot) | Environment state | [[04_GIT_STIGMERGY_FOUNDATION.md]]:37-47 |
| Diff (computed) | Inferred action | [[04_GIT_STIGMERGY_FOUNDATION.md]]:20-23 |
| Push/pull | State propagation | [[04_GIT_STIGMERGY_FOUNDATION.md]]:266-268 |
| Commit author | Attribution | [[04_GIT_STIGMERGY_FOUNDATION.md]]:45 |
| `git log --since` | Signal decay | [[04_GIT_STIGMERGY_FOUNDATION.md]]:44 |

### Why This Matters for Tastematter

Agents and humans coordinate by **reading and writing to git repos**, not by direct messaging. The repo IS the communication medium.

From [[stigmergy.md]]:49-67:
> "Agents read shared state files (pipeline.yaml, directory structure). Agents modify environment (create files, update state). Other agents respond to those modifications. Coordination emerges without direct agent-to-agent communication."

**Tastematter's role:** Enable humans to SEE the git state, RESPOND to agent modifications, and COORDINATE through the stigmergic substrate.

---

## The Five Non-Negotiable Principles

### Principle 1: IMMEDIATE

**Source:** [[05_PRODUCT_ARCHITECTURE.md]]:154-159 ("Feels like fucking magic")

**Requirement:** <100ms for any navigation

**Why:** Latency breaks the stigmergic feedback loop:
- Agent commits change
- Human needs to SEE that change immediately
- Human responds (accepts, modifies, overrides)
- Agent sees human's response
- Coordination emerges

**Current violation:** 5-second view switches [VERIFIED: user report in session]

**From [[05_PRODUCT_ARCHITECTURE.md]]:155-159:**
> "The experience should be: Effortless (files just appear where they should), Surprising (how did it know to do that?), Trustworthy (but I understand why it did that)"

---

### Principle 2: STIGMERGIC

**Source:** [[04_GIT_STIGMERGY_FOUNDATION.md]], [[stigmergy.md]]

**Requirement:** Shows git state, highlights agent modifications, enables human response

**Why:** Git is the coordination substrate. Tastematter must surface:
- What changed (git commits)
- Who changed it (human vs agent attribution)
- When (timeline of modifications)
- So human can respond and complete the coordination loop

**From [[04_GIT_STIGMERGY_FOUNDATION.md]]:309-313:**
> "Git is your coordination layer - Don't build custom sync, use git. Agents read and write state - They don't send messages to each other. Coordination emerges - You don't orchestrate, you set up the environment."

---

### Principle 3: MULTI-REPO AWARE

**Source:** [[06_INTELLIGENT_GITOPS_SPEC.md]]:73-79, [[05_PRODUCT_ARCHITECTURE.md]]:259-291

**Requirement:** Manage Personal → Team → Company layers

**Why:** Context OS operates at multiple scales:

```
Personal Context OS (your repo)
        │
        │ push/pull
        ▼
Team Context OS (shared repo)
        │
        │ push/pull
        ▼
Company Context OS (org repo)
```
[VERIFIED: [[04_GIT_STIGMERGY_FOUNDATION.md]]:253-268]

**From [[05_PRODUCT_ARCHITECTURE.md]]:291:**
> "Key principle: Each layer has its own stigmergic coordination. Propagation between layers is selective, not automatic."

---

### Principle 4: AGENT-CONTROLLABLE

**Source:** User vision + [[context-worker-agent-pattern.md]] + [[06_INTELLIGENT_GITOPS_SPEC.md]]

**Requirement:** Pre-built guardrailed UI states that agents can invoke

**Why:** 10x human control via agent augmentation

**User's vision (from session):**
> "I envision the tastematter app to be used by the human user to augment their ability to control their agents, which can use tastematter via the command line interface. Eventually I would like to be able to have an agent control the UI so the user can 10x their control/depth of vision by just telling the agent and it uses the cli hooked up to the app with partially prebuild (think effective guardrails to ensure elegant/simple and effective design) UIs"

**Guardrails concept (from [[context-worker-agent-pattern.md]]:404-449):**
- Agent can't create arbitrary UI
- Agent can only invoke pre-defined view states/transitions
- This keeps the UI elegant and prevents agent-generated chaos

**Protocol sketch:**
```
Human: "Show me everything related to the Pixee chain from last week"

Agent executes:
  1. context-os query chains --name "pixee" → gets chain_id
  2. context-os ui navigate --view timeline --chain {chain_id} --time 7d
  3. context-os ui highlight --related-files

Tastematter: Animates to timeline view, highlights Pixee chain, shows connections
```

---

### Principle 5: INVESTMENT NOT RENT

**Source:** [[05_PRODUCT_ARCHITECTURE.md]]:125-148

**Requirement:** User owns all data (their git repos)

| Rent Extraction | Investment |
|-----------------|------------|
| Value locked in vendor's system | Value locked in user's system |
| Switching cost = data migration | Switching cost = learning curve only |
| Vendor captures appreciation | User captures appreciation |
| User is customer | User is owner |

[VERIFIED: [[05_PRODUCT_ARCHITECTURE.md]]:139-147]

**From [[05_PRODUCT_ARCHITECTURE.md]]:148:**
> "The pitch: 'We help your knowledge compound. You keep the compounded value.'"

---

## The Vision-Aligned Roadmap

### Current State

**Implemented (from specs 07-10):**
- Chain integration (Spec 07) [VERIFIED: ChainNav.svelte exists]
- Unified data architecture (Spec 08) [VERIFIED: context.svelte.ts exists]
- Logging service (Spec 09) [VERIFIED: LogService exists]
- Performance optimization (Spec 10) [VERIFIED: commit f9c0729]

**Test state:** 236 TypeScript + 6 Rust tests passing [VERIFIED: npm test, cargo test 2026-01-06]

**Current limitation:** 5-second view switches [VERIFIED: user report]

---

### Phase 0: Performance Foundation

**Principle violated:** Immediate (<100ms)

**Tasks:**
1. Replace Python CLI calls with Rust-native queries in Tauri
2. Implement background indexer (pre-compute on git events)
3. Cache sessions, chains, file rankings on ingest
4. Add progressive loading (show shape first, details on demand)

**Success criteria:** <100ms for any view switch

**Dependencies:** None - this unblocks everything else

**From [[06_INTELLIGENT_GITOPS_SPEC.md]]:274-276:**
> Technology for Level 2: "Tauri (Rust backend + web UI, small binaries, cross-platform)"
> "Note: Agentic coding means you don't need to learn Rust - you spec it, agents build it."

---

### Phase 1: Stigmergic Display

**Principle addressed:** Stigmergic

**Tasks:**
1. Show git commit timeline (who modified what, when)
2. Differentiate agent vs human commits (badges/colors)
3. "What changed since I last looked?" view
4. Enable human to respond to agent modifications (approve/reject)

**Success criteria:** Human can see agent activity and respond within the stigmergic loop

**From [[context-worker-agent-pattern.md]]:198-210:**
> "The created directory becomes a signal. Next transcript with CVI participants will find existing directory and route directly. No message passing needed."
> This is what humans need to SEE.

---

### Phase 2: Multi-Repo Dashboard

**Principle addressed:** Multi-Repo Aware

**Tasks:**
1. Repo selector (switch between context OS repos)
2. Unified timeline across repos (filtered or combined view)
3. Cross-repo search
4. Status indicators per repo (clean/dirty/behind remote)

**Success criteria:** Manage Personal → Team layers from single interface

**From [[05_PRODUCT_ARCHITECTURE.md]]:259-291:** Layered Context OS Vision diagram

---

### Phase 3: Agent UI Control Protocol

**Principle addressed:** Agent-Controllable

**Tasks:**
1. Define finite set of UI states (views + filters + selections)
2. Create CLI command: `context-os ui navigate --view X --filters Y`
3. Tastematter listens and animates to requested state
4. Agent can query current UI state: `context-os ui state`
5. Document guardrails (what agent CAN'T do)

**Success criteria:** Agent can invoke any pre-defined view state via CLI

**From [[06_INTELLIGENT_GITOPS_SPEC.md]]:138-142:**
```
$ context-os status
$ context-os commit
$ context-os watch
$ context-os rules
```
Add: `$ context-os ui navigate --view timeline --chain X`

---

### Phase 4: Intelligent GitOps Integration

**Principle addressed:** All (this is the coordination layer)

**Tasks:**
1. Connect to Level 0 daemon (watches for changes, suggests commits)
2. Surface daemon notifications in Tastematter sidebar
3. One-click approve/reject agent-suggested commits
4. Promptable rules UI (edit `~/.context-os/rules.yaml`)

**Success criteria:** Git hygiene automated, human approves suggestions

**From [[06_INTELLIGENT_GITOPS_SPEC.md]]:249-259:** Promptable Rules
```yaml
rules:
  - "Commit knowledge_base/ changes within 1 hour of modification"
  - "Never auto-commit files in _system/state/ - always ask first"
  - "If I haven't committed in 3 days, send me a notification"
```

---

### Phase 5: MCP Publishing (Future)

**Principle addressed:** Investment Not Rent (externalize value)

**Tasks:**
1. Select which repos/paths to publish as MCP server
2. Auth configuration (who can access what)
3. Pay-walling integration (monetize context)
4. MCP server management UI

**Success criteria:** Context shareable as auth-walled MCP service

**From [[06_INTELLIGENT_GITOPS_SPEC.md]]:86-96:**
> "Context as a service, Context streaming between systems, Standardized APIs for context exchange, Federation between context operating systems"

---

## Additional User Requirements (From Session)

Beyond the core vision, user specified additional goals:

1. **Automate git ops overhead with intelligent agent**
   - Addressed by Phase 4 (Intelligent GitOps Integration)
   - [[06_INTELLIGENT_GITOPS_SPEC.md]] is the spec

2. **Manage different context OS repos with ease and grace**
   - Addressed by Phase 2 (Multi-Repo Dashboard)
   - [[05_PRODUCT_ARCHITECTURE.md]]:259-291 is the architecture

3. **Eventually easy publish context as MCP with auth/pay walling**
   - Addressed by Phase 5 (MCP Publishing)
   - This is Level 3 in the GitOps spec

---

## Context Worker Agent Pattern (For Agent UI Control)

When implementing Phase 3, follow the 5-component pattern from [[context-worker-agent-pattern.md]]:56-66:

1. **Context Injection** - UI state injected into agent prompt
2. **Tools** - Read (query current state) + Write (navigate to state)
3. **Classification with Confidence** - Agent decides which view fits user intent
4. **Environment Modification** - Agent invokes UI state change
5. **Prompt Externalization** - UI command syntax in external file

**Anti-pattern from [[context-worker-agent-pattern.md]]:404-430:**
- Don't give agent full UI control (arbitrary DOM manipulation)
- Don't let agent generate new UI elements
- Do provide finite set of guardrailed states

---

## Evidence Attribution System (For All Docs)

When creating canonical docs, follow [[evidence-attribution-system.md]]:52-120:

**Three levels:**
1. `[VERIFIED: source:line]` - Direct evidence from specific file
2. `[INFERRED: logic from sources]` - Deduced from multiple sources
3. `[UNVERIFIABLE: reason]` - Cannot confirm

**Quality standard from [[evidence-attribution-system.md]]:323-329:**
- VERIFIED claims: >60%
- INFERRED claims: 30-40%
- UNVERIFIABLE gaps: <10%

---

## File Locations

### Source Specs (Read for Context)

| File | Purpose | Key Content |
|------|---------|-------------|
| [[_system/specs/architecture/context_operating_system/04_GIT_STIGMERGY_FOUNDATION.md]] | Git as coordination | Lines 24-49: isomorphism table |
| [[_system/specs/architecture/context_operating_system/05_PRODUCT_ARCHITECTURE.md]] | Product vision | Lines 154-178: UX principles |
| [[_system/specs/architecture/context_operating_system/06_INTELLIGENT_GITOPS_SPEC.md]] | Level 0-3 stack | Lines 28-34: level progression |
| [[04_knowledge_base/methodology/stigmergy.md]] | Theory | Lines 50-67: definition |
| [[04_knowledge_base/technical/context-worker-agent-pattern.md]] | Implementation | Lines 56-66: 5 components |
| [[04_knowledge_base/methodology/evidence-attribution-system.md]] | Attribution | Lines 59-120: three levels |
| [[.claude/skills/context-operating-system/skill.md]] | Build patterns | Lines 95-125: two-layer architecture |

### Tastematter Specs (Already Exist)

| File | Purpose | Status |
|------|---------|--------|
| [[specs/07_CHAIN_INTEGRATION_SPEC.md]] | Chain nav | Implemented |
| [[specs/08_UNIFIED_DATA_ARCHITECTURE.md]] | Shared context | Implemented |
| [[specs/09_LOGGING_SERVICE_SPEC.md]] | JSONL logging | Implemented |
| [[specs/10_PERF_OPTIMIZATION_SPEC.md]] | 6 performance fixes | Implemented |

### To Be Created (Next Agent)

| File | Purpose | Template |
|------|---------|----------|
| [[specs/canonical/00_VISION.md]] | What Tastematter IS | See plan file |
| [[specs/canonical/01_PRINCIPLES.md]] | 5 non-negotiable principles | See plan file |
| [[specs/canonical/02_ROADMAP.md]] | 6-phase development | See plan file |

---

## Session Accomplishments

### Directory Cleanup (Commit `628ab07`)
- [X] Moved spec 11 → 10 (renumbered) [VERIFIED: git log]
- [X] Moved context packages 02-04 to single directory [VERIFIED: ls specs/context_packages/]
- [X] Fixed broken package chain (02 → 01) [VERIFIED: frontmatter]
- [X] Updated README with full timeline [VERIFIED: README.md]

### Vision Synthesis
- [X] Read 6 foundational documents [VERIFIED: this package lists all]
- [X] Identified Tastematter position in Level 0-3 stack [VERIFIED: this package]
- [X] Derived 5 non-negotiable principles [VERIFIED: this package]
- [X] Created vision-aligned 6-phase roadmap [VERIFIED: this package]
- [X] Mapped user requirements to roadmap phases [VERIFIED: this package]

### Plan Created
- [X] Plan file at `C:\Users\dietl\.claude\plans\ethereal-inventing-river.md` [VERIFIED: file exists]
- [X] Defines structure for canonical docs [VERIFIED: plan content]

---

## For Next Agent

### Context Chain
- Previous: [[04_2026-01-06_PERF_OPTIMIZATION_COMPLETE]] (perf work done)
- This package: Vision foundation established
- Next action: Create canonical docs in `specs/canonical/`

### Start Here

1. Read this context package (you're doing it now)
2. Read the plan file: `C:\Users\dietl\.claude\plans\ethereal-inventing-river.md`
3. Create directory: `apps/tastematter/specs/canonical/`
4. Create `00_VISION.md` with:
   - YAML frontmatter (see plan)
   - Content from "The Breakthrough" and "Position in Context OS Stack" sections above
   - Wiki-links to source specs
5. Create `01_PRINCIPLES.md` with:
   - YAML frontmatter (see plan)
   - The 5 principles from this package
   - Each principle with source attribution
6. Create `02_ROADMAP.md` with:
   - YAML frontmatter (see plan)
   - The 6 phases from this package
   - Dependency diagram
7. Update `specs/context_packages/README.md` to reference canonical docs
8. Git commit

### Do NOT

- Don't re-read the source specs - they're synthesized in this package
- Don't change the 5 principles - they're derived from authoritative sources
- Don't reorder roadmap phases - dependencies are mapped
- Don't skip wiki-links - attribution is critical

### Key Insight

Tastematter is Level 2 of the Context Operating System architecture. It's not a visualization tool - it's the **human interface** for stigmergic coordination with agents via git. The 5-second latency isn't a bug - it's a violation of the core "Immediate" principle that breaks the coordination loop.

[VERIFIED: Synthesized from [[04_GIT_STIGMERGY_FOUNDATION.md]], [[05_PRODUCT_ARCHITECTURE.md]], [[06_INTELLIGENT_GITOPS_SPEC.md]]]

---

## Verification Commands

```bash
# Verify git state
cd apps/tastematter && git status && git log --oneline -5

# Verify test state
cd apps/tastematter && npm test
cd apps/tastematter/src-tauri && cargo test

# Verify source specs exist
ls "_system/specs/architecture/context_operating_system/"

# Verify knowledge base concepts exist
ls "04_knowledge_base/methodology/"
ls "04_knowledge_base/technical/"
```

---

**Package written:** 2026-01-07
**Session duration:** ~3 hours (cleanup + vision synthesis)
**Lines in package:** ~600 (mega package for vision foundation)
**Attribution quality:** ~70% VERIFIED, ~30% INFERRED, 0% UNVERIFIABLE
