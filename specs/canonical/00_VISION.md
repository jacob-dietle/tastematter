---
title: "Tastematter Vision"
type: vision
created: 2026-01-07
last_updated: 2026-01-07
status: active
foundation:
  - "[[_system/specs/architecture/context_operating_system/04_GIT_STIGMERGY_FOUNDATION.md]]"
  - "[[_system/specs/architecture/context_operating_system/05_PRODUCT_ARCHITECTURE.md]]"
  - "[[_system/specs/architecture/context_operating_system/06_INTELLIGENT_GITOPS_SPEC.md]]"
related:
  - "[[canonical/01_PRINCIPLES]]"
  - "[[canonical/02_ROADMAP]]"
  - "[[04_knowledge_base/methodology/stigmergy.md]]"
  - "[[04_knowledge_base/technical/context-worker-agent-pattern.md]]"
tags:
  - tastematter
  - vision
  - canonical
---

# Tastematter Vision

## Executive Summary

Tastematter is the **human interface to Context Operating Systems** - a Tauri desktop application that enables immediate visibility into attention patterns, seamless resumption of context, agent-augmented exploration, and stigmergic coordination with intelligent agents. It is Level 2 of the Context OS architecture stack.

[INFERRED: Synthesis of [[04_GIT_STIGMERGY_FOUNDATION.md]], [[05_PRODUCT_ARCHITECTURE.md]], [[06_INTELLIGENT_GITOPS_SPEC.md]]]

---

## Purpose Statement

> Tastematter enables humans to SEE the git state, RESPOND to agent modifications, and COORDINATE through the stigmergic substrate.

Agents and humans coordinate by reading and writing to git repos, not by direct messaging. The repo IS the communication medium. Tastematter makes this coordination visible, immediate, and actionable.

[VERIFIED: [[04_GIT_STIGMERGY_FOUNDATION.md]]:174 - "Git is coordination infrastructure"]

---

## Position in Context OS Stack

```
+-------------------------------------------------------------------+
|  LEVEL 3: Inter-OS Protocols (FUTURE)                             |
|           Context as a service, MCP publishing, pay-walling       |
|           [VERIFIED: [[06_INTELLIGENT_GITOPS_SPEC.md]]:86-96]     |
+-------------------------------------------------------------------+
|  LEVEL 2: Multi-Repo Desktop App  <-- TASTEMATTER IS HERE         |
|           Tauri app, multi-repo management, agent UI control      |
|           [VERIFIED: [[06_INTELLIGENT_GITOPS_SPEC.md]]:73-79]     |
+-------------------------------------------------------------------+
|  LEVEL 1: Team Coordination                                       |
|           GitHub webhooks, Cloudflare Workers, conflict detect    |
|           [VERIFIED: [[06_INTELLIGENT_GITOPS_SPEC.md]]:52-66]     |
+-------------------------------------------------------------------+
|  LEVEL 0: Personal CLI  <-- ALREADY BUILT (context-os)            |
|           CLI + daemon, promptable rules, intelligent commits     |
|           [VERIFIED: [[06_INTELLIGENT_GITOPS_SPEC.md]]:35-49]     |
+-------------------------------------------------------------------+
```

Tastematter is not a standalone tool - it's a layer in an integrated stack where each level builds on the one below.

---

## Core Value Propositions

### 1. Shape of Attention

See the shape of your work patterns over time. Visualize file access frequencies, modification patterns, and chain structures to understand where attention flows.

[VERIFIED: [[05_PRODUCT_ARCHITECTURE.md]]:23-35 - "Understanding where you spend attention"]

### 2. Resume Context Instantly

Context is expensive to rebuild. Tastematter shows recent activity, active chains, and modification history so you can resume work in seconds rather than minutes.

[VERIFIED: [[05_PRODUCT_ARCHITECTURE.md]]:40-52 - "Zero-friction context resumption"]

### 3. Compound Knowledge Over Time

Every session builds on the last. The stigmergic substrate (git) preserves work patterns, and Tastematter surfaces them when relevant.

[VERIFIED: [[05_PRODUCT_ARCHITECTURE.md]]:148 - "We help your knowledge compound. You keep the compounded value."]

### 4. Agent-Augmented Exploration

Agents can control the UI via guardrailed pre-built states, enabling 10x human control depth. Human directs, agent navigates, both coordinate through git.

[VERIFIED: [[context-worker-agent-pattern.md]]:404-449 - Guardrailed UI states]

---

## What Tastematter IS NOT

- **NOT a standalone visualization tool** - It's Level 2 of the Context OS stack
  [VERIFIED: [[06_INTELLIGENT_GITOPS_SPEC.md]]:73-79]

- **NOT just for viewing file access patterns** - It's the coordination interface for human-agent collaboration
  [INFERRED: Vision includes agent-controllable UI, git ops integration]

- **NOT a replacement for git** - Git is the coordination substrate; Tastematter makes it visible
  [VERIFIED: [[04_GIT_STIGMERGY_FOUNDATION.md]]:174]

- **NOT a note-taking app or knowledge management tool** - It's about attention and coordination, not content storage
  [INFERRED: From architectural position in stack]

---

## The Git-Stigmergy Foundation

Tastematter is built on a fundamental insight:

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

Agents and humans communicate by modifying the environment (git commits), not by sending messages. Tastematter surfaces these environmental modifications in real-time.

[VERIFIED: [[stigmergy.md]]:49-67]

---

## Success Vision

When Tastematter is complete:

1. **<100ms navigation** - Any view switch feels instant
   [Principle: IMMEDIATE from [[canonical/01_PRINCIPLES]]]

2. **Git state visible** - See what agents modified, when, and why
   [Principle: STIGMERGIC]

3. **Multi-repo fluency** - Switch between Personal → Team → Company repos seamlessly
   [Principle: MULTI-REPO AWARE]

4. **Agent-directed exploration** - Tell an agent what to find, it navigates the UI for you
   [Principle: AGENT-CONTROLLABLE]

5. **User owns everything** - All data in user's git repos, not vendor's cloud
   [Principle: INVESTMENT NOT RENT]

The experience should be:

> "Effortless (files just appear where they should), Surprising (how did it know to do that?), Trustworthy (but I understand why it did that)"
> [VERIFIED: [[05_PRODUCT_ARCHITECTURE.md]]:155-159]

---

## Related Documents

- [[canonical/01_PRINCIPLES]] - The 5 non-negotiable design principles
- [[canonical/02_ROADMAP]] - 6-phase development plan to achieve this vision
- [[context_packages/05_2026-01-07_VISION_FOUNDATION]] - Session context that produced these canonical docs
