# Spec 07: Chain Integration for Tastematter

**Date:** 2026-01-04
**Status:** In Progress
**Purpose:** Enable chain navigation in Tastematter UI

---

## Problem Statement

Currently `query_sessions` returns `chain_id: None` for all sessions because the CLI subprocess call doesn't include chain data per session. The CLI has `query chains` which returns proper chain data (session_count, files, time_range), but this isn't exposed via Tauri.

**User need:** See which sessions belong to which conversation chain to navigate related work.

---

## Architecture (Minimal)

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│ ChainNavigator  │────▶│ chainStore       │────▶│ query_chains    │
│ (Svelte)        │     │ (Svelte 5 runes) │     │ (Rust Tauri)    │
└─────────────────┘     └──────────────────┘     └─────────────────┘
                                                         │
                                                         ▼
                                                 ┌─────────────────┐
                                                 │ context-os CLI  │
                                                 │ query chains    │
                                                 │ --format json   │
                                                 └─────────────────┘
```

**Components:**
1. Rust: `query_chains` command (mirrors `query_sessions` pattern)
2. TypeScript: `ChainData` type + `queryChains()` API function
3. Svelte: `ChainNavigator` component (list of chains with selection)
4. Existing: `ChainBadge` already works (just needs chain data)

---

## Type Contracts

### Rust (commands.rs)

```rust
#[derive(Serialize)]
pub struct ChainData {
    pub chain_id: String,
    pub session_count: u32,
    pub file_count: u32,
    pub time_range: Option<(String, String)>,  // (start, end) ISO dates
    pub sessions: Vec<String>,                  // Session IDs in chain
}

#[derive(Serialize)]
pub struct ChainQueryResult {
    pub chains: Vec<ChainData>,
    pub total_chains: u32,
}
```

### TypeScript (types/index.ts)

```typescript
export interface ChainData {
  chain_id: string;
  session_count: number;
  file_count: number;
  time_range: [string, string] | null;  // [start, end] ISO dates
  sessions: string[];                    // Session IDs
}

export interface ChainQueryResult {
  chains: ChainData[];
  total_chains: number;
}

export interface ChainQueryArgs {
  limit?: number;  // Default: 20
}
```

---

## Implementation Steps

### Step 1: Rust Backend (commands.rs)

Add `query_chains` command that calls `context-os query chains --format json`:

```rust
#[command]
pub async fn query_chains(
    limit: Option<u32>,
) -> Result<ChainQueryResult, CommandError> {
    let cli_path = std::env::var("CONTEXT_OS_CLI")
        .unwrap_or_else(|_| "C:/Users/dietl/.context-os/bin/context-os.cmd".to_string());

    let mut cmd = Command::new(&cli_path);
    cmd.current_dir("../../..");
    cmd.args(["query", "chains", "--format", "json"]);
    cmd.args(["--limit", &limit.unwrap_or(20).to_string()]);

    // Execute and parse...
}
```

Register in `lib.rs`:
```rust
.invoke_handler(tauri::generate_handler![
    // existing...
    query_chains,
])
```

### Step 2: TypeScript Types (types/index.ts)

Add types after existing `SessionState`:

```typescript
// Chain types
export interface ChainData {
  chain_id: string;
  session_count: number;
  file_count: number;
  time_range: [string, string] | null;
  sessions: string[];
}

export interface ChainQueryResult {
  chains: ChainData[];
  total_chains: number;
}

export interface ChainQueryArgs {
  limit?: number;
}

