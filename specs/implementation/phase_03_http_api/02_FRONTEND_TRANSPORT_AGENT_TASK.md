# Agent Task Spec: Frontend Transport Abstraction

**Phase:** 3.2 - Frontend Transport Abstraction
**Estimated Time:** 1-2 hours
**Agent Type:** Implementation Agent
**Prerequisite:** Phase 3.1 HTTP Server MUST be complete
**Skill Reference:** specification-driven-development

---

## Mission Statement

Create a transport abstraction layer in the frontend that allows seamless switching between Tauri IPC (production) and HTTP API (development). This enables the same frontend code to work in both Tauri and browser environments.

**You are NOT:**
- Modifying store logic (only import paths change)
- Changing component code (stores handle the abstraction)
- Adding new features (just abstraction layer)
- Breaking existing Tauri functionality

**You ARE:**
- Creating transport interface
- Implementing Tauri transport (extracting existing code)
- Implementing HTTP transport (new)
- Updating stores to use transport abstraction
- Configuring Vite proxy for development

---

## Read-First Checklist

Read these files IN ORDER before writing any code:

1. **[[04_TRANSPORT_ARCHITECTURE.md]]** - Architecture spec (Phase 2 section)
   - Focus: Transport interface definition, auto-detection logic
   - Location: `apps/tastematter/specs/canonical/04_TRANSPORT_ARCHITECTURE.md`

2. **[[tauri.ts]]** - Current Tauri API wrapper
   - Focus: Existing invoke patterns, type definitions
   - Location: `apps/tastematter/src/lib/api/tauri.ts`

3. **[[files.svelte.ts]]** - Example store using Tauri API
   - Focus: How queryFlex is called, what needs abstraction
   - Location: `apps/tastematter/src/lib/stores/files.svelte.ts`

4. **[[timeline.svelte.ts]]** - Another store example
   - Focus: queryTimeline usage pattern
   - Location: `apps/tastematter/src/lib/stores/timeline.svelte.ts`

5. **[[vite.config.ts]]** - Vite configuration
   - Focus: Where to add proxy configuration
   - Location: `apps/tastematter/vite.config.ts`

---

## Implementation Steps

### Step 1: Create Transport Interface (10 min)

**File:** `apps/tastematter/src/lib/api/transport.ts` (NEW)

```typescript
/**
 * Transport-agnostic API interface for context-os queries.
 *
 * Enables seamless switching between:
 * - Tauri IPC (production): Direct Rust calls via @tauri-apps/api
 * - HTTP API (development): REST calls to localhost:3001
 */

import type {
  QueryFlexInput,
  QueryResult,
  QueryTimelineInput,
  TimelineData,
  QuerySessionsInput,
  SessionQueryResult,
  QueryChainsInput,
  ChainQueryResult,
} from '../types';

/**
 * Transport interface - all query methods the frontend needs.
 */
export interface QueryTransport {
  queryFlex(input: QueryFlexInput): Promise<QueryResult>;
  queryTimeline(input: QueryTimelineInput): Promise<TimelineData>;
  querySessions(input: QuerySessionsInput): Promise<SessionQueryResult>;
  queryChains(input: QueryChainsInput): Promise<ChainQueryResult>;
}

/**
 * Detect if we're running inside Tauri.
 * window.__TAURI__ is injected by Tauri runtime.
 */
export function isTauri(): boolean {
  return typeof window !== 'undefined' && !!(window as any).__TAURI__;
}
```

### Step 2: Create Tauri Transport (15 min)

**File:** `apps/tastematter/src/lib/api/tauri-transport.ts` (NEW)

```typescript
/**
 * Tauri IPC transport implementation.
 * Used in production Tauri desktop app.
 */

import { invoke } from '@tauri-apps/api/core';
import type { QueryTransport } from './transport';
import type {
  QueryFlexInput,
  QueryResult,
  QueryTimelineInput,
  TimelineData,
  QuerySessionsInput,
  SessionQueryResult,
  QueryChainsInput,
  ChainQueryResult,
} from '../types';

export const tauriTransport: QueryTransport = {
  async queryFlex(input: QueryFlexInput): Promise<QueryResult> {
    return invoke('query_flex', {
      files: input.files,
      time: input.time,
      chain: input.chain,
      session: input.session,
      agg: input.agg,
      limit: input.limit,
      sort: input.sort,
    });
  },

  async queryTimeline(input: QueryTimelineInput): Promise<TimelineData> {
    return invoke('query_timeline', {
      time: input.time,
      files: input.files,
      chain: input.chain,
      limit: input.limit,
    });
  },

  async querySessions(input: QuerySessionsInput): Promise<SessionQueryResult> {
    return invoke('query_sessions', {
      time: input.time,
      chain: input.chain,
      limit: input.limit,
    });
  },

  async queryChains(input: QueryChainsInput): Promise<ChainQueryResult> {
    return invoke('query_chains', {
      limit: input.limit,
    });
  },
};
```

