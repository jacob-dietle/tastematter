---
title: "Tastematter Roadmap"
type: roadmap
created: 2026-01-07
last_updated: 2026-01-07
status: active
foundation:
  - "[[_system/specs/architecture/context_operating_system/06_INTELLIGENT_GITOPS_SPEC.md]]"
  - "[[specs/08_UNIFIED_DATA_ARCHITECTURE.md]]"
  - "[[specs/10_PERF_OPTIMIZATION_SPEC.md]]"
related:
  - "[[canonical/00_VISION]]"
  - "[[canonical/01_PRINCIPLES]]"
  - "[[context_packages/05_2026-01-07_VISION_FOUNDATION]]"
tags:
  - tastematter
  - roadmap
  - canonical
---

# Tastematter Roadmap

## Executive Summary

Six phases from current state to full vision, each addressing specific principle violations:

| Phase | Focus | Principle Addressed |
|-------|-------|---------------------|
| 0 | Performance Foundation | IMMEDIATE |
| 1 | Stigmergic Display | STIGMERGIC |
| 2 | Multi-Repo Dashboard | MULTI-REPO AWARE |
| 3 | Agent UI Control Protocol | AGENT-CONTROLLABLE |
| 4 | Intelligent GitOps Integration | All principles |
| 5 | MCP Publishing | INVESTMENT NOT RENT (externalization) |

---

## Current State Assessment

### What's Implemented

**Architecture Foundation (Solid):**
- Unified context store pattern with global state (timeRange, selectedChain, chains)
- Request deduplication pattern preventing stale UI updates
- Svelte 5 reactive state ($state, $effect)
- Thin Tauri wrapper delegating to context-os CLI

**Views (Partially Complete):**

| View | Status | Notes |
|------|--------|-------|
| Files View | 80% | Table + heatmap working, query_flex fully integrated |
| Timeline View | 40% | Structure exists, **per-day data is simulated** |
| Sessions View | 40% | Bulk fetch works, **metadata is synthesized** |
| Git Panel | 90% | Status, pull, push working |

**Stores:**
- `context.svelte.ts` - Global state (timeRange, selectedChain, chains)
- `files.svelte.ts` - File query with aggregations
- `timeline.svelte.ts` - Timeline buckets (simulated data)
- `workstream.svelte.ts` - Sessions by chain (lazy-loaded)
- `git.svelte.ts` - Git status and operations

**Tauri Commands:**
- `query_flex` - Hypercube slicing with aggregations
- `query_timeline` - Per-day buckets (**simulated**)
- `query_sessions` - Sessions by time/chain
- `query_chains` - List conversation chains
- `git_status`, `git_pull`, `git_push` - Git operations

### What's NOT Implemented

| Gap | Severity | Principle Violated |
|-----|----------|-------------------|
| Real per-day timeline data | P1 | STIGMERGIC |
| Real session metadata | P1 | STIGMERGIC |
| Multi-repo management | P1 | MULTI-REPO AWARE |
| Agent CLI control protocol | P0 | AGENT-CONTROLLABLE |
| Real-time updates (file watcher) | P1 | STIGMERGIC |
| <100ms view switches | P0 | IMMEDIATE |

[VERIFIED: Codebase exploration 2026-01-07]

---

## Phase 0: Performance Foundation

> **Principle violated:** IMMEDIATE (<100ms)
> **Current violation:** 5-second view switches

### Problem Statement

The current architecture calls Python CLI (`context-os`) synchronously from the Tauri Rust backend. Each query spawns a Python process, which is unacceptably slow.

### Solution Architecture

```
CURRENT (Slow):
Svelte → Tauri Command → spawn Python → JSON parse → return

PHASE 0 (Fast):
Svelte → Tauri Command → Rust SQLite query → return
                              ↑
                    Background indexer updates
                    (watches git + file events)
```

### Tasks

1. **Rust-native SQLite queries**
   - Reimplement `query_flex`, `query_timeline`, `query_sessions` in pure Rust
   - Query the same SQLite database the Python CLI uses
   - Eliminate Python process spawn overhead

