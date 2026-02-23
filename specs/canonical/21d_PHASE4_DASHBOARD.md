# Phase 4: Dashboard — Visibility + Control Surface

**Parent:** [[21_STIGMERGIC_CONTROL_PLANE_V2]]
**Depends on:** [[21a_PHASE1_FOUNDATION]], [[21b_PHASE2_ALERTING]], [[21c_PHASE3_STATUS_IMPLEMENTATION]]
**Estimated:** ~400 lines, manual testing

---

## Mission

Rebuild the web app dashboard to show system-level health, rich worker detail, push/pull delta indicators, and control actions. All data loaded server-side via `+page.server.ts` using CF Access service tokens.

## Prerequisites

- Phase 1: control plane serves GET /systems, GET /sync/:id, proxy routes
- Phase 2: system health transitions trigger Knock (dashboard just reflects state)
- Phase 3: alert-worker and control-plane serve /status (richer data available)

## Architecture: SvelteKit SSR

**Pattern from [[cloudflare-fullstack-engineering]] skill:**

```
Browser (app.tastematter.dev)
  └── +page.svelte (renders data from server load)
        └── +page.server.ts (runs on CF edge)
              ├── GET control.tastematter.dev/systems   (systems + members + health)
              ├── GET control.tastematter.dev/workers    (all workers with status)
              └── GET api.tastematter.dev/alert/history  (recent alerts)

Control actions:
  └── +page.server.ts (form actions or API routes)
        ├── POST control.tastematter.dev/workers/:id/check    (force check)
        ├── POST control.tastematter.dev/workers/:id/reload   (reload corpus)
        ├── PATCH control.tastematter.dev/workers/:id          (toggle enabled)
        └── POST control.tastematter.dev/systems              (register system)
```

**Key rule:** All fetches happen server-side with service token headers. Browser never talks to control plane or alert worker directly. Eliminates CORS and cross-origin cookie problems.

## Files to Modify

### `src/routes/+page.server.ts`

Expand data loading:

```typescript
import type { PageServerLoad, Actions } from './$types';

const CONTROL_PLANE = 'https://control.tastematter.dev';
const ALERT_WORKER = 'https://api.tastematter.dev';

function getAuthHeaders(env: any): Record<string, string> {
  const clientId = String(env?.CF_ACCESS_CLIENT_ID ?? '');
  const clientSecret = String(env?.CF_ACCESS_CLIENT_SECRET ?? '');
  if (!clientId || !clientSecret) return {};
  return {
    'CF-Access-Client-Id': clientId,
    'CF-Access-Client-Secret': clientSecret,
  };
}

export const load: PageServerLoad = async ({ platform }) => {
  const auth = getAuthHeaders(platform?.env);

  const [systemsRes, workersRes, alertsRes] = await Promise.all([
    fetch(`${CONTROL_PLANE}/systems`, { headers: auth }).catch(() => null),
    fetch(`${CONTROL_PLANE}/workers`, { headers: auth }).catch(() => null),
    fetch(`${ALERT_WORKER}/alert/history?limit=50`, { headers: auth }).catch(() => null),
  ]);

  const systems = systemsRes?.ok ? (await systemsRes.json() as any).data ?? [] : [];
  const workers = workersRes?.ok ? (await workersRes.json() as any).data ?? [] : [];
  const alerts = alertsRes?.ok ? (await alertsRes.json() as any).data ?? [] : [];

  return {
    systems,
    workers,
    alerts,
    systemsError: systemsRes?.ok ? null : 'Failed to load systems',
    workersError: workersRes?.ok ? null : 'Failed to load workers',
    alertsError: alertsRes?.ok ? null : 'Failed to load alerts',
  };
};

// Form actions for control surface
export const actions: Actions = {
  forceCheck: async ({ request, platform }) => {
    const auth = getAuthHeaders(platform?.env);
    const data = await request.formData();
    const workerId = data.get('worker_id');
    await fetch(`${CONTROL_PLANE}/workers/${workerId}/check`, {
      method: 'POST', headers: auth,
    });
  },
  reloadCorpus: async ({ request, platform }) => {
    const auth = getAuthHeaders(platform?.env);
    const data = await request.formData();
    const workerId = data.get('worker_id');
    await fetch(`${CONTROL_PLANE}/workers/${workerId}/reload`, {
      method: 'POST', headers: auth,
    });
  },
  toggleWorker: async ({ request, platform }) => {
    const auth = getAuthHeaders(platform?.env);
    const data = await request.formData();
    const workerId = data.get('worker_id');
    const enabled = data.get('enabled') === '1' ? 0 : 1;  // toggle
    await fetch(`${CONTROL_PLANE}/workers/${workerId}`, {
      method: 'PATCH',
      headers: { ...auth, 'Content-Type': 'application/json' },
      body: JSON.stringify({ enabled }),
    });
  },
};
```

