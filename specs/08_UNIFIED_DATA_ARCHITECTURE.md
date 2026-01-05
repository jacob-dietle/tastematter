# Spec 08: Unified Data Architecture

**Feature:** Holistic data model and shared context for Tastematter views
**Status:** SPECIFICATION
**Created:** 2026-01-04
**Based On:** CLI Hypercube Spec (12_CLI_HYPERCUBE_SPEC.md)

---

## Executive Summary

Refactor Tastematter from 4 independent stores to a **unified architecture** based on the hypercube model. All views (Files, Timeline, Workstreams) are projections of the same underlying data, sharing global filters (timeRange, selectedChain).

**Problem:** Current architecture has 4 disconnected stores with duplicated state, inconsistent filtering, and no cross-view awareness.

**Solution:** Shared `ContextProvider` that owns global state + view-specific stores that subscribe to it.

**Core Insight:** From the Hypercube Spec:
```
Files × Sessions × Time × Chains × AccessType

Every "view" = slice + aggregate + render
```

---

## Problem Statement

### Current State (Broken)

```
┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐
│ queryStore  │  │timelineStore│  │sessionStore │  │ chainStore  │
├─────────────┤  ├─────────────┤  ├─────────────┤  ├─────────────┤
│ timeRange   │  │ timeRange   │  │ timeRange   │  │             │
│ data        │  │ data        │  │ selectedChain│ │ selectedChain│
│ loading     │  │ hoveredCell │  │ expandedSessions│ │ data     │
└──────┬──────┘  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘
       │                │                │                │
       ▼                ▼                ▼                ▼
   queryFlex      queryTimeline    querySessions    queryChains
   (CLI)          (CLI)            (CLI)            (CLI)
```

**Problems:**

| Issue | Impact |
|-------|--------|
| `timeRange` duplicated 3x | Changing time in one view doesn't update others |
| `selectedChain` duplicated 2x | Chain filter state is inconsistent |
| No data relationship | Can't answer "which chain touched this file?" |
| Independent fetches | Switching views = full refetch, slow UX |
| Mental model mismatch | Views feel like separate tools, not lenses on same data |

### User Decisions (Validated)

1. **Chain sidebar visible in ALL views** - Chain navigation is always available
2. **Chain selection FILTERS all views** - Selecting a chain filters Files, Timeline, AND Workstreams
3. **Time range is GLOBAL** - One time range for entire app

---

## Architecture

### The Hypercube Model

All data is a 5-dimensional hypercube:

```
┌─────────────────────────────────────────────────────────────────┐
│                    CONTEXT HYPERCUBE                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Dimension 1: FILES                                             │
│  └── All file paths ever touched                                │
│                                                                 │
│  Dimension 2: SESSIONS                                          │
│  └── All Claude Code sessions (UUIDs)                           │
│                                                                 │
│  Dimension 3: TIME                                              │
│  └── Temporal axis (days, weeks, ranges)                        │
│                                                                 │
│  Dimension 4: CHAINS                                            │
│  └── Conversation chains (work streams)                         │
│                                                                 │
│  Dimension 5: ACCESS_TYPE                                       │
│  └── read | write | create                                      │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Views as Projections

Every view is just a different projection of this hypercube:

| View | Slice Dimensions | Primary Axis | Aggregations |
|------|------------------|--------------|--------------|
| **Files** | time, chain | FILES | count, recency, sessions |
| **Timeline** | time, chain | TIME × FILES | daily buckets |
| **Workstreams** | time | CHAINS → SESSIONS → FILES | hierarchy |

### Target Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     ContextProvider                              │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │ Global State (single source of truth)                       ││
│  │                                                             ││
│  │ • timeRange: '7d' | '14d' | '30d'                          ││
│  │ • selectedChain: string | null                              ││
│  │ • chains: ChainData[]  (always loaded, for navigation)     ││
│  │ • chainsLoading: boolean                                    ││
│  │ • chainsError: CommandError | null                          ││
│  └─────────────────────────────────────────────────────────────┘│
│                              │                                   │
│              ┌───────────────┼───────────────┐                  │
│              ▼               ▼               ▼                  │
│  ┌───────────────┐ ┌───────────────┐ ┌───────────────────────┐ │
│  │  FilesStore   │ │ TimelineStore │ │   WorkstreamStore     │ │
│  │               │ │               │ │                       │ │
│  │ Reads:        │ │ Reads:        │ │ Reads:                │ │
│  │ • timeRange   │ │ • timeRange   │ │ • timeRange           │ │
│  │ • chain       │ │ • chain       │ │ • chains (from ctx)   │ │
│  │               │ │               │ │                       │ │
│  │ Own state:    │ │ Own state:    │ │ Own state:            │ │
│  │ • data        │ │ • data        │ │ • sessionsByChain     │ │
│  │ • sort        │ │ • hoveredCell │ │ • expandedChains      │ │
│  │ • granularity │ │               │ │ • expandedSessions    │ │
│  │ • loading     │ │ • loading     │ │ • loading             │ │
│  └───────────────┘ └───────────────┘ └───────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

### Data Flow

```
User changes time range (in header)
         │
         ▼
    ContextProvider.setTimeRange('30d')
         │
         ├──► FilesStore.onContextChange() → refetch with new range
         ├──► TimelineStore.onContextChange() → refetch with new range
         └──► WorkstreamStore.onContextChange() → refetch with new range