2. **Background indexer daemon**
   - Watch for file system changes
   - Watch for git events (commits, pulls)
   - Pre-compute aggregations on change (not on demand)

3. **Pre-computed views cache**
   - Timeline buckets computed on ingest
   - Session metadata extracted from index
   - Common queries cached

4. **Progressive loading UI**
   - Show shape/structure immediately (<50ms)
   - Fill in details progressively
   - Never block on full data load

### Success Criteria

| Metric | Target |
|--------|--------|
| View switch (Files ↔ Timeline ↔ Sessions) | <100ms |
| Filter change (timeRange, chain) | <50ms |
| Initial load | <200ms |
| Search results | <200ms |

### Dependencies

None - this unblocks everything else.

### Technical Notes

From [[06_INTELLIGENT_GITOPS_SPEC.md]]:274-276:
> "Tauri (Rust backend + web UI, small binaries, cross-platform)"
> "Note: Agentic coding means you don't need to learn Rust - you spec it, agents build it."

---

## Phase 1: Stigmergic Display

> **Principle addressed:** STIGMERGIC
> **Gap:** No git commit visibility, no agent vs human differentiation

### Problem Statement

Currently Tastematter shows file access patterns but NOT what actually changed in git. Users can't see agent modifications or respond to them, breaking the stigmergic coordination loop.

### Solution Architecture

```
Git State Visibility:
+------------------+
| Git Panel        |  ← Current: status only
| + Commit History |  ← New: recent commits with diffs
| + Author Badge   |  ← New: agent vs human indicator
| + "What Changed" |  ← New: since last viewed
+------------------+
```

### Tasks

1. **Git commit timeline in sidebar**
   - Recent commits (configurable depth)
   - Commit message preview
   - Files changed per commit (expandable)

2. **Agent vs human commit badges**
   - Parse commit author: `Co-Authored-By: Claude` indicates agent
   - Visual differentiation (icon/color)
   - Filter by author type

3. **"What changed since I last looked?" view**
   - Track last-viewed timestamp per repo
   - Highlight new commits since then
   - Show diff summary

4. **Integration with file views**
   - Click file in commit → navigate to file in Files view
   - Click commit → filter views to files in that commit
   - Correlate git history with access patterns

### Success Criteria

- User can see WHAT agents modified (commits)
- User can see WHO modified (agent vs human)
- User can respond (approve, reject, modify)
- Git panel integrates with chain filtering

### Dependencies

Phase 0 (performance) should complete first, but can start concurrently.

---

## Phase 2: Multi-Repo Dashboard

> **Principle addressed:** MULTI-REPO AWARE
> **Gap:** App locked to single repo

### Problem Statement

Context OS operates at multiple scales (Personal → Team → Company). Currently Tastematter is locked to whichever repo the CLI was initialized in.

### Solution Architecture

```
Multi-Repo State:
+------------------------+
| context.svelte.ts      |
| + activeRepo: RepoId   |  ← New
| + repos: RepoConfig[]  |  ← New
+------------------------+
         ↓
+------------------------+
| All queries pass       |
| --repo <activeRepo>    |
+------------------------+
```

### Tasks

1. **Repo configuration storage**
   - `~/.context-os/repos.yaml` - list of registered repos
   - Each repo: path, name, layer (personal/team/company)
   - CLI command: `context-os repo add/remove/list`

2. **Repo selector UI**
   - Dropdown or sidebar panel
   - Shows repo name, layer, status indicator
   - Quick switch between repos

3. **Per-repo context isolation**
   - `activeRepo` in context store
   - All queries include repo filter
   - State (chains, files, timeline) scoped to active repo

4. **Cross-repo unified view (optional)**
   - Toggle between "single repo" and "all repos" mode
   - Unified timeline across repos
   - Repo badge on each entry

5. **Status indicators per repo**
   - Clean/dirty (uncommitted changes)
   - Behind remote (needs pull)
   - Ahead of remote (needs push)

### Success Criteria

- Switch between repos in <100ms
- Each repo maintains independent context
- Status visible for all registered repos
- Optional cross-repo unified view

### Dependencies

