<script lang="ts">
  import { enhance } from '$app/forms';

  interface SystemData {
    id: string;
    display_name: string;
    description: string | null;
    health_rule: string;
    current_status: string;
    status_changed_at: string | null;
    members: WorkerData[];
  }

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
    raw_response: string | null;
    enabled: number;
  }

  interface AlertData {
    engagement_id: string;
    rule_name: string;
    trigger_type: string;
    fired_at: string;
    success: number;
    error_message: string | null;
  }

  let { data } = $props();

  // Workers not in any system
  const systemWorkerIds = new Set(
    (data.systems ?? []).flatMap((s: SystemData) => (s.members ?? []).map((m: WorkerData) => m.id))
  );
  const ungroupedWorkers = (data.workers ?? []).filter((w: WorkerData) => !systemWorkerIds.has(w.id));

  function formatRelativeTime(iso: string | null): string {
    if (!iso) return 'never';
    const diff = Date.now() - new Date(iso).getTime();
    const mins = Math.floor(diff / 60000);
    if (mins < 1) return 'just now';
    if (mins < 60) return `${mins}m ago`;
    const hours = Math.floor(mins / 60);
    if (hours < 24) return `${hours}h ago`;
    const days = Math.floor(hours / 24);
    return `${days}d ago`;
  }

  function statusColor(status: string | null): string {
    switch (status) {
      case 'healthy': case 'reachable': return '#22c55e';
      case 'stale': case 'degraded': return '#eab308';
      case 'down': case 'timeout': case 'broken': return '#ef4444';
      default: return '#71717a';
    }
  }

  function systemStatusLabel(status: string | null): string {
    switch (status) {
      case 'healthy': return 'HEALTHY';
      case 'degraded': return 'DEGRADED';
      case 'broken': return 'BROKEN';
      default: return 'UNKNOWN';
    }
  }

  function parseStatusInfo(raw: string | null): { corpus?: any; trail?: any; d1_health?: any } {
    if (!raw) return {};
    try {
      const parsed = JSON.parse(raw);
      if (parsed?.identity?.worker) return parsed; // full /status response
      return {};
    } catch { return {}; }
  }

  // Group alerts by engagement with full detail
  function groupAlerts(alerts: AlertData[]) {
    const grouped = new Map<string, { count: number; lastFired: string; lastRule: string; lastTrigger: string; failures: number }>();
    for (const a of alerts) {
      const id = a.engagement_id || 'unknown';
      const existing = grouped.get(id);
      if (!existing) {
        grouped.set(id, { count: 1, lastFired: a.fired_at, lastRule: a.rule_name, lastTrigger: a.trigger_type, failures: a.success ? 0 : 1 });
      } else {
        existing.count++;
        if (!a.success) existing.failures++;
        if (a.fired_at > existing.lastFired) {
          existing.lastFired = a.fired_at;
          existing.lastRule = a.rule_name;
          existing.lastTrigger = a.trigger_type;
        }
      }
    }
    return Array.from(grouped.entries()).map(([id, d]) => ({
      id,
      name: id.replace(/_/g, ' ').replace(/\b\w/g, (c: string) => c.toUpperCase()),
      ...d,
      lastFiredRelative: formatRelativeTime(d.lastFired),
    }));
  }

  const engagements = groupAlerts(data.alerts ?? []);
</script>

<h1>Tastematter</h1>

