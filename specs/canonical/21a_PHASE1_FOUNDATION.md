# Phase 1: Foundation

**Parent:** [[21_STIGMERGIC_CONTROL_PLANE_V2]]
**Blocks:** [[21b_PHASE2_ALERTING]], [[21c_PHASE3_STATUS_IMPLEMENTATION]], [[21d_PHASE4_DASHBOARD]]
**Estimated:** ~350 lines, 20-25 tests

---

## Mission

Extend the existing control plane worker with system grouping, /status polling with fallback, and push webhook support. This is the schema and API foundation everything else builds on.

## Prerequisites

- Control plane deployed at `control.tastematter.dev` with D1 `tastematter-control`
- 20/20 existing tests passing
- CF Access service token working for cross-worker auth

## Files to Create/Modify

### New Files

**`migrations/002_control_plane_v2.sql`**

```sql
-- System registry
CREATE TABLE IF NOT EXISTS system_registry (
  id TEXT PRIMARY KEY,
  display_name TEXT NOT NULL,
  description TEXT,
  health_rule TEXT DEFAULT 'all',
  current_status TEXT DEFAULT 'unknown',
  status_changed_at TEXT,
  created_at TEXT DEFAULT (datetime('now'))
);

-- Extend worker_registry
ALTER TABLE worker_registry ADD COLUMN system_id TEXT REFERENCES system_registry(id);
ALTER TABLE worker_registry ADD COLUMN account_id TEXT;
ALTER TABLE worker_registry ADD COLUMN status_url TEXT;

-- Sync log
CREATE TABLE IF NOT EXISTS sync_log (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  worker_id TEXT NOT NULL,
  synced_at TEXT DEFAULT (datetime('now')),
  commit_sha TEXT NOT NULL,
  file_count INTEGER,
  source_repo TEXT,
  action_run_url TEXT,
  success INTEGER DEFAULT 1,
  error_message TEXT
);

CREATE INDEX IF NOT EXISTS idx_sync_log_worker ON sync_log(worker_id, synced_at DESC);

-- Seed systems
INSERT OR IGNORE INTO system_registry (id, display_name, description, health_rule) VALUES
  ('intel-pipeline', 'Intelligence Pipeline', 'Ingestion + generation sharing 05_transcripts/', 'all'),
  ('tastematter-platform', 'Tastematter Platform', 'Alerting + publishing + monitoring', 'all'),
  ('client-deployments', 'Client Deployments', 'Multi-account client workers', 'any');

-- Assign existing workers to systems
UPDATE worker_registry SET system_id = 'intel-pipeline' WHERE id = 'transcript-processing';
UPDATE worker_registry SET system_id = 'intel-pipeline' WHERE id = 'intelligence-pipeline';
UPDATE worker_registry SET system_id = 'tastematter-platform' WHERE id = 'tastematter-alert-worker';
```

### Modified Files

**`src/types.ts`** — Add these types:

```typescript
// /status contract — what workers return
export interface WorkerStatusResponse {
  identity: {
    worker: string;
    display_name: string;
    system_id?: string;
    account_id?: string;
    version?: string;
  };
  vitals: {
    status: 'ok' | 'degraded' | 'error';
    started_at?: string;
    features?: Record<string, boolean>;
  };
  corpus?: {
    commit: string;
    file_count: number;
    loaded_at: string;
    source_repo?: string;
  };
  trail?: {
    last_deposit: string;
    at: string;
    type: string;
    detail?: string;
  };
  d1_health?: {
    total_executions: number;
    total_failures: number;
    failure_rate: string;
    last_execution?: { status: string; duration_ms: number; at: string };
    last_failure?: { error: string; at: string };
  };
  schedule?: {
    cron: string;
    last_run?: string;
    next_run?: string;
  };
}

// System types
export type SystemStatus = 'healthy' | 'degraded' | 'broken' | 'unknown';

export interface SystemRegistryRow {
  id: string;
  display_name: string;
  description: string | null;
  health_rule: string;
  current_status: SystemStatus;
  status_changed_at: string | null;
  created_at: string;
}

export interface SystemWithMembers extends SystemRegistryRow {
  members: WorkerWithStatus[];
}

// Sync types
export interface SyncLogRow {
  id: number;
  worker_id: string;
  synced_at: string;
  commit_sha: string;
  file_count: number | null;
  source_repo: string | null;
  action_run_url: string | null;
  success: number;
  error_message: string | null;
}

export interface SyncWebhookPayload {
  worker_id: string;
  commit_sha: string;
  file_count?: number;
  source_repo?: string;
  action_run_url?: string;
  success?: boolean;
  error_message?: string;
}

// Extend WorkerRegistryRow (add to existing)
// system_id: string | null;
// account_id: string | null;
// status_url: string | null;

// Extend WorkerWithStatus (add to existing)
// system_id: string | null;
// account_id: string | null;
// latest_sync: SyncLogRow | null;
```