- Phase 0 (performance) must complete
- CLI must support `--repo` parameter

---

## Phase 3: Agent UI Control Protocol

> **Principle addressed:** AGENT-CONTROLLABLE
> **Gap:** No CLI protocol for UI navigation

### Problem Statement

Agents (Claude, Claude Code) can query data via `context-os query`, but cannot control what the human SEES in Tastematter. For 10x human leverage, agents need to navigate the UI to relevant information.

### Solution Architecture

```
Agent → CLI → IPC → Tastematter

CLI commands (new):
$ context-os ui state          # Query current UI state
$ context-os ui navigate ...   # Navigate to specific view/state
$ context-os ui highlight ...  # Highlight specific items

IPC Protocol:
Tauri listens on local port for UI commands
Commands are JSON-RPC style
```

### Tasks

1. **Define finite UI state space**
   - Views: files | timeline | sessions
   - Filters: timeRange, selectedChain
   - Selections: highlighted files, expanded sessions
   - Document all valid states

2. **CLI commands for UI control**
   ```bash
   # Query current state
   context-os ui state --format json

   # Navigate to view
   context-os ui navigate --view timeline --time 7d --chain <id>

   # Highlight items
   context-os ui highlight --files "*.md" --duration 5s

   # Expand/collapse
   context-os ui expand --chain <id>
   ```

3. **IPC listener in Tauri**
   - Local socket or named pipe
   - Accepts JSON-RPC commands
   - Translates to store mutations

4. **UI animation on remote command**
   - Smooth transitions when agent navigates
   - Visual indicator that agent is controlling UI
   - User can override at any time

5. **Guardrails documentation**
   - What agent CAN do (navigate, filter, highlight)
   - What agent CAN'T do (create UI, modify data, arbitrary DOM)
   - Error responses for invalid commands

### Example Workflow

```
Human: "Show me everything related to the Pixee chain from last week"

Agent executes:
  1. context-os query chains --name "pixee" → gets chain_id
  2. context-os ui navigate --view timeline --chain {chain_id} --time 7d
  3. context-os ui highlight --files "*pixee*" --duration 5s

Tastematter:
  - Animates to timeline view
  - Filters to Pixee chain
  - Highlights related files with pulsing glow

Human: Sees exactly what they asked for, can refine or explore further
```

### Success Criteria

- Agent can navigate to any valid UI state via CLI
- Human sees smooth animation on agent navigation
- Human can override agent at any time
- Invalid commands return clear errors

### Dependencies

- Phase 0 (performance) must complete
- Phase 1 (stigmergic) provides context for what to highlight

---

## Phase 4: Intelligent GitOps Integration

> **Principle addressed:** All principles (integration layer)
> **Implements:** Level 0 daemon from [[06_INTELLIGENT_GITOPS_SPEC.md]]

### Problem Statement

Tastematter shows state but doesn't SUGGEST actions. The Level 0 daemon should watch for changes and surface intelligent suggestions (commit this, pull that, review this agent change).

### Solution Architecture

```
Level 0 Daemon
+------------------------+
| File watcher           |
| Git event listener     |
| Promptable rules       |
+------------------------+
         ↓
+------------------------+
| Notification stream    |
+------------------------+
         ↓
+------------------------+
| Tastematter sidebar    |
| "Suggestions" panel    |
+------------------------+
```

### Tasks

1. **Level 0 daemon integration**
   - Daemon runs as background service
   - Watches for file changes
   - Applies promptable rules
   - Emits suggestions

2. **Notification stream to Tastematter**
   - Daemon → Tastematter IPC
   - Suggestions appear in sidebar
   - Types: commit, pull, review, warning

3. **One-click actions**
   - "Commit these changes" → stages + commits
   - "Pull latest" → git pull
   - "Review agent change" → shows diff
   - "Dismiss" → hides suggestion

4. **Promptable rules UI**
   - View current rules from `~/.context-os/rules.yaml`
   - Edit rules in Tastematter
   - Test rules against current state

### Example Rules