### Step 3: Create HTTP Transport (15 min)

**File:** `apps/tastematter/src/lib/api/http-transport.ts` (NEW)

```typescript
/**
 * HTTP API transport implementation.
 * Used in browser development mode.
 * Requires: context-os serve --port 3001 --cors
 */

import type { QueryTransport } from './transport';
import type {
  QueryFlexInput,
  QueryResult,
  QueryTimelineInput,
  TimelineData,
  QuerySessionsInput,
  SessionQueryResult,
  QueryChainsInput,
  ChainQueryResult,
} from '../types';

/**
 * Base URL for HTTP API.
 * In dev mode, Vite proxy forwards /api/* to localhost:3001
 */
const API_BASE = '/api';

async function post<T>(endpoint: string, body: unknown): Promise<T> {
  const response = await fetch(`${API_BASE}${endpoint}`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(body),
  });

  if (!response.ok) {
    const error = await response.json().catch(() => ({ message: response.statusText }));
    throw new Error(error.message || `HTTP ${response.status}`);
  }

  return response.json();
}

export const httpTransport: QueryTransport = {
  async queryFlex(input: QueryFlexInput): Promise<QueryResult> {
    return post('/query/flex', input);
  },

  async queryTimeline(input: QueryTimelineInput): Promise<TimelineData> {
    return post('/query/timeline', input);
  },

  async querySessions(input: QuerySessionsInput): Promise<SessionQueryResult> {
    return post('/query/sessions', input);
  },

  async queryChains(input: QueryChainsInput): Promise<ChainQueryResult> {
    return post('/query/chains', input);
  },
};
```

### Step 4: Create Transport Auto-Selector (10 min)

**File:** `apps/tastematter/src/lib/api/index.ts` (NEW or UPDATE)

```typescript
/**
 * Auto-selecting transport based on environment.
 *
 * Usage in stores:
 *   import { transport } from '$lib/api';
 *   const result = await transport.queryFlex({ time: '7d' });
 *
 * In Tauri: Uses IPC (0ms overhead)
 * In Browser: Uses HTTP (needs context-os serve running)
 */

import { isTauri, type QueryTransport } from './transport';
import { tauriTransport } from './tauri-transport';
import { httpTransport } from './http-transport';

// Export the transport interface for type-only imports
export type { QueryTransport } from './transport';
export { isTauri } from './transport';

// Auto-select transport based on environment
export const transport: QueryTransport = isTauri()
  ? tauriTransport
  : httpTransport;

// Export individual methods for convenience
export const queryFlex = transport.queryFlex.bind(transport);
export const queryTimeline = transport.queryTimeline.bind(transport);
export const querySessions = transport.querySessions.bind(transport);
export const queryChains = transport.queryChains.bind(transport);

// Log which transport is being used (dev only)
if (import.meta.env.DEV) {
  console.log(`[transport] Using ${isTauri() ? 'Tauri IPC' : 'HTTP API'}`);
}
```

### Step 5: Update Vite Config for Proxy (5 min)

**File:** `apps/tastematter/vite.config.ts` (MODIFY)

Add proxy configuration inside `defineConfig`:

```typescript
export default defineConfig({
  // ... existing config ...

  server: {
    // ... existing server config ...

    proxy: {
      '/api': {
        target: 'http://localhost:3001',
        changeOrigin: true,
      },
    },
  },
});
```

### Step 6: Update Stores to Use Transport (30 min)

Update each store to use the transport abstraction instead of direct Tauri imports.

**File:** `apps/tastematter/src/lib/stores/files.svelte.ts`

BEFORE:
```typescript
import { queryFlex } from '$lib/api/tauri';
// or
import { invoke } from '@tauri-apps/api/core';
```

AFTER:
```typescript
import { queryFlex } from '$lib/api';
```

**Stores to update:**
1. `files.svelte.ts` - Uses queryFlex
2. `timeline.svelte.ts` - Uses queryTimeline
3. `workstream.svelte.ts` - Uses querySessions
4. `context.svelte.ts` - Uses queryChains

**Pattern for each:**
1. Change import from `'$lib/api/tauri'` to `'$lib/api'`
2. No other changes needed (function signatures are identical)

### Step 7: Test in Browser Mode (15 min)

**Terminal 1: Start HTTP server**
```bash
cd apps/context-os/core
cargo run --bin context-os -- serve --port 3001 --cors
```