**`src/db.ts`** — Add these operations:

```typescript
// System CRUD
async getSystems(): Promise<SystemRegistryRow[]>
async getSystem(id: string): Promise<SystemRegistryRow | null>
async registerSystem(system: { id: string; display_name: string; description?: string; health_rule?: string }): Promise<void>
async deleteSystem(id: string): Promise<void>
async updateSystemStatus(id: string, status: SystemStatus): Promise<void>

// Sync CRUD
async logSync(entry: SyncWebhookPayload): Promise<void>
async getLatestSync(workerId: string): Promise<SyncLogRow | null>
async getSyncHistory(workerId: string, limit?: number): Promise<SyncLogRow[]>

// Extended worker queries
async getSystemsWithMembers(): Promise<SystemWithMembers[]>
async updateWorker(id: string, fields: Partial<{ system_id: string; enabled: number; account_id: string; status_url: string }>): Promise<void>

// Extend getWorkersWithStatus to include system_id, account_id, latest sync
```

**`src/health-checker.ts`** — Modify `checkWorkerHealth`:

```typescript
// New: try /status first, fallback to /health
export async function checkWorkerHealth(
  worker: WorkerRegistryRow,
  env: Env,
): Promise<HealthCheckResult> {
  const statusUrl = worker.status_url ?? worker.health_url.replace('/health', '/status');

  // Try /status first
  const statusResult = await tryFetch(statusUrl, worker, env);
  if (statusResult.http_status === 200) {
    const parsed = parseStatusResponse(statusResult.raw_response);
    if (parsed) {
      return buildResultFromStatus(worker.id, statusResult, parsed);
    }
  }

  // Fallback to /health
  if (statusResult.http_status === 404 || !statusResult.raw_response) {
    return tryFetchHealth(worker, env);
  }

  return statusResult;
}

// Map old /health response shape into HealthCheckResult
function parseStatusResponse(raw: string | null): WorkerStatusResponse | null {
  // Parse JSON, validate has identity.worker and vitals.status
  // Return null if doesn't match /status contract (triggers /health fallback)
}
```

**`src/index.ts`** — Add routes:

```typescript
// GET /systems — list all systems with holistic health
// POST /systems — register a system
// DELETE /systems/:id — deregister a system
// PATCH /workers/:id — update worker fields
// POST /sync — push webhook from GitHub Actions
// GET /sync/:worker_id — sync history for a worker
// POST /workers/:id/reload — proxy reload to worker
// POST /workers/:id/trigger — proxy trigger to worker
```

Cron handler update:
```typescript
// After polling all workers:
// 1. Get all systems
// 2. For each system, compute health from member statuses
// 3. Compare with current_status
// 4. If changed, update system_registry and log transition
// 5. Phase 2 will add: trigger Knock on transition
```

## Implementation Steps

### Step 1: Migration
1. Write `002_control_plane_v2.sql`
2. Apply: `wrangler d1 migrations apply tastematter-control --remote`
3. Verify: query tables exist, seed data present

### Step 2: Types
1. Add all new types to `src/types.ts`
2. Extend existing `WorkerRegistryRow` and `WorkerWithStatus`

