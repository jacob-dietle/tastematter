# Spec 21: Stigmergic Control Plane v2

**Date:** 2026-02-20
**Status:** Architecture Approved
**Depends on:** [[17_CONTEXT_ALERTING_AND_PUBLISHING]] (alert worker, control plane v1)
**Phases:** [[21a_PHASE1_FOUNDATION]], [[21b_PHASE2_ALERTING]], [[21c_PHASE3_STATUS_IMPLEMENTATION]], [[21d_PHASE4_DASHBOARD]]

---

## Problem

~20 Cloudflare Workers across 3 accounts with zero unified visibility or control. ATP was down for 6 days before anyone noticed. Corpus syncs can fail silently. Worker coordination (ingestion → generation) has no observability.

**Success metric:** See the state of all workers, systems, and data pipelines in under 60 seconds. Get alerted when system health changes.

## Architecture: Two Primitives

Everything in this system reduces to two load-bearing mechanisms:

**PUSH** — After corpus sync, GitHub Action POSTs to control plane: "I synced worker X to commit Y at time T."

**PULL** — Control plane polls `/status` on each worker hourly, gets identity, vitals, corpus state, trail info, D1 health.

**Intelligence = delta(push, pull)** — Compare what SHOULD be true (push) with what IS true (pull). The gap is the signal.

## System Model

```
GitHub Actions ──POST /sync──► Control Plane ──GET /status──► Workers
                (push)          │  D1:                         (pull)
                                │  worker_registry              │
                                │  system_registry              │
                                │  health_log                   └── /status response
                                │  sync_log
                                │
                                ├── Cron: hourly poll
                                ├── Knock: on system state transitions
                                │
                    Dashboard ◄──┘ (SSR +page.server.ts)
                    app.tastematter.dev
```

## Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Registration | Manual API, CLI-wrappable | Explicit control now, `tastematter worker register` later |
| /status contract | Enforced v1, /health fallback | Consistency with robustness |
| Dashboard | Full control surface | Maximize value per deployment |
| Alerting | Knock on system transitions | Solve ATP-down-6-days immediately |
| Multi-account | Service token per account stored in registry | CF Access headers per-worker |

## D1 Schema (migration 002)

### New: `system_registry`

```sql
CREATE TABLE system_registry (
  id TEXT PRIMARY KEY,
  display_name TEXT NOT NULL,
  description TEXT,
  health_rule TEXT DEFAULT 'all',   -- 'all' | 'any'
  current_status TEXT DEFAULT 'unknown',
  status_changed_at TEXT,
  created_at TEXT DEFAULT (datetime('now'))
);
```

### Extend: `worker_registry`

```sql
ALTER TABLE worker_registry ADD COLUMN system_id TEXT REFERENCES system_registry(id);
ALTER TABLE worker_registry ADD COLUMN account_id TEXT;
ALTER TABLE worker_registry ADD COLUMN status_url TEXT;
```

### New: `sync_log`

```sql
CREATE TABLE sync_log (
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

CREATE INDEX idx_sync_log_worker ON sync_log(worker_id, synced_at DESC);
```

## /status Contract (v1)

```typescript
// Every worker returns this from GET /status
interface WorkerStatusResponse {
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
```

**Fallback:** If `/status` returns 404 or unparseable response, fall back to `GET /health` and map:
```typescript
function mapHealthToStatus(health: any): WorkerStatusResponse {
  return {
    identity: { worker: health.worker ?? 'unknown', display_name: health.worker ?? 'Unknown' },
    vitals: { status: health.status === 'ok' ? 'ok' : 'error' },
  };
}
```

## Control Plane API Surface

### Existing (unchanged)
```
GET  /health                → control plane's own health
GET  /workers/:id/health    → health history for one worker
POST /workers/:id/check     → force immediate health check
```

### Modified
```
GET  /workers               → now includes system_id, account_id, sync status
POST /workers               → accepts system_id, account_id, status_url
```

### New
```
GET    /systems                  → all systems with holistic health
POST   /systems                  → register a system
DELETE /systems/:id              → deregister a system
PATCH  /workers/:id              → update worker fields (system_id, enabled, etc.)
DELETE /workers/:id              → deregister a worker (already exists)
POST   /sync                     → push webhook from GitHub Actions
GET    /sync/:worker_id          → sync history for a worker
POST   /workers/:id/reload       → proxy POST /reload to worker
POST   /workers/:id/trigger      → proxy POST /alert/trigger to worker
```

## System Health Computation

```typescript
type SystemStatus = 'healthy' | 'degraded' | 'broken' | 'unknown';

function computeSystemHealth(rule: string, memberStatuses: WorkerStatus[]): SystemStatus {
  if (memberStatuses.length === 0) return 'unknown';
  const healthy = (s: WorkerStatus) => s === 'healthy' || s === 'reachable';
  const down = (s: WorkerStatus) => s === 'down' || s === 'timeout';

  if (rule === 'all') {
    if (memberStatuses.every(healthy)) return 'healthy';
    if (memberStatuses.some(down)) return 'broken';
    return 'degraded';
  }
  // 'any'
  if (memberStatuses.some(healthy)) return 'healthy';
  return 'broken';
}
```

State transitions (healthy → broken, broken → healthy) trigger Knock.

## Phase Breakdown

| Phase | Name | Scope | Lines | Tests | Spec |
|-------|------|-------|-------|-------|------|
| 1 | Foundation | Schema, /status polling, push webhook, system APIs | ~350 | 20-25 | [[21a_PHASE1_FOUNDATION]] |
| 2 | Alerting | System health → Knock notifications | ~100 | 8-10 | [[21b_PHASE2_ALERTING]] |
| 3 | /status Implementation | Add /status to alert-worker + template | ~80 | 5-8 | [[21c_PHASE3_STATUS_IMPLEMENTATION]] |
| 4 | Dashboard | SSR data loading, system cards, controls | ~400 | 0 (manual) | [[21d_PHASE4_DASHBOARD]] |

**Total:** ~930 lines, 33-43 tests

**Dependency graph:**
```
Phase 1 (Foundation) → Phase 2 (Alerting)
Phase 1 (Foundation) → Phase 3 (/status on workers)
Phase 1 + 2 + 3     → Phase 4 (Dashboard)
```

Phase 2 and 3 can run in parallel after Phase 1.

## Key Files

### Control Plane (`apps/tastematter/control-plane/`)
| File | Current | Phase 1 Changes |
|------|---------|-----------------|
| `migrations/002_control_plane_v2.sql` | NEW | Schema changes |
| `src/types.ts` | 62 lines | Add system types, /status types, sync types |
| `src/db.ts` | 97 lines | Add system CRUD, sync CRUD, extended queries |
| `src/health-checker.ts` | 115 lines | /status polling with fallback |
| `src/index.ts` | 151 lines | New routes, system health in cron |

### Alert Worker (`apps/tastematter/alert-worker/`)
| File | Phase 3 Changes |
|------|-----------------|
| `src/index.ts` | Add `GET /status` route |

### Web App (`apps/tastematter/web-app/`)
| File | Phase 4 Changes |
|------|-----------------|
| `src/routes/+page.server.ts` | Expand data loading |
| `src/routes/+page.svelte` | System cards, controls |

## Visualization

See `_system/reports/control_plane_architecture.html` for the full architecture diagram.

## Stigmergic Foundation

This architecture is designed so the control plane's D1 data is itself a pheromone trail. Future meta-workers can read `health_log`, `sync_log`, and `system_registry` to make coordination decisions without direct messaging — stigmergy at the infrastructure level.