User clicks chain (in ChainNav sidebar)
         │
         ▼
    ContextProvider.selectChain('abc123')
         │
         ├──► FilesStore.onContextChange() → refetch filtered by chain
         ├──► TimelineStore.onContextChange() → refetch filtered by chain
         └──► WorkstreamStore.onContextChange() → highlight/expand chain
```

---

## Type Contracts

### ContextProvider (Global State)

```typescript
// stores/context.svelte.ts

interface ContextState {
  // Shared filters (hypercube slice dimensions)
  timeRange: '7d' | '14d' | '30d';
  selectedChain: string | null;

  // Chains always loaded (for sidebar navigation)
  chains: ChainData[];
  chainsLoading: boolean;
  chainsError: CommandError | null;
}

interface ContextActions {
  // Filter actions (trigger refetch in all view stores)
  setTimeRange(range: '7d' | '14d' | '30d'): Promise<void>;
  selectChain(chainId: string | null): void;
  clearChain(): void;

  // Data actions
  refreshChains(): Promise<void>;
}

type ContextStore = ContextState & ContextActions;
```

### FilesStore (View-Specific)

```typescript
// stores/files.svelte.ts

interface FilesState {
  // Data (fetched based on context filters)
  data: FileResult[] | null;
  loading: boolean;
  error: CommandError | null;

  // UI state (local to this view)
  sort: 'count' | 'recency' | 'alpha';
  granularity: 'file' | 'directory';
}

interface FilesActions {
  fetch(): Promise<void>;  // Uses context.timeRange, context.selectedChain
  setSort(sort: 'count' | 'recency' | 'alpha'): void;
  setGranularity(granularity: 'file' | 'directory'): void;
}

// Factory that receives context
function createFilesStore(ctx: ContextStore): FilesState & FilesActions;
```

### TimelineStore (View-Specific)

```typescript
// stores/timeline.svelte.ts

interface TimelineState {
  // Data
  data: TimelineData | null;
  loading: boolean;
  error: CommandError | null;

  // UI state
  hoveredCell: { file: string; date: string } | null;
}

interface TimelineActions {
  fetch(): Promise<void>;  // Uses context.timeRange, context.selectedChain
  setHoveredCell(file: string, date: string): void;
  clearHover(): void;
}

function createTimelineStore(ctx: ContextStore): TimelineState & TimelineActions;
```

### WorkstreamStore (View-Specific)

```typescript
// stores/workstream.svelte.ts

interface WorkstreamState {
  // Data (chains come from context, sessions fetched per-chain)
  sessionsByChain: Map<string, SessionData[]>;
  sessionsLoading: Set<string>;  // Which chains are loading
  sessionsError: Map<string, CommandError>;

  // UI state
  expandedChains: Set<string>;
  expandedSessions: Set<string>;
}

interface WorkstreamActions {
  // Lazy-load sessions when chain is expanded
  toggleChainExpanded(chainId: string): Promise<void>;
  toggleSessionExpanded(sessionId: string): void;
  expandAll(): void;
  collapseAll(): void;
}