### Step 3: Database Operations
1. Add system CRUD to `src/db.ts`
2. Add sync CRUD to `src/db.ts`
3. Extend `getWorkersWithStatus` to include system_id, account_id
4. Add `getSystemsWithMembers` (JOIN worker_registry with health_log and system_registry)
5. Add `updateWorker` for PATCH support

### Step 4: /status Polling Upgrade
1. Modify `checkWorkerHealth` in `src/health-checker.ts`
2. Try `/status` → parse → if fails, fallback to `/health`
3. Store richer data from /status response (corpus info, trail, d1_health go into raw_response)
4. Keep existing `evaluateStaleness` and `shouldAlert` logic unchanged

### Step 5: New Routes
1. System CRUD routes in `src/index.ts`
2. PATCH /workers/:id route
3. POST /sync webhook endpoint
4. GET /sync/:worker_id endpoint
5. Proxy routes (POST /workers/:id/reload, /workers/:id/trigger)

### Step 6: Cron Update
1. After health polling loop, compute system health
2. Compare with stored status, detect transitions
3. Update `system_registry.current_status` and `status_changed_at`
4. Log transitions to console (Phase 2 adds Knock)

## Tests

### Unit Tests (health-checker)
- `tryStatusFirst_success` — /status returns 200, parsed correctly
- `tryStatusFirst_404_fallback` — /status returns 404, falls back to /health
- `tryStatusFirst_malformed_fallback` — /status returns invalid JSON, falls back
- `parseStatusResponse_valid` — valid /status JSON parsed correctly
- `parseStatusResponse_minimal` — only identity + vitals, no optional fields
- `parseStatusResponse_invalid` — returns null for non-conforming JSON

### Unit Tests (system health)
- `computeSystemHealth_allHealthy` — all members healthy → system healthy
- `computeSystemHealth_oneDown` — one member down → system broken (rule: all)
- `computeSystemHealth_oneDown_anyRule` — one member down, one healthy → system healthy (rule: any)
- `computeSystemHealth_allDown` — all members down → system broken
- `computeSystemHealth_noMembers` — empty → unknown
- `computeSystemHealth_stale` — stale members → degraded

### Integration Tests (routes)
- `POST /systems` — creates system, returns 201
- `GET /systems` — returns systems with member workers
- `DELETE /systems/:id` — removes system
- `PATCH /workers/:id` — updates system_id, verified in GET /workers
- `POST /sync` — logs sync, appears in GET /sync/:worker_id
- `POST /sync` — rejects missing commit_sha with 400
- `POST /workers/:id/reload` — proxies to worker (mock worker response)
- `POST /workers/:id/trigger` — proxies to worker (mock worker response)

### Cron Tests
- `cron_computesSystemHealth` — after polling, system statuses updated in D1
- `cron_detectsTransition` — system status change logged

## Success Criteria

- [ ] All existing 20 tests still pass
- [ ] 20-25 new tests pass
- [ ] `GET /systems` returns 3 seeded systems with member workers
- [ ] `POST /sync` stores webhook data in sync_log
- [ ] Polling tries /status first, falls back to /health gracefully
- [ ] System health computed correctly in cron
- [ ] State transitions detected and logged

## Pitfalls

1. **D1 ALTER TABLE limitations** — D1 supports ADD COLUMN but not all ALTER operations. Test migration on remote before assuming it works.
2. **FOREIGN KEY on system_id** — Workers without a system_id should have NULL, not empty string. D1 enforces FK constraints.
3. **Proxy routes need auth** — When proxying /reload or /trigger to a worker, include the correct CF Access headers based on worker's auth_type.
4. **Don't break existing /workers response** — Dashboard currently expects `WorkerWithStatus` shape. New fields must be additive, not breaking.

## Integration with Other Phases

- **Phase 2** reads: system_registry.current_status, status_changed_at. Fires Knock when transition detected.
- **Phase 3** implements: /status endpoint on alert-worker. Once deployed, control plane will get richer data from its /status poll.
- **Phase 4** reads: all new endpoints (GET /systems, GET /sync/:id). Renders system cards and controls.
