import type { Env, SystemStatus, WorkerStatusResponse } from "./types.js";
import { createDB } from "./db.js";
import { checkWorkerHealth, shouldAlert, computeSystemHealth } from "./health-checker.js";
import { triggerSystemAlert, buildAlertSummary } from "./knock.js";

const ALLOWED_ORIGINS = ["https://app.tastematter.dev"];

function corsHeaders(request: Request): Record<string, string> {
  const origin = request.headers.get("Origin") || "";
  if (!ALLOWED_ORIGINS.includes(origin)) return {};
  return {
    "Access-Control-Allow-Origin": origin,
    "Access-Control-Allow-Credentials": "true",
    "Access-Control-Allow-Methods": "GET, POST, PATCH, DELETE, OPTIONS",
    "Access-Control-Allow-Headers": "Content-Type",
  };
}

function withCors(response: Response, request: Request): Response {
  const headers = corsHeaders(request);
  if (Object.keys(headers).length === 0) return response;
  const newResp = new Response(response.body, response);
  for (const [k, v] of Object.entries(headers)) {
    newResp.headers.set(k, v);
  }
  return newResp;
}

function extractId(pathname: string, position: number): string {
  return decodeURIComponent(pathname.split("/")[position]);
}

async function handleRequest(
  url: URL,
  request: Request,
  env: Env,
  ctx: ExecutionContext,
): Promise<Response> {
  const db = createDB(env.DB);
  const method = request.method;

  // --- Rich status (self-monitoring) ---

  if (url.pathname === "/status" && method === "GET") {
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
        total_executions: workers.length,
        total_failures: 0,
        failure_rate: '0%',
      },
      schedule: {
        cron: '0 * * * *',
      },
    } satisfies WorkerStatusResponse);
  }

  // --- Health ---

  if (url.pathname === "/health" && method === "GET") {
    return Response.json({
      status: "ok",
      worker: "tastematter-control-plane",
      timestamp: new Date().toISOString(),
    });
  }

  // --- Systems ---

  if (url.pathname === "/systems" && method === "GET") {
    const systems = await db.getSystemsWithMembers();
    return Response.json({ data: systems });
  }

  if (url.pathname === "/systems" && method === "POST") {
    const body = await request.json() as any;
    if (!body.id || !body.display_name) {
      return Response.json({ error: "Required: id, display_name" }, { status: 400 });
    }
    await db.registerSystem({
      id: body.id,
      display_name: body.display_name,
      description: body.description ?? null,
      health_rule: body.health_rule ?? "all",
    });
    return Response.json({ success: true, id: body.id }, { status: 201 });
  }

  if (url.pathname.match(/^\/systems\/[^/]+$/) && method === "DELETE") {
    const id = extractId(url.pathname, 2);
    await db.deleteSystem(id);
    return Response.json({ success: true });
  }

  // --- Workers ---

  if (url.pathname === "/workers" && method === "GET") {
    const workers = await db.getWorkersWithStatus();
    return Response.json({ data: workers });
  }

  if (url.pathname === "/workers" && method === "POST") {
    const body = await request.json() as any;
    if (!body.id || !body.display_name || !body.health_url) {
      return Response.json({ error: "Required: id, display_name, health_url" }, { status: 400 });
    }
    await db.registerWorker({
      id: body.id,
      display_name: body.display_name,
      health_url: body.health_url,
      expected_cadence: body.expected_cadence ?? null,
      max_silence_hours: body.max_silence_hours ?? 24,
      auth_type: body.auth_type ?? "none",
      tags: body.tags ? JSON.stringify(body.tags) : null,
      enabled: body.enabled ?? 1,
      system_id: body.system_id ?? null,
      account_id: body.account_id ?? null,
      status_url: body.status_url ?? null,
    });
    return Response.json({ success: true, id: body.id }, { status: 201 });
  }

  if (url.pathname.match(/^\/workers\/[^/]+$/) && method === "PATCH") {
    const id = extractId(url.pathname, 2);
    const worker = await db.getWorker(id);
    if (!worker) {
      return Response.json({ error: "Worker not found" }, { status: 404 });
    }
    const body = await request.json() as any;
    await db.updateWorker(id, body);
    return Response.json({ success: true });
  }

  if (url.pathname.match(/^\/workers\/[^/]+$/) && method === "DELETE") {
    const id = extractId(url.pathname, 2);
    await db.deleteWorker(id);
    return Response.json({ success: true });
  }

  // GET /workers/:id/health — health history
  if (url.pathname.match(/^\/workers\/[^/]+\/health$/) && method === "GET") {
    const id = extractId(url.pathname, 2);
    const worker = await db.getWorker(id);
    if (!worker) {
      return Response.json({ error: "Worker not found" }, { status: 404 });
    }
    const history = await db.getHealthHistory(id);
    return Response.json({ worker, history });
  }

  // POST /workers/:id/check — force health check
  if (url.pathname.match(/^\/workers\/[^/]+\/check$/) && method === "POST") {
    const id = extractId(url.pathname, 2);
    const worker = await db.getWorker(id);
    if (!worker) {
      return Response.json({ error: "Worker not found" }, { status: 404 });
    }
    const result = await checkWorkerHealth(worker, env);
    result.worker_id = id;
    await db.logHealthCheck(result);
    return Response.json({ data: result });
  }

  // POST /workers/:id/reload — proxy reload to worker
  if (url.pathname.match(/^\/workers\/[^/]+\/reload$/) && method === "POST") {
    const id = extractId(url.pathname, 2);
    const worker = await db.getWorker(id);
    if (!worker) {
      return Response.json({ error: "Worker not found" }, { status: 404 });
    }
    const reloadUrl = worker.health_url.replace(/\/health$/, "/reload");
    const headers: Record<string, string> = { "Content-Type": "application/json" };
    if (worker.auth_type === "cf-access") {
      headers["CF-Access-Client-Id"] = env.CF_ACCESS_CLIENT_ID;
      headers["CF-Access-Client-Secret"] = env.CF_ACCESS_CLIENT_SECRET;
    }
    try {
      const resp = await fetch(reloadUrl, { method: "POST", headers });
      const body = await resp.text();
      return new Response(body, { status: resp.status, headers: { "Content-Type": "application/json" } });
    } catch (err: any) {
      return Response.json({ error: `Proxy failed: ${err.message}` }, { status: 502 });
    }
  }

  // POST /workers/:id/trigger — proxy trigger to worker
  if (url.pathname.match(/^\/workers\/[^/]+\/trigger$/) && method === "POST") {
    const id = extractId(url.pathname, 2);
    const worker = await db.getWorker(id);
    if (!worker) {
      return Response.json({ error: "Worker not found" }, { status: 404 });
    }
    const triggerUrl = worker.health_url.replace(/\/health$/, "/alert/trigger");
    const headers: Record<string, string> = { "Content-Type": "application/json" };
    if (worker.auth_type === "cf-access") {
      headers["CF-Access-Client-Id"] = env.CF_ACCESS_CLIENT_ID;
      headers["CF-Access-Client-Secret"] = env.CF_ACCESS_CLIENT_SECRET;
    }
    try {
      const resp = await fetch(triggerUrl, { method: "POST", headers });
      const body = await resp.text();
      return new Response(body, { status: resp.status, headers: { "Content-Type": "application/json" } });
    } catch (err: any) {
      return Response.json({ error: `Proxy failed: ${err.message}` }, { status: 502 });
    }
  }

  // --- Sync ---

  if (url.pathname === "/sync" && method === "POST") {
    const body = await request.json() as any;
    if (!body.worker_id || !body.commit_sha) {
      return Response.json({ error: "Required: worker_id, commit_sha" }, { status: 400 });
    }
    await db.logSync(body);
    return Response.json({ success: true }, { status: 201 });
  }

  if (url.pathname.match(/^\/sync\/[^/]+$/) && method === "GET") {
    const workerId = extractId(url.pathname, 2);
    const history = await db.getSyncHistory(workerId);
    return Response.json({ data: history });
  }

  return Response.json({ error: "Not found" }, { status: 404 });
}