export interface ChainState {
  loading: boolean;
  data: ChainQueryResult | null;
  error: CommandError | null;
  selectedChain: string | null;
}
```

### Step 3: API Function (api/tauri.ts)

```typescript
export async function queryChains(args: ChainQueryArgs): Promise<ChainQueryResult> {
  try {
    return await invoke<ChainQueryResult>('query_chains', {
      limit: args.limit,
    });
  } catch (error) {
    if (typeof error === 'string') {
      throw { code: 'CHAIN_ERROR', message: error } as CommandError;
    }
    throw error;
  }
}
```

### Step 4: Chain Store (stores/chain.svelte.ts)

```typescript
export function createChainStore() {
  let loading = $state(false);
  let data = $state<ChainQueryResult | null>(null);
  let error = $state<CommandError | null>(null);
  let selectedChain = $state<string | null>(null);

  async function fetch(limit?: number) {
    loading = true;
    error = null;
    try {
      data = await queryChains({ limit: limit ?? 20 });
    } catch (e) {
      error = e as CommandError;
    } finally {
      loading = false;
    }
  }

  function selectChain(chainId: string | null) {
    selectedChain = chainId;
  }

  return {
    get loading() { return loading; },
    get data() { return data; },
    get error() { return error; },
    get selectedChain() { return selectedChain; },
    get chains() { return data?.chains ?? []; },
    fetch,
    selectChain,
  };
}
```

### Step 5: ChainNavigator Component

```svelte
<!-- src/lib/components/ChainNavigator.svelte -->
<script lang="ts">
  import { onMount } from 'svelte';
  import { createChainStore } from '$lib/stores/chain.svelte';
  import ChainBadge from './ChainBadge.svelte';
  import LoadingSpinner from './LoadingSpinner.svelte';

  let { onSelect }: { onSelect?: (chainId: string | null) => void } = $props();

  const chainStore = createChainStore();

  onMount(() => {
    chainStore.fetch();
  });

  function handleClick(chainId: string) {
    const newChain = chainStore.selectedChain === chainId ? null : chainId;
    chainStore.selectChain(newChain);
    onSelect?.(newChain);
  }
</script>

<div class="chain-navigator">
  <h4>Chains</h4>
  {#if chainStore.loading}
    <LoadingSpinner />
  {:else if chainStore.chains.length > 0}
    <div class="chain-list">
      {#each chainStore.chains as chain}
        <button
          class="chain-item"
          class:selected={chainStore.selectedChain === chain.chain_id}
          onclick={() => handleClick(chain.chain_id)}
        >
          <ChainBadge chainId={chain.chain_id} />
          <span class="stats">{chain.session_count} sessions</span>
        </button>
      {/each}
    </div>
  {:else}
    <p class="empty">No chains found</p>
  {/if}
</div>
```

---

## Success Criteria

- [ ] `query_chains` Tauri command works and returns chain data
- [ ] ChainNavigator shows chains sorted by session_count (largest first)
- [ ] Clicking a chain filters SessionView to show only that chain's sessions
- [ ] ChainBadge displays correctly with color-coded chain IDs
- [ ] All existing tests still pass

---

## Test Plan

### Unit Tests (Vitest)
1. `queryChains` returns expected structure
2. `createChainStore` handles loading/error states
3. ChainNavigator renders chains correctly

### Integration Tests
1. End-to-end: Click chain → SessionView filters → correct sessions shown
2. Chain selection persists across time range changes

---

## CLI Output Reference (Verified)

```bash
$ context-os query chains --format json --limit 2
{
  "command": "chains",
  "timestamp": "2026-01-04T20:17:30.284861",
  "result_count": 2,
  "results": [
    {
      "chain_id": "7f389600",
      "session_count": 79,
      "file_count": 669,
      "time_range": {
        "start": "2025-12-11T04:31:05.507000+00:00",
        "end": "2026-01-05T01:03:16.750000+00:00"
      }
    },
    {
      "chain_id": "fa6b4bf6",
      "session_count": 21,
      "file_count": 193,
      "time_range": {
        "start": "2025-12-07T23:32:17.397000+00:00",
        "end": "2025-12-19T23:01:24.564000+00:00"
      }
    }
  ],
  "total_chains": 622
}
```

**Note:** `time_range` is an object with `start` and `end` ISO datetime strings.