### `src/routes/+page.svelte`

Rewrite to show:

**1. System Cards** (top section)
- One card per system
- System name + HEALTHY/BROKEN/DEGRADED badge
- Member workers with status dots and detail
- System summary (trail health, corpus freshness)
- Expand/collapse for worker detail

**2. Control Actions** (per worker)
- Force Check button (form action → forceCheck)
- Reload Corpus button (form action → reloadCorpus, only if worker has corpus)
- Toggle Enabled switch (form action → toggleWorker)

**3. Active Engagements** (existing section, enriched)
- Alert history with rule_name, trigger_type, success/failure
- Not just count + last_fired

**4. Push/Pull Delta Indicators**
- Per worker: show last sync commit vs /status corpus commit
- Visual indicator: green (match), yellow (mismatch), gray (no sync data)

## Component Structure

```svelte
<h1>Dashboard</h1>

<!-- System cards -->
<section class="systems">
  <h2>Systems</h2>
  {#each data.systems as system}
    <SystemCard {system} workers={data.workers.filter(w => w.system_id === system.id)} />
  {/each}

  <!-- Ungrouped workers -->
  {#if ungroupedWorkers.length > 0}
    <h3>Ungrouped Workers</h3>
    {#each ungroupedWorkers as w}
      <WorkerRow worker={w} />
    {/each}
  {/if}
</section>

<!-- Alert history -->
<section class="alerts">
  <h2>Recent Alerts</h2>
  {#each data.alerts as alert}
    <AlertRow {alert} />
  {/each}
</section>
```

**SystemCard** shows:
```
┌─ Intelligence Pipeline ──────────────────── BROKEN ─┐
│                                                      │
│ ● Transcript Processing    DOWN 6d    [Force Check]  │
│   Error: HTTP 500 — D1 binding missing               │
│                                                      │
│ ● Intelligence Generation  STALE      [Force Check]  │
│   Last active: 3d ago                                │
│                                                      │
│ Trail: broken at ingestion                           │
│ D1: 47 executions, 3 failures (6.4%)                 │
├──────────────────────────────────────────────────────┤
│ [Force Check All] [Reload Corpus]                    │
└──────────────────────────────────────────────────────┘
```

**WorkerRow** shows:
- Status dot (color based on status)
- Worker name
- Status label + time context (e.g., "DOWN 6d", "cron 2h ago")
- Error message (if down/timeout, show error_message from health_log)
- Action buttons (Force Check, Reload if has corpus, Toggle)

## Data Model (from server load)

```typescript
// System with members (from GET /systems)
interface SystemData {
  id: string;
  display_name: string;
  description: string | null;
  health_rule: string;
  current_status: string;
  status_changed_at: string | null;
  members: WorkerData[];
}

// Worker with status (from GET /workers)
interface WorkerData {
  id: string;
  display_name: string;
  system_id: string | null;
  account_id: string | null;
  current_status: string | null;
  last_checked: string | null;
  last_activity: string | null;
  last_response_time_ms: number | null;
  expected_cadence: string | null;
  error_message: string | null;
  raw_response: string | null;   // /status JSON for parsing corpus, trail, d1_health
  enabled: number;
  // sync data (if available)
  latest_sync_commit: string | null;
  latest_sync_at: string | null;
}

// Alert history (from GET /alert/history)
interface AlertData {
  engagement_id: string;
  rule_name: string;
  trigger_type: string;
  fired_at: string;
  success: number;
  error_message: string | null;
}
```