function createWorkstreamStore(ctx: ContextStore): WorkstreamState & WorkstreamActions;
```

---

## Component Hierarchy

```
App.svelte
├── ContextProvider (creates context store, provides via Svelte context)
│   │
│   ├── Header
│   │   ├── Logo/Title
│   │   ├── ViewToggle [Files | Timeline | Workstreams]
│   │   └── TimeRangeToggle (writes to ctx.setTimeRange)
│   │
│   ├── Layout (grid: content + sidebar)
│   │   │
│   │   ├── MainContent
│   │   │   ├── FilesView (when activeView === 'files')
│   │   │   │   └── uses createFilesStore(ctx)
│   │   │   │
│   │   │   ├── TimelineView (when activeView === 'timeline')
│   │   │   │   └── uses createTimelineStore(ctx)
│   │   │   │
│   │   │   └── WorkstreamView (when activeView === 'workstreams')
│   │   │       └── uses createWorkstreamStore(ctx)
│   │   │
│   │   └── Sidebar
│   │       ├── GitPanel
│   │       └── ChainNav (reads ctx.chains, writes ctx.selectChain)
│   │           └── Always visible, regardless of active view
```

---

## CLI Integration

### Tauri Commands Used

| Command | Used By | Filters |
|---------|---------|---------|
| `query_chains` | ContextProvider | limit |
| `query_flex` | FilesStore | --time, --chain, --agg count,recency |
| `query_timeline` | TimelineStore | --time (chain filter TODO) |
| `query_sessions` | WorkstreamStore | --time, --chain |

### CLI Enhancement Needed

The `query_timeline` Rust command currently doesn't support `--chain` filter. Two options:

1. **Option A (Quick):** Filter client-side after fetch
2. **Option B (Proper):** Add `--chain` to CLI `query_timeline` command

For MVP, use Option A. Can enhance CLI later.

---

## Implementation Plan

### Phase 1: Create ContextProvider (~100 lines)

```typescript
// stores/context.svelte.ts

import { getContext, setContext } from 'svelte';
import { queryChains } from '$lib/api/tauri';

const CONTEXT_KEY = Symbol('app-context');

export function createContextStore() {
  // State
  let timeRange = $state<'7d' | '14d' | '30d'>('7d');
  let selectedChain = $state<string | null>(null);
  let chains = $state<ChainData[]>([]);
  let chainsLoading = $state(false);
  let chainsError = $state<CommandError | null>(null);

  // Actions
  async function refreshChains() {
    chainsLoading = true;
    chainsError = null;
    try {
      const result = await queryChains({ limit: 50 });
      chains = result.chains;
    } catch (e) {
      chainsError = e as CommandError;
    } finally {
      chainsLoading = false;
    }
  }

  async function setTimeRange(range: '7d' | '14d' | '30d') {
    timeRange = range;
    // View stores will react to this change
  }

  function selectChain(chainId: string | null) {
    selectedChain = chainId;
    // View stores will react to this change
  }

  function clearChain() {
    selectedChain = null;
  }

  return {
    // State getters
    get timeRange() { return timeRange; },
    get selectedChain() { return selectedChain; },
    get chains() { return chains; },
    get chainsLoading() { return chainsLoading; },
    get chainsError() { return chainsError; },

    // Actions
    setTimeRange,
    selectChain,
    clearChain,
    refreshChains,
  };
}

export type ContextStore = ReturnType<typeof createContextStore>;

export function setAppContext(ctx: ContextStore) {
  setContext(CONTEXT_KEY, ctx);
}

export function getAppContext(): ContextStore {
  return getContext(CONTEXT_KEY);
}
```

### Phase 2: Refactor FilesStore (~80 lines)

```typescript
// stores/files.svelte.ts

import type { ContextStore } from './context.svelte';
import { queryFlex } from '$lib/api/tauri';

export function createFilesStore(ctx: ContextStore) {
  // State
  let data = $state<FileResult[] | null>(null);
  let loading = $state(false);
  let error = $state<CommandError | null>(null);
  let sort = $state<'count' | 'recency' | 'alpha'>('count');
  let granularity = $state<'file' | 'directory'>('file');

  // Fetch using context filters
  async function fetch() {
    loading = true;
    error = null;
    try {
      const result = await queryFlex({
        time: ctx.timeRange,
        chain: ctx.selectedChain ?? undefined,
        agg: ['count', 'recency', 'sessions'],
        limit: 50,
        sort: sort,
      });
      data = result.results;
    } catch (e) {
      error = e as CommandError;
    } finally {
      loading = false;
    }
  }

  // React to context changes
  $effect(() => {
    // Access ctx.timeRange and ctx.selectedChain to create dependency
    const _ = ctx.timeRange;
    const __ = ctx.selectedChain;
    fetch();
  });

  // ... rest of store
}
```

### Phase 3: Refactor TimelineStore (~60 lines)

Similar pattern - subscribe to context, refetch on change.

### Phase 4: Create WorkstreamStore (~150 lines)

```typescript
// stores/workstream.svelte.ts

