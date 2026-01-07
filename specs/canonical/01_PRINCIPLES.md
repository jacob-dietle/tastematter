---
title: "Tastematter Design Principles"
type: principles
created: 2026-01-07
last_updated: 2026-01-07
status: active
foundation:
  - "[[_system/specs/architecture/context_operating_system/04_GIT_STIGMERGY_FOUNDATION.md]]"
  - "[[_system/specs/architecture/context_operating_system/05_PRODUCT_ARCHITECTURE.md]]"
  - "[[_system/specs/architecture/context_operating_system/06_INTELLIGENT_GITOPS_SPEC.md]]"
  - "[[04_knowledge_base/methodology/evidence-attribution-system.md]]"
related:
  - "[[canonical/00_VISION]]"
  - "[[canonical/02_ROADMAP]]"
  - "[[04_knowledge_base/methodology/stigmergy.md]]"
  - "[[04_knowledge_base/technical/context-worker-agent-pattern.md]]"
tags:
  - tastematter
  - principles
  - canonical
---

# Tastematter Design Principles

## Executive Summary

Five non-negotiable principles guide all Tastematter development:

1. **IMMEDIATE** - <100ms for any navigation
2. **STIGMERGIC** - Shows git state, enables coordination through environment
3. **MULTI-REPO AWARE** - Personal → Team → Company layers
4. **AGENT-CONTROLLABLE** - Pre-built guardrailed UI states agents can invoke
5. **INVESTMENT NOT RENT** - User owns all data in their git repos

Every feature decision, architecture choice, and implementation detail must align with these principles. Violations are bugs.

---

## Principle 1: IMMEDIATE

> Any navigation must complete in <100ms.

### Source

- [[05_PRODUCT_ARCHITECTURE.md]]:154-159 - "Feels like fucking magic"
- Bret Victor's "Inventing on Principle" - the idea that creators need an immediate connection to what they create

### The Expert POV: Bret Victor

Bret Victor's core insight: **Latency destroys creative flow.** When there's delay between action and feedback, you lose the ability to explore, iterate, and think through the medium. Tastematter must feel like an extension of thought, not a tool that requires waiting.

> "The creative process is fundamentally iterative. If you can't see the result of a change immediately, you can't think by making."

This is why <100ms is non-negotiable - not for vanity metrics, but because latency structurally breaks how humans coordinate with agents through stigmergic feedback.

### Requirement

- View switches: <100ms
- Filter changes: <50ms
- Search results: <200ms
- Any user action must have visible response within 100ms

### Why This Matters

Latency breaks the stigmergic feedback loop:

```
Agent commits change
      |
      v
Human needs to SEE that change immediately  <-- LATENCY HERE BREAKS LOOP
      |
      v
Human responds (accepts, modifies, overrides)
      |
      v
Agent sees human's response
      |
      v
Coordination emerges
```

If the human can't see changes immediately, they can't respond, and the coordination loop fails.

[VERIFIED: [[05_PRODUCT_ARCHITECTURE.md]]:155-159]

### Violation Example

5-second view switches break the feedback loop. User loses context while waiting, can't maintain flow state, coordination becomes frustrating rather than fluid.

---

## Principle 2: STIGMERGIC

> The app shows git state and enables coordination through environment modification.

### Source

- [[04_GIT_STIGMERGY_FOUNDATION.md]] - Git as coordination infrastructure
- [[04_knowledge_base/methodology/stigmergy.md]] - Coordination theory

### Requirement

- Show git commits in timeline (what changed)
- Differentiate agent vs human commits (who changed it)
- Show modification timestamps (when)
- Enable human response to agent modifications (approve, reject, modify)

### Why This Matters

> "Agents read shared state files (pipeline.yaml, directory structure). Agents modify environment (create files, update state). Other agents respond to those modifications. Coordination emerges without direct agent-to-agent communication."

[VERIFIED: [[stigmergy.md]]:49-67]

The repo IS the communication medium. Tastematter must surface this communication visually.

### From Foundation Spec

> "Git is your coordination layer - Don't build custom sync, use git. Agents read and write state - They don't send messages to each other. Coordination emerges - You don't orchestrate, you set up the environment."

[VERIFIED: [[04_GIT_STIGMERGY_FOUNDATION.md]]:309-313]

### Violation Example

An app that only shows file access patterns without git state is not stigmergic. The user can't see WHAT agents did or respond appropriately.

---

## Principle 3: MULTI-REPO AWARE

> Manage Personal → Team → Company context layers from single interface.

### Source

- [[06_INTELLIGENT_GITOPS_SPEC.md]]:73-79 - Level 2 requirements
- [[05_PRODUCT_ARCHITECTURE.md]]:259-291 - Layered Context OS Vision

### Requirement

- Repo selector to switch between context OS repos
- Unified timeline across repos (filtered or combined)
- Cross-repo search capability
- Status indicators per repo (clean/dirty/behind remote)