## Implementation Steps

### Step 1: Server-Side Data Loading
1. Expand +page.server.ts to fetch /systems and /workers
2. Parse raw_response from workers to extract corpus, trail, d1_health
3. Return structured data to page

### Step 2: Form Actions
1. Add SvelteKit form actions for forceCheck, reloadCorpus, toggleWorker
2. Each action POSTs/PATCHes to control plane with auth headers
3. Page reloads after action (SvelteKit default behavior)

### Step 3: System Cards
1. Create SystemCard component (or inline in +page.svelte)
2. Group workers by system_id
3. Show system badge (HEALTHY/BROKEN/DEGRADED)
4. Show member workers with status details
5. Show system-level summary (trail health, D1 aggregate)

### Step 4: Control Actions UI
1. Add form buttons for Force Check, Reload Corpus, Toggle
2. Use SvelteKit `use:enhance` for progressive enhancement
3. Show loading state during action

### Step 5: Alert History Enrichment
1. Show rule_name, trigger_type, success/failure
2. Group by engagement (existing pattern)
3. Show error_message for failed alerts

### Step 6: Delta Indicators
1. Parse latest_sync_commit from worker data
2. Parse corpus.commit from raw_response
3. Compare: match = green, mismatch = yellow, no data = gray
4. Show delta inline on worker card

## Deployment

```bash
# Deploy to production (MUST use --branch main)
wrangler pages deploy dist --project-name tastematter-web-app --branch main

# Verify
curl -s https://app.tastematter.dev/__data.json | head -c 200
```

**Key rules from [[cloudflare-fullstack-engineering]]:**
- `--branch main` for production (NOT preview)
- Redeploy after setting secrets
- `String(env?.KEY ?? '')` for platform.env access
- `printf` not `echo` for secrets

## Success Criteria

- [ ] Dashboard loads with system grouping
- [ ] System cards show HEALTHY/BROKEN with correct computation
- [ ] Worker detail shows error context when down
- [ ] Force Check button triggers health check and page reloads with result
- [ ] Reload Corpus button proxies to worker
- [ ] Toggle button enables/disables worker
- [ ] Alert history shows rule_name and trigger_type
- [ ] Delta indicators show push/pull comparison
- [ ] All data loaded server-side (no client-side fetches to workers)

## Pitfalls

1. **`--branch main`** — Without this flag, `wrangler pages deploy` targets preview. Custom domain only serves production.
2. **`String(env?.KEY ?? '')`** — Don't cast platform.env to `Record<string,string>`. Use String() coercion for each key.
3. **Redeploy after secrets** — Pages secrets bind at deployment time, unlike Worker secrets.
4. **raw_response parsing** — The health_log.raw_response contains the full /status JSON. Parse it client-side in +page.svelte to extract corpus, trail, d1_health. Handle malformed JSON gracefully.
5. **Form actions need CSRF** — SvelteKit form actions handle CSRF automatically when using `<form method="POST">`. Don't bypass with client-side fetch.
6. **CORS not needed** — Server-side loading eliminates CORS. If you find yourself adding CORS headers to the control plane for the dashboard, you're doing it wrong.

## Integration Points

- **Phase 1** provides: GET /systems, GET /workers (with system_id), GET /sync/:id, PATCH /workers/:id, proxy routes
- **Phase 2** provides: system_registry.current_status reflects Knock-alerting health transitions
- **Phase 3** provides: richer raw_response from workers serving /status (corpus, trail, d1_health)
- **Dashboard** is purely a rendering layer — it computes nothing, only displays what the control plane provides