export default {
  async fetch(
    request: Request,
    env: Env,
    ctx: ExecutionContext,
  ): Promise<Response> {
    const url = new URL(request.url);

    if (request.method === "OPTIONS") {
      return new Response(null, { status: 204, headers: corsHeaders(request) });
    }

    const response = await handleRequest(url, request, env, ctx);
    return withCors(response, request);
  },

  async scheduled(
    _event: ScheduledEvent,
    env: Env,
    ctx: ExecutionContext,
  ): Promise<void> {
    const db = createDB(env.DB);
    const workers = await db.getEnabledWorkers();

    let checked = 0;
    let workerAlerts = 0;

    // Phase 1: Poll all workers (skip self — can't poll through CF Access from inside)
    const SELF_ID = "tastematter-control-plane";
    for (const worker of workers) {
      if (worker.id === SELF_ID) {
        // Self-report as healthy without HTTP poll
        await db.logHealthCheck({
          worker_id: SELF_ID, http_status: 200, response_time_ms: 0,
          status: "healthy", last_activity: new Date().toISOString(),
          activity_type: "cron_run", raw_response: null, error_message: null,
        });
        checked++;
        continue;
      }
      const previousLog = await db.getLatestHealthStatus(worker.id);
      const result = await checkWorkerHealth(worker, env);
      await db.logHealthCheck(result);
      checked++;

      const previousStatus = previousLog?.status ?? null;
      if (shouldAlert(result.status, previousStatus as any)) {
        workerAlerts++;
        console.log(
          `WORKER ALERT: ${worker.display_name} changed from ${previousStatus} to ${result.status}`,
        );
      }
    }

    // Phase 2: Compute system health
    const systems = await db.getSystems();
    const allWorkers = await db.getWorkersWithStatus();
    let systemTransitions = 0;

    for (const system of systems) {
      const members = allWorkers.filter((w) => w.system_id === system.id);
      const memberStatuses = members.map((m) => m.current_status).filter(Boolean);
      const newStatus = computeSystemHealth(system.health_rule, memberStatuses);
      const previousStatus = system.current_status;

      if (newStatus !== previousStatus) {
        await db.updateSystemStatus(system.id, newStatus);
        systemTransitions++;
        console.log(
          `SYSTEM TRANSITION: ${system.display_name} changed from ${previousStatus} to ${newStatus}`,
        );

        // Phase 2: Knock alerting — only on real transitions (skip first check)
        if (previousStatus !== null && previousStatus !== "unknown") {
          const result = await triggerSystemAlert(env.KNOCK_API_KEY, env.KNOCK_WORKFLOW_KEY, {
            recipients: [env.OWNER_ID],
            data: {
              system_id: system.id,
              system_name: system.display_name,
              previous_status: previousStatus,
              current_status: newStatus,
              changed_at: new Date().toISOString(),
              affected_workers: members.map((m) => ({
                name: m.display_name,
                status: m.current_status ?? "unknown",
                error: m.error_message ?? undefined,
              })),
              summary: buildAlertSummary(system, newStatus, members),
            },
          });

          if (result.success) {
            console.log(`Knock alert sent for ${system.display_name}: ${previousStatus} -> ${newStatus}`);
          } else {
            console.error(`Knock alert failed for ${system.display_name}: ${result.error}`);
          }
        }
      }
    }

    console.log(
      `Health check: ${checked} workers polled, ${workerAlerts} worker alerts, ${systemTransitions} system transitions`,
    );
  },
} satisfies ExportedHandler<Env>;