From [[06_INTELLIGENT_GITOPS_SPEC.md]]:249-259:
```yaml
rules:
  - "Commit knowledge_base/ changes within 1 hour of modification"
  - "Never auto-commit files in _system/state/ - always ask first"
  - "If I haven't committed in 3 days, send me a notification"
  - "Review any agent commit that modifies more than 10 files"
```

### Success Criteria

- Suggestions appear within 1s of triggering event
- One-click actions work reliably
- Rules are editable in UI
- Real-time updates (no manual refresh)

### Dependencies

- Phase 0, 1, 2, 3 should complete
- Daemon must be implemented (separate work)

---

## Phase 5: MCP Publishing

> **Principle addressed:** INVESTMENT NOT RENT (value externalization)
> **Implements:** Level 3 from [[06_INTELLIGENT_GITOPS_SPEC.md]]

### Problem Statement

User has built valuable context. How do they share it selectively, with authentication and optionally monetization?

### Solution Architecture

```
Context OS Repo
       ↓
MCP Server (generated)
       ↓
Auth Layer (optional)
       ↓
Pay-wall (optional)
       ↓
External Consumers
```

### Tasks

1. **MCP server generation**
   - Select which paths/repos to publish
   - Generate MCP server configuration
   - Expose query endpoints

2. **Authentication configuration**
   - API keys per consumer
   - Rate limiting
   - Access logs

3. **Pay-walling integration**
   - Stripe/payment integration
   - Usage-based billing
   - Free tier vs paid

4. **Management UI**
   - List published contexts
   - View access logs
   - Manage API keys
   - Revenue dashboard

### Success Criteria

- User can publish context as MCP server in <5 min
- Authentication prevents unauthorized access
- Pay-walling enables monetization
- All data remains in user's git repos

### Dependencies

- All previous phases
- MCP server tooling (may be external dependency)

---

## Dependencies Between Phases

```
Phase 0: Performance Foundation
    ↓ (unblocks all)
    ├─→ Phase 1: Stigmergic Display
    │       ↓
    ├─→ Phase 2: Multi-Repo Dashboard
    │       ↓
    └─→ Phase 3: Agent UI Control
            ↓
        Phase 4: Intelligent GitOps
            ↓
        Phase 5: MCP Publishing
```

**Critical path:** Phase 0 → Phase 1 → Phase 3 → Phase 4

Phase 2 (multi-repo) can proceed in parallel with Phase 1.

---

## Principle Alignment Table

| Phase | IMMEDIATE | STIGMERGIC | MULTI-REPO | AGENT-CTRL | INVESTMENT |
|-------|-----------|------------|------------|------------|------------|
| 0 | **PRIMARY** | - | - | - | - |
| 1 | - | **PRIMARY** | - | - | - |
| 2 | - | - | **PRIMARY** | - | - |
| 3 | - | - | - | **PRIMARY** | - |
| 4 | Yes | Yes | Yes | Yes | - |
| 5 | - | - | - | - | **PRIMARY** |

---

## Current CLI Commands Reference

Commands available in `context-os` CLI for integration:

| Command | Purpose | Status |
|---------|---------|--------|
| `query flex` | Hypercube slicing with aggregations | Working |
| `query co-access` | Find related files | Working |
| `query session` | All files in a session | Working |
| `query chains` | List conversation chains | Working |
| `query verify` | Verify a receipt | Working |
| `query search` | Keyword search in file paths | Working |
| `ui state` | Query current UI state | **Not implemented** |
| `ui navigate` | Navigate to view/state | **Not implemented** |
| `repo add/list` | Multi-repo management | **Not implemented** |

[VERIFIED: context-query skill documentation 2026-01-07]

---

## Related Documents

- [[canonical/00_VISION]] - What Tastematter IS
- [[canonical/01_PRINCIPLES]] - The 5 non-negotiable principles this roadmap addresses
- [[specs/08_UNIFIED_DATA_ARCHITECTURE.md]] - Current architecture details
- [[specs/10_PERF_OPTIMIZATION_SPEC.md]] - Phase 0 performance work
- [[context_packages/05_2026-01-07_VISION_FOUNDATION]] - Session that produced this roadmap