### Why This Matters

Context OS operates at multiple scales:

```
Personal Context OS (your repo)
        |
        | push/pull
        v
Team Context OS (shared repo)
        |
        | push/pull
        v
Company Context OS (org repo)
```

[VERIFIED: [[04_GIT_STIGMERGY_FOUNDATION.md]]:253-268]

> "Key principle: Each layer has its own stigmergic coordination. Propagation between layers is selective, not automatic."

[VERIFIED: [[05_PRODUCT_ARCHITECTURE.md]]:291]

### Violation Example

An app locked to single repo forces users to quit and relaunch for different contexts. This fragments attention and breaks coordination.

---

## Principle 4: AGENT-CONTROLLABLE

> Agents can invoke pre-built guardrailed UI states via CLI.

### Source

- User vision + [[04_knowledge_base/technical/context-worker-agent-pattern.md]]
- [[06_INTELLIGENT_GITOPS_SPEC.md]]:138-142 - CLI commands

### Requirement

- Define finite set of UI states (views + filters + selections)
- Create CLI command: `context-os ui navigate --view X --filters Y`
- Tastematter listens and animates to requested state
- Agent can query current UI state: `context-os ui state`
- Document guardrails (what agent CAN'T do)

### Why This Matters

10x human control via agent augmentation:

```
Human: "Show me everything related to the Pixee chain from last week"

Agent executes:
  1. context-os query chains --name "pixee" → gets chain_id
  2. context-os ui navigate --view timeline --chain {chain_id} --time 7d
  3. context-os ui highlight --related-files

Tastematter: Animates to timeline view, highlights Pixee chain, shows connections
```

### Guardrails Are Critical

From [[context-worker-agent-pattern.md]]:404-449:
- Agent can't create arbitrary UI
- Agent can only invoke pre-defined view states/transitions
- This keeps UI elegant and prevents agent-generated chaos

### Violation Example

An agent that can manipulate DOM directly or create arbitrary UI elements violates the guardrail principle. The result is chaotic, inconsistent, and untrustworthy UX.

---

## Principle 5: INVESTMENT NOT RENT

> User owns all data in their git repos. Value compounds for user, not vendor.

### Source

- [[05_PRODUCT_ARCHITECTURE.md]]:125-148

### Requirement

- All data stored in user's git repos
- No vendor cloud storage required for core functionality
- Switching cost = learning curve only, not data migration
- User captures all appreciation of their knowledge work

### Comparison

| Rent Extraction | Investment |
|-----------------|------------|
| Value locked in vendor's system | Value locked in user's system |
| Switching cost = data migration | Switching cost = learning curve only |
| Vendor captures appreciation | User captures appreciation |
| User is customer | User is owner |

[VERIFIED: [[05_PRODUCT_ARCHITECTURE.md]]:139-147]

### The Pitch

> "We help your knowledge compound. You keep the compounded value."

[VERIFIED: [[05_PRODUCT_ARCHITECTURE.md]]:148]

### Violation Example

Storing session data, preferences, or indexes in a cloud service (even for convenience) begins the rent extraction pattern. The moment user data lives in vendor infrastructure, switching costs emerge.

---

## How Principles Interact

```
                    IMMEDIATE
                        |
                        | Enables real-time
                        v
    STIGMERGIC <-----> AGENT-CONTROLLABLE
        |                     |
        | Git-based           | Pre-built states
        | coordination        | via CLI
        v                     v
    MULTI-REPO  <---------> INVESTMENT
    AWARE               NOT RENT
        |                     |
        | Multiple            | User owns
        | layers              | all repos
        v                     v
    [Context OS operates at scale with user ownership]
```

**IMMEDIATE** enables the stigmergic feedback loop to function.
**STIGMERGIC** provides the coordination substrate (git).
**MULTI-REPO AWARE** extends stigmergy across organizational boundaries.
**AGENT-CONTROLLABLE** amplifies human coordination through agent augmentation.
**INVESTMENT NOT RENT** ensures user captures all compound value.

---

## Principle Violations Are Bugs

| Principle | Violation | Severity |
|-----------|-----------|----------|
| IMMEDIATE | >100ms view switch | P0 - Breaks coordination loop |
| STIGMERGIC | No git state visibility | P0 - No coordination possible |
| MULTI-REPO | Single repo only | P1 - Limits scale |
| AGENT-CONTROLLABLE | No CLI protocol | P1 - No augmentation |
| INVESTMENT NOT RENT | Vendor cloud storage | P0 - Breaks user ownership |

When in doubt, check: Does this decision align with all 5 principles? If not, it's a bug.

---

## Related Documents

- [[canonical/00_VISION]] - What Tastematter IS and why
- [[canonical/02_ROADMAP]] - How we achieve these principles phase by phase
- [[context_packages/05_2026-01-07_VISION_FOUNDATION]] - Session context that derived these principles
