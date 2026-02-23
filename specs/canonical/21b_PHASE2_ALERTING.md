# Phase 2: System Health Alerting

**Parent:** [[21_STIGMERGIC_CONTROL_PLANE_V2]]
**Depends on:** [[21a_PHASE1_FOUNDATION]] (system health computation must exist)
**Parallel with:** [[21c_PHASE3_STATUS_IMPLEMENTATION]]
**Estimated:** ~100 lines, 8-10 tests

---

## Mission

Wire Knock notifications for system health state transitions. When a system goes HEALTHY → BROKEN or BROKEN → HEALTHY, trigger a Knock workflow that notifies via email + in-app.

## Prerequisites

- Phase 1 complete: system_registry with current_status and status_changed_at
- Cron handler computes system health and detects transitions
- Knock API key in control plane env (`KNOCK_API_KEY`)
- Knock workflow created for system health alerts

## Existing Patterns to Reuse

The alert-worker already has Knock integration at `apps/tastematter/alert-worker/src/knock.ts`:

```typescript
export async function triggerKnockWorkflow(
  apiKey: string,
  workflowKey: string,
  payload: KnockTriggerPayload,
): Promise<Result<{ workflow_run_id: string }>> {
  // Single fetch() to api.knock.app
}
```

**Copy this pattern, not the code.** Control plane gets its own Knock function since it's a separate worker with separate concerns.

## Files to Create/Modify

### New Files

**`src/knock.ts`** (~40 lines)

```typescript
import type { Result } from './types.js';

interface SystemAlertPayload {
  recipients: string[];
  data: {
    system_id: string;
    system_name: string;
    previous_status: string;
    current_status: string;
    changed_at: string;
    affected_workers: Array<{ name: string; status: string; error?: string }>;
    summary: string;
  };
}

export async function triggerSystemAlert(
  apiKey: string,
  workflowKey: string,
  payload: SystemAlertPayload,
): Promise<Result<{ workflow_run_id: string }>> {
  const resp = await fetch(`https://api.knock.app/v1/workflows/${workflowKey}/trigger`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${apiKey}`,
    },
    body: JSON.stringify(payload),
  });
  if (!resp.ok) {
    return { success: false, error: `Knock ${resp.status}: ${await resp.text()}` };
  }
  const body = await resp.json() as { workflow_run_id: string };
  return { success: true, data: body };
}
```

### Modified Files

**`src/types.ts`** — Add to Env:

```typescript
export interface Env {
  // ... existing
  KNOCK_WORKFLOW_KEY: string;  // NEW: workflow key for system alerts
}
```

**`src/index.ts`** — Modify cron handler:

```typescript
// After computing system health and detecting transition:
if (previousStatus !== newStatus && previousStatus !== null) {
  const members = workers.filter(w => w.system_id === system.id);
  const result = await triggerSystemAlert(env.KNOCK_API_KEY, env.KNOCK_WORKFLOW_KEY, {
    recipients: [env.OWNER_ID],
    data: {
      system_id: system.id,
      system_name: system.display_name,
      previous_status: previousStatus,
      current_status: newStatus,
      changed_at: new Date().toISOString(),
      affected_workers: members.map(m => ({
        name: m.display_name,
        status: m.current_status ?? 'unknown',
        error: m.error_message ?? undefined,
      })),
      summary: buildAlertSummary(system, newStatus, members),
    },
  });

  if (result.success) {
    console.log(`Knock alert sent for ${system.display_name}: ${previousStatus} → ${newStatus}`);
  } else {
    console.error(`Knock alert failed for ${system.display_name}: ${result.error}`);
  }
}
```

**Helper:**
```typescript
function buildAlertSummary(system: SystemRegistryRow, status: SystemStatus, members: WorkerWithStatus[]): string {
  if (status === 'broken') {
    const downWorkers = members.filter(m => m.current_status === 'down' || m.current_status === 'timeout');
    return `${system.display_name} is BROKEN. ${downWorkers.map(w => w.display_name).join(', ')} down.`;
  }
  return `${system.display_name} recovered to ${status}.`;
}
```

## Implementation Steps

### Step 1: Knock Workflow Setup
1. Create workflow in Knock dashboard: `system-health-alert`
2. Template: email + in-app feed
3. Email template uses: `{{data.system_name}}`, `{{data.previous_status}}`, `{{data.current_status}}`, `{{data.summary}}`

### Step 2: Knock Integration Code
1. Create `src/knock.ts` with `triggerSystemAlert` function
2. Add `KNOCK_WORKFLOW_KEY` to `src/types.ts` Env
3. Set secret: `printf "system-health-alert" | wrangler secret put KNOCK_WORKFLOW_KEY`

### Step 3: Wire Into Cron
1. In cron handler, after system health computation and transition detection
2. Call `triggerSystemAlert` with affected worker details
3. Log success/failure

### Step 4: Deduplication
1. Only fire on actual transitions (status changed, not continued failure)
2. Skip first check (no previous status → don't alert on initial poll)
3. Include `status_changed_at` in system_registry to prevent re-alerting

## Tests

### Unit Tests
- `triggerSystemAlert_success` — mock fetch returns 200, returns workflow_run_id
- `triggerSystemAlert_failure` — mock fetch returns 500, returns error
- `buildAlertSummary_broken` — correct summary with down worker names
- `buildAlertSummary_recovered` — correct recovery message

### Integration Tests (cron behavior)
- `cron_sendsKnockOnTransition` — system goes healthy → broken, Knock called
- `cron_skipsKnockOnContinuedFailure` — system stays broken, Knock NOT called
- `cron_sendsKnockOnRecovery` — system goes broken → healthy, Knock called
- `cron_skipsKnockOnFirstCheck` — no previous status, Knock NOT called

## Success Criteria

- [ ] Knock workflow `system-health-alert` created in dashboard
- [ ] `KNOCK_WORKFLOW_KEY` secret set on control plane
- [ ] System transition healthy → broken triggers Knock notification
- [ ] System transition broken → healthy triggers recovery notification
- [ ] Continued failure does NOT spam notifications
- [ ] First check does NOT trigger notification
- [ ] All 8-10 tests pass

## Pitfalls

1. **`printf` not `echo` for wrangler secrets** — echo adds trailing newline that corrupts the API key.
2. **Knock workflow must exist before triggering** — 404 from Knock if workflow key is wrong.
3. **OWNER_ID must be a valid Knock recipient** — The user must be identified in Knock first (alert-worker should have already done this).
4. **Rate limiting** — Knock has rate limits. With hourly polling and ~3 systems, this is well within limits. But don't trigger on every poll — only transitions.
