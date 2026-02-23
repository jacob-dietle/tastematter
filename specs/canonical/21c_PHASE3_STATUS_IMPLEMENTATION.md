# Phase 3: /status Implementation on Workers

**Parent:** [[21_STIGMERGIC_CONTROL_PLANE_V2]]
**Depends on:** [[21a_PHASE1_FOUNDATION]] (defines the /status contract)
**Parallel with:** [[21b_PHASE2_ALERTING]]
**Estimated:** ~80 lines per worker, 5-8 tests

---

## Mission

Add the `/status` endpoint to the alert-worker as reference implementation. Create a copy-paste template for all other workers. Once deployed, the control plane's /status polling will get richer data.

## Prerequisites

- Phase 1 complete: control plane polls /status with /health fallback
- /status contract defined in [[21_STIGMERGIC_CONTROL_PLANE_V2]]

## Reference Implementation: Alert Worker

### File: `apps/tastematter/alert-worker/src/index.ts`

Add a `GET /status` route **before** the existing `GET /health` route:

```typescript
// GET /status — rich status for control plane polling
if (url.pathname === "/status" && request.method === "GET") {
  const db = createDB(env.ALERTS_DB);

  // Corpus info
  let corpus: WorkerStatusResponse['corpus'] = undefined;
  if (env.CONTEXT_DO) {
    try {
      const doId = env.CONTEXT_DO.idFromName('singleton');
      const stub = env.CONTEXT_DO.get(doId);
      const health = await stub.fetch(new Request('http://internal/health'));
      const body = await health.json() as any;
      corpus = {
        commit: body.commit ?? 'unknown',
        file_count: body.fileCount ?? 0,
        loaded_at: body.loadedAt ?? new Date().toISOString(),
        source_repo: 'gtm_operating_system',
      };
    } catch { /* corpus not available */ }
  }

  // Trail: last alert fired
  const lastAlert = await db.getAlertHistory(undefined, 1);
  let trail: WorkerStatusResponse['trail'] = undefined;
  if (lastAlert.success && lastAlert.data.length > 0) {
    const a = lastAlert.data[0];
    trail = {
      last_deposit: `alert_fired: ${a.rule_name}`,
      at: a.fired_at,
      type: a.trigger_type,
      detail: `${a.engagement_id} — ${a.success ? 'sent' : 'failed'}`,
    };
  }

  // D1 health: from activity_log counts
  // Note: alert-worker doesn't use flow_logs. Approximate from alert_history.
  const historyResult = await db.getAlertHistory(undefined, 100);
  let d1Health: WorkerStatusResponse['d1_health'] = undefined;
  if (historyResult.success) {
    const all = historyResult.data;
    const failures = all.filter(a => !a.success);
    d1Health = {
      total_executions: all.length,
      total_failures: failures.length,
      failure_rate: all.length > 0 ? `${((failures.length / all.length) * 100).toFixed(1)}%` : '0%',
      last_execution: all.length > 0 ? {
        status: all[0].success ? 'completed' : 'failed',
        duration_ms: 0,  // alert-worker doesn't track duration
        at: all[0].fired_at,
      } : undefined,
      last_failure: failures.length > 0 ? {
        error: failures[0].error_message ?? 'unknown',
        at: failures[0].fired_at,
      } : undefined,
    };
  }

  return Response.json({
    identity: {
      worker: 'tastematter-alert-worker',
      display_name: 'Tastematter Alert Worker',
      system_id: 'tastematter-platform',
      account_id: '4c8353a21e0bfc69a1e036e223cba4d8',
    },
    vitals: {
      status: 'ok',
      features: { alerting: true, publishing: !!env.CONTEXT_DO },
    },
    corpus,
    trail,
    d1_health: d1Health,
    schedule: {
      cron: '0 */4 * * *',
      last_run: trail?.at,
    },
  } satisfies WorkerStatusResponse);
}
```

### Type Import

Add to `apps/tastematter/alert-worker/src/types.ts`:

```typescript
// Copy from control plane's /status contract
export interface WorkerStatusResponse {
  identity: { worker: string; display_name: string; system_id?: string; account_id?: string; version?: string };
  vitals: { status: 'ok' | 'degraded' | 'error'; started_at?: string; features?: Record<string, boolean> };
  corpus?: { commit: string; file_count: number; loaded_at: string; source_repo?: string };
  trail?: { last_deposit: string; at: string; type: string; detail?: string };
  d1_health?: { total_executions: number; total_failures: number; failure_rate: string; last_execution?: { status: string; duration_ms: number; at: string }; last_failure?: { error: string; at: string } };
  schedule?: { cron: string; last_run?: string; next_run?: string };
}
```

## Template for Other Workers

Workers without D1, corpus, or complex features can return a minimal /status:

```typescript
// Minimal /status for simple workers
if (url.pathname === "/status" && request.method === "GET") {
  return Response.json({
    identity: {
      worker: 'WORKER_ID',
      display_name: 'WORKER_DISPLAY_NAME',
      system_id: 'SYSTEM_ID',       // optional
      account_id: 'ACCOUNT_ID',     // optional
    },
    vitals: {
      status: 'ok',
      features: {},
    },
    // Add corpus, trail, d1_health, schedule as applicable
  });
}
```

**Workers to migrate (priority order):**

| Worker | System | Has Corpus | Has D1 | Priority |
|--------|--------|-----------|--------|----------|
| alert-worker | tastematter-platform | Yes | Yes | **Phase 3** (reference) |
| control-plane | tastematter-platform | No | Yes | **Phase 3** (self-monitor) |
| nickel-conference-pr | client-deployments | Yes | Yes | Post-v1 |
| transcript-processing | intel-pipeline | No | Yes | Post-v1 |
| intelligence-pipeline | intel-pipeline | No | Yes | Post-v1 |
| pixee-linkedin-digest | client-deployments | No | No | Post-v1 |

## Control Plane Self-Monitoring

Add `/status` to the control plane itself (`apps/tastematter/control-plane/src/index.ts`):

```typescript
if (url.pathname === "/status" && request.method === "GET") {
  const db = createDB(env.DB);
  const workers = await db.getEnabledWorkers();
  const systems = await db.getSystems();

  return Response.json({
    identity: {
      worker: 'tastematter-control-plane',
      display_name: 'Control Plane',
      system_id: 'tastematter-platform',
      account_id: '4c8353a21e0bfc69a1e036e223cba4d8',
    },
    vitals: {
      status: 'ok',
      features: { health_polling: true, system_grouping: true, sync_tracking: true },
    },
    d1_health: {
      total_executions: workers.length,  // workers monitored
      total_failures: 0,
      failure_rate: '0%',
    },
    schedule: {
      cron: '0 * * * *',
    },
  } satisfies WorkerStatusResponse);
}
```

## Tests

### Alert Worker Tests

- `GET /status` — returns valid WorkerStatusResponse shape
- `GET /status` — includes corpus info when ContextDO available
- `GET /status` — handles ContextDO unavailable gracefully
- `GET /status` — includes last alert as trail
- `GET /status` — d1_health counts match alert_history

### Control Plane Tests

- `GET /status` — returns valid WorkerStatusResponse shape
- `GET /status` — d1_health reflects worker count
- `GET /health` — still works (backwards compatibility)

## Success Criteria

- [ ] Alert worker `GET /status` returns full contract with corpus, trail, d1_health
- [ ] Control plane `GET /status` returns self-monitoring data
- [ ] Both still serve `GET /health` for backwards compatibility
- [ ] Control plane polling now receives richer data from alert worker
- [ ] All existing tests still pass + 5-8 new tests

## Pitfalls

1. **Don't remove /health** — Other tools and CF Access health checks may use /health. Keep it.
2. **Route order matters** — /status must be checked BEFORE /health in the route handler since /status is more specific.
3. **D1 queries in /status must be fast** — The control plane polls /status with a 5s timeout. Keep queries simple (no full table scans).
4. **Corpus DO fetch can hang** — Use the existing try/catch pattern. If DO is slow, skip corpus info rather than timing out the whole /status.
5. **WorkerStatusResponse type duplication** — For v1, duplicating the type in each worker's types.ts is fine. Future: extract to shared package.