**Terminal 2: Start Vite dev server**
```bash
cd apps/tastematter
npm run dev
```

**Browser: Open http://localhost:5173**
- Check console for `[transport] Using HTTP API`
- Verify all views load data
- Check Network tab for /api/* requests

### Step 8: Test in Tauri Mode (10 min)

```bash
cd apps/tastematter
npm run tauri dev
```

- Check console for `[transport] Using Tauri IPC`
- Verify all views load data
- No /api/* requests in Network tab

---

## Type Contracts

### No New Types

This phase uses existing types from `$lib/types/index.ts`:
- `QueryFlexInput`, `QueryResult`
- `QueryTimelineInput`, `TimelineData`
- `QuerySessionsInput`, `SessionQueryResult`
- `QueryChainsInput`, `ChainQueryResult`

### Interface Contract

```typescript
interface QueryTransport {
  queryFlex(input: QueryFlexInput): Promise<QueryResult>;
  queryTimeline(input: QueryTimelineInput): Promise<TimelineData>;
  querySessions(input: QuerySessionsInput): Promise<SessionQueryResult>;
  queryChains(input: QueryChainsInput): Promise<ChainQueryResult>;
}
```

Both `tauriTransport` and `httpTransport` MUST implement this interface exactly.

---

## Success Criteria

**MUST pass before marking complete:**

- [ ] All new files compile without TypeScript errors
- [ ] Browser mode works: `npm run dev` shows data from HTTP API
- [ ] Tauri mode works: `npm run tauri dev` shows data from IPC
- [ ] Console shows correct transport selection
- [ ] All 4 views (Files, Timeline, Sessions, Chains) work in both modes
- [ ] No changes to store logic beyond import paths
- [ ] No changes to component code

**Browser mode checklist:**
- [ ] Files view loads
- [ ] Timeline view loads
- [ ] Sessions/Workstream view loads
- [ ] Chain selector populates
- [ ] Switching chains filters data

---

## Common Pitfalls

### Pitfall 1: Vite Proxy Not Working

**Symptom:** 404 errors for /api/* requests
**Fix:** Ensure vite.config.ts has proxy configured AND restart Vite

### Pitfall 2: CORS Errors

**Symptom:** Browser shows CORS error
**Fix:** Ensure `context-os serve --cors` flag is set

### Pitfall 3: HTTP Server Not Running

**Symptom:** `ECONNREFUSED` or `Failed to fetch`
**Fix:** Start HTTP server first: `context-os serve --port 3001 --cors`

### Pitfall 4: Type Mismatch

**Symptom:** TypeScript errors on transport calls
**Fix:** Ensure input types match exactly (check optional vs required fields)

### Pitfall 5: Wrong Import Path

**Symptom:** `Module not found: $lib/api`
**Fix:** Create `$lib/api/index.ts` with all exports

---

## Files Created/Modified Summary

| File | Action | Lines |
|------|--------|-------|
| `src/lib/api/transport.ts` | CREATE | ~30 |
| `src/lib/api/tauri-transport.ts` | CREATE | ~50 |
| `src/lib/api/http-transport.ts` | CREATE | ~50 |
| `src/lib/api/index.ts` | CREATE | ~25 |
| `vite.config.ts` | MODIFY | +5 |
| `src/lib/stores/files.svelte.ts` | MODIFY | ~2 |
| `src/lib/stores/timeline.svelte.ts` | MODIFY | ~2 |
| `src/lib/stores/workstream.svelte.ts` | MODIFY | ~2 |
| `src/lib/stores/context.svelte.ts` | MODIFY | ~2 |

**Total new code:** ~155 lines
**Total modified:** ~15 lines (mostly import changes)

---

## Completion Report Template

After completing, write to `PHASE_3_2_COMPLETION_REPORT.md`:

```markdown
# Phase 3.2 Completion Report

**Status:** ✅ COMPLETE | ⚠️ INCOMPLETE

## What Was Implemented
- [ ] Transport interface
- [ ] Tauri transport
- [ ] HTTP transport
- [ ] Auto-selector
- [ ] Vite proxy config
- [ ] Store updates (4 files)

## Test Results
- Browser mode: ✅ All views working
- Tauri mode: ✅ All views working
- Transport detection: ✅ Correct in both modes

## Dev Workflow Verified
1. Start HTTP server: `context-os serve --port 3001 --cors`
2. Start Vite: `npm run dev`
3. Open browser: http://localhost:5173
4. All data loads via HTTP API

## Known Issues
[List any issues discovered]

## Next Steps
- Phase 3.3: Fix hardcoded limits
- Phase 3.4: Add Playwright E2E tests
```

---

**Spec Version:** 1.0
**Last Updated:** 2026-01-09