export function createWorkstreamStore(ctx: ContextStore) {
  // Chains come from context
  // Sessions are lazy-loaded per chain

  let sessionsByChain = $state<Map<string, SessionData[]>>(new Map());
  let sessionsLoading = $state<Set<string>>(new Set());
  let expandedChains = $state<Set<string>>(new Set());
  let expandedSessions = $state<Set<string>>(new Set());

  async function toggleChainExpanded(chainId: string) {
    const newExpanded = new Set(expandedChains);

    if (newExpanded.has(chainId)) {
      newExpanded.delete(chainId);
    } else {
      newExpanded.add(chainId);

      // Lazy load sessions if not already loaded
      if (!sessionsByChain.has(chainId)) {
        await loadSessionsForChain(chainId);
      }
    }

    expandedChains = newExpanded;
  }

  async function loadSessionsForChain(chainId: string) {
    sessionsLoading = new Set([...sessionsLoading, chainId]);
    try {
      const result = await querySessions({
        time: ctx.timeRange,
        chain: chainId,
        limit: 50,
      });
      sessionsByChain = new Map([...sessionsByChain, [chainId, result.sessions]]);
    } finally {
      const newLoading = new Set(sessionsLoading);
      newLoading.delete(chainId);
      sessionsLoading = newLoading;
    }
  }

  // ... rest of store
}
```

### Phase 5: Update App.svelte (~50 lines changed)

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { createContextStore, setAppContext } from '$lib/stores/context.svelte';
  // ... other imports

  // Create and provide context
  const ctx = createContextStore();
  setAppContext(ctx);

  let activeView = $state<'files' | 'timeline' | 'workstreams'>('files');

  onMount(() => {
    ctx.refreshChains();
  });
</script>

<main>
  <header>
    <h1>Tastematter</h1>
    <div class="header-controls">
      <ViewToggle bind:selected={activeView} />
      <TimeRangeToggle
        selected={ctx.timeRange}
        onchange={(range) => ctx.setTimeRange(range)}
      />
    </div>
  </header>

  <div class="layout">
    <section class="content">
      {#if activeView === 'workstreams'}
        <WorkstreamView />
      {:else if activeView === 'timeline'}
        <TimelineView />
      {:else}
        <FilesView />
      {/if}
    </section>

    <aside class="sidebar">
      <GitPanel />
      <ChainNav
        chains={ctx.chains}
        loading={ctx.chainsLoading}
        selected={ctx.selectedChain}
        onSelect={(id) => ctx.selectChain(id)}
      />
    </aside>
  </div>
</main>
```

---

## Migration Strategy

### Step 1: Create new stores alongside old (no breaking changes)
- `context.svelte.ts` (new)
- `files.svelte.ts` (new, replaces query.svelte.ts)
- `workstream.svelte.ts` (new, replaces session.svelte.ts + chain.svelte.ts)

### Step 2: Update views one at a time
- FilesView → use new FilesStore
- TimelineView → use new TimelineStore (refactored)
- Create WorkstreamView (replaces SessionView + ChainNavigator integration)

### Step 3: Delete old stores
- Remove query.svelte.ts
- Remove session.svelte.ts
- Remove chain.svelte.ts

### Step 4: Update App.svelte
- Use ContextProvider
- Wire up ChainNav to all views

---

## Success Criteria

### Functional
- [ ] Changing time range updates ALL views
- [ ] Selecting a chain filters ALL views to that chain's data
- [ ] ChainNav is visible and functional in all views
- [ ] Workstreams view shows chains → sessions → files hierarchy
- [ ] Expanding a chain lazy-loads its sessions

### Performance
- [ ] Chain list loads on app start (single fetch)
- [ ] View switches don't refetch if filters unchanged
- [ ] Sessions are lazy-loaded (not all at once)

### UX
- [ ] Clear visual indication of selected chain
- [ ] Clear visual indication of active time range
- [ ] Smooth transitions between views
- [ ] Loading states for all async operations

---

## Files to Create/Modify

| File | Action | Est. Lines |
|------|--------|------------|
| `stores/context.svelte.ts` | Create | ~100 |
| `stores/files.svelte.ts` | Create (replaces query) | ~80 |
| `stores/timeline.svelte.ts` | Refactor | ~80 |
| `stores/workstream.svelte.ts` | Create (replaces session+chain) | ~150 |
| `components/WorkstreamView.svelte` | Create | ~200 |
| `components/ChainCard.svelte` | Create | ~100 |
| `components/ChainNav.svelte` | Refactor from ChainNavigator | ~80 |
| `App.svelte` | Refactor | ~50 changed |
| `types/index.ts` | Add new types | ~30 |

**Total new/changed:** ~870 lines

---

## Related Specifications

- **Spec 07:** Chain Integration (current, being superseded)
- **CLI Hypercube Spec:** Data model foundation
- **Phase 5 Session View:** Original session implementation

---

**Last Updated:** 2026-01-04
**Status:** SPECIFICATION COMPLETE - Ready for Review