<!-- ═══ SYSTEMS ═══ -->
<section>
  <h2>Systems</h2>
  {#if data.systemsError}
    <p class="error">Control plane: {data.systemsError}</p>
  {:else if (data.systems ?? []).length === 0}
    <p class="muted">No systems registered.</p>
  {:else}
    {#each data.systems as system}
      {@const statusInfo = system.current_status}
      <div class="system-card" class:system-broken={statusInfo === 'broken'}>
        <div class="system-header">
          <div class="system-title">
            <span class="system-name">{system.display_name}</span>
            {#if system.description}
              <span class="system-desc">{system.description}</span>
            {/if}
          </div>
          <span class="badge" style="background: {statusColor(statusInfo)}20; color: {statusColor(statusInfo)}">
            {systemStatusLabel(statusInfo)}
          </span>
        </div>

        {#if (system.members ?? []).length > 0}
          <div class="members">
            {#each system.members as w}
              {@const info = parseStatusInfo(w.raw_response)}
              <div class="worker-row">
                <div class="worker-left">
                  <span class="dot" style="background: {statusColor(w.current_status)}"></span>
                  <span class="worker-name">{w.display_name}</span>
                  <span class="worker-status" style="color: {statusColor(w.current_status)}">{w.current_status ?? 'unknown'}</span>
                </div>
                <div class="worker-right">
                  {#if w.last_response_time_ms != null}
                    <span class="mono muted">{w.last_response_time_ms}ms</span>
                  {/if}
                  <div class="actions">
                    <form method="POST" action="?/forceCheck" use:enhance>
                      <input type="hidden" name="worker_id" value={w.id} />
                      <button class="btn-sm" type="submit">Check</button>
                    </form>
                  </div>
                </div>
              </div>
              <div class="worker-detail">
                <span>checked {formatRelativeTime(w.last_checked)}</span>
                {#if w.last_activity}
                  <span>active {formatRelativeTime(w.last_activity)}</span>
                {/if}
                {#if w.expected_cadence}
                  <span>cadence: {w.expected_cadence}</span>
                {/if}
                {#if info.corpus}
                  <span>corpus: {info.corpus.file_count} files @ {info.corpus.commit?.slice(0, 7)}</span>
                {/if}
              </div>
              {#if w.error_message && (w.current_status === 'down' || w.current_status === 'timeout')}
                <div class="worker-error">{w.error_message}</div>
              {/if}
              {#if info.trail}
                <div class="worker-trail">
                  trail: {info.trail.last_deposit} ({formatRelativeTime(info.trail.at)})
                </div>
              {/if}
            {/each}
          </div>
        {:else}
          <p class="muted small">No workers assigned to this system.</p>
        {/if}
      </div>
    {/each}
  {/if}

  <!-- Ungrouped workers -->
  {#if ungroupedWorkers.length > 0}
    <h3>Ungrouped Workers</h3>
    {#each ungroupedWorkers as w}
      <div class="worker-card">
        <div class="worker-row">
          <div class="worker-left">
            <span class="dot" style="background: {statusColor(w.current_status)}"></span>
            <span class="worker-name">{w.display_name}</span>
            <span class="worker-status" style="color: {statusColor(w.current_status)}">{w.current_status ?? 'unknown'}</span>
          </div>
          <div class="worker-right">
            {#if w.last_response_time_ms != null}
              <span class="mono muted">{w.last_response_time_ms}ms</span>
            {/if}
            <form method="POST" action="?/forceCheck" use:enhance>
              <input type="hidden" name="worker_id" value={w.id} />
              <button class="btn-sm" type="submit">Check</button>
            </form>
          </div>
        </div>
        <div class="worker-detail">
          <span>checked {formatRelativeTime(w.last_checked)}</span>
        </div>
      </div>
    {/each}
  {/if}

  {#if data.workersError}
    <p class="error">Workers: {data.workersError}</p>
  {/if}
</section>

<!-- ═══ ALERTS ═══ -->
<section>
  <h2>Alert History</h2>
  {#if data.alertsError}
    <p class="error">API: {data.alertsError}</p>
  {:else if engagements.length === 0}
    <p class="muted">No alert history.</p>
  {:else}
    {#each engagements as eng}
      <div class="alert-card">
        <div class="alert-header">
          <span class="name">{eng.name}</span>
          <div class="alert-meta">
            <span class="count">{eng.count} alerts</span>
            {#if eng.failures > 0}
              <span class="count failures">{eng.failures} failed</span>
            {/if}
          </div>
        </div>
        <div class="alert-detail">
          <span>Last: {eng.lastRule} ({eng.lastTrigger})</span>
          <span>{eng.lastFiredRelative}</span>
        </div>
      </div>
    {/each}
  {/if}
</section>

<style>
  h1 { font-size: 24px; font-weight: 700; margin-bottom: 24px; color: #fafafa; }
  h2 { font-size: 14px; font-weight: 600; color: #a1a1aa; margin-bottom: 12px; letter-spacing: 0.5px; text-transform: uppercase; }
  h3 { font-size: 13px; font-weight: 600; color: #71717a; margin: 16px 0 8px; }
  section { margin-bottom: 40px; }

  .system-card {
    background: #18181b;
    border: 1px solid #27272a;
    border-radius: 8px;
    padding: 16px;
    margin-bottom: 12px;
  }
  .system-broken { border-color: #7f1d1d; }

  .system-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: 12px;
  }
  .system-title { display: flex; flex-direction: column; gap: 2px; }
  .system-name { font-weight: 600; font-size: 16px; color: #fafafa; }
  .system-desc { font-size: 12px; color: #71717a; }

  .badge {
    font-size: 11px;
    font-weight: 700;
    padding: 3px 10px;
    border-radius: 4px;
    letter-spacing: 0.5px;
    flex-shrink: 0;
  }

  .members { display: flex; flex-direction: column; }

  .worker-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 0 2px;
    border-top: 1px solid #1f1f23;
  }
  .worker-row:first-child { border-top: none; }

  .worker-left { display: flex; align-items: center; gap: 10px; }
  .worker-right { display: flex; align-items: center; gap: 12px; }

  .dot { width: 8px; height: 8px; border-radius: 50%; flex-shrink: 0; }
  .worker-name { font-size: 14px; font-weight: 500; color: #e4e4e7; }
  .worker-status { font-size: 11px; font-weight: 600; text-transform: uppercase; letter-spacing: 0.3px; }

  .worker-detail {
    display: flex;
    gap: 16px;
    padding: 2px 0 6px 18px;
    font-size: 12px;
    color: #71717a;
  }
  .worker-error {
    padding: 4px 0 4px 18px;
    font-size: 12px;
    color: #ef4444;
    font-family: 'JetBrains Mono', monospace;
  }
  .worker-trail {
    padding: 0 0 4px 18px;
    font-size: 12px;
    color: #52525b;
    font-family: 'JetBrains Mono', monospace;
  }

  .worker-card {
    background: #18181b;
    border: 1px solid #27272a;
    border-radius: 8px;
    padding: 12px 16px;
    margin-bottom: 8px;
  }

  .actions { display: flex; gap: 6px; }
  .btn-sm {
    font-size: 11px;
    padding: 3px 10px;
    border-radius: 4px;
    border: 1px solid #3f3f46;
    background: #27272a;
    color: #d4d4d8;
    cursor: pointer;
    font-family: inherit;
  }
  .btn-sm:hover { background: #3f3f46; color: #fafafa; }

  .alert-card {
    background: #18181b;
    border: 1px solid #27272a;
    border-radius: 8px;
    padding: 14px 16px;
    margin-bottom: 8px;
  }
  .alert-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 4px;
  }
  .alert-detail {
    display: flex;
    justify-content: space-between;
    font-size: 12px;
    color: #71717a;
  }

  .name { font-weight: 600; font-size: 15px; color: #e4e4e7; }
  .count {
    font-size: 11px;
    color: #a1a1aa;
    background: #27272a;
    padding: 2px 8px;
    border-radius: 4px;
  }
  .failures { color: #ef4444; background: #2a0a0a; }

  .mono { font-family: 'JetBrains Mono', monospace; }
  .muted { color: #71717a; }
  .small { font-size: 12px; }
  .error { color: #ef4444; font-size: 12px; }
</style>
