import type {
  Env, HealthCheckResult, WorkerRegistryRow,
  WorkerHealthResponse, WorkerStatusResponse, WorkerStatus, SystemStatus,
} from "./types.js";

const HEALTH_TIMEOUT_MS = 5000;

export async function checkWorkerHealth(
  worker: WorkerRegistryRow,
  env: Env,
): Promise<HealthCheckResult> {
  const start = Date.now();
  const headers: Record<string, string> = {};

  if (worker.auth_type === "cf-access") {
    headers["CF-Access-Client-Id"] = env.CF_ACCESS_CLIENT_ID;
    headers["CF-Access-Client-Secret"] = env.CF_ACCESS_CLIENT_SECRET;
  }

  const setId = (r: HealthCheckResult) => { r.worker_id = worker.id; return r; };

  // Try /status first, then fall back to /health
  const statusUrl = worker.status_url ?? worker.health_url.replace(/\/health$/, "/status");
  const statusResult = await fetchEndpoint(statusUrl, headers, start);

  if (statusResult.http_status === 200 && statusResult.raw_response) {
    const parsed = parseStatusResponse(statusResult.raw_response);
    if (parsed) {
      return setId(buildResultFromStatus(worker, statusResult, parsed));
    }
    // 200 but not valid /status shape — parse as legacy /health response
    try {
      const body = JSON.parse(statusResult.raw_response) as WorkerHealthResponse;
      const status = evaluateStaleness(body, worker);
      return setId({ ...statusResult, status, last_activity: body.last_activity ?? null, activity_type: body.activity_type ?? null });
    } catch {
      return setId(statusResult);
    }
  }

  // Fallback: /status returned 404 or wasn't parseable — try /health
  if (statusResult.http_status === 404 || statusResult.status === "timeout") {
    const healthResult = await fetchEndpoint(worker.health_url, headers, start);
    if (healthResult.http_status === 200 && healthResult.raw_response) {
      try {
        const body = JSON.parse(healthResult.raw_response) as WorkerHealthResponse;
        const status = evaluateStaleness(body, worker);
        return setId({ ...healthResult, status, last_activity: body.last_activity ?? null, activity_type: body.activity_type ?? null });
      } catch {
        return setId(healthResult);
      }
    }
    return setId(healthResult);
  }

  // /status returned a non-200, non-404 — treat as the health result
  return setId(statusResult);
}

async function fetchEndpoint(
  url: string,
  headers: Record<string, string>,
  start: number,
): Promise<HealthCheckResult> {
  const workerId = ""; // filled by caller via spread
  try {
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), HEALTH_TIMEOUT_MS);

    const resp = await fetch(url, { headers, signal: controller.signal });
    clearTimeout(timeout);
    const responseTime = Date.now() - start;

    if (!resp.ok) {
      return {
        worker_id: workerId,
        http_status: resp.status,
        response_time_ms: responseTime,
        status: resp.status === 404 ? "unknown" as WorkerStatus : "down",
        last_activity: null,
        activity_type: null,
        raw_response: await resp.text().catch(() => null),
        error_message: `HTTP ${resp.status}`,
      };
    }

    const text = await resp.text();
    return {
      worker_id: workerId,
      http_status: resp.status,
      response_time_ms: responseTime,
      status: "reachable",
      last_activity: null,
      activity_type: null,
      raw_response: text,
      error_message: null,
    };
  } catch (err: any) {
    const responseTime = Date.now() - start;
    if (err.name === "AbortError") {
      return {
        worker_id: workerId,
        http_status: null,
        response_time_ms: responseTime,
        status: "timeout",
        last_activity: null,
        activity_type: null,
        raw_response: null,
        error_message: `Timeout after ${HEALTH_TIMEOUT_MS}ms`,
      };
    }
    return {
      worker_id: workerId,
      http_status: null,
      response_time_ms: responseTime,
      status: "down",
      last_activity: null,
      activity_type: null,
      raw_response: null,
      error_message: err.message || String(err),
    };
  }
}

export function parseStatusResponse(raw: string): WorkerStatusResponse | null {
  try {
    const parsed = JSON.parse(raw);
    // Must have identity.worker and vitals.status to be a valid /status response
    if (parsed?.identity?.worker && parsed?.vitals?.status) {
      return parsed as WorkerStatusResponse;
    }
    return null;
  } catch {
    return null;
  }
}

function buildResultFromStatus(
  worker: WorkerRegistryRow,
  fetchResult: HealthCheckResult,
  status: WorkerStatusResponse,
): HealthCheckResult {
  const vitalsStatus = status.vitals.status;
  let workerStatus: WorkerStatus;

  if (vitalsStatus === "error") {
    workerStatus = "down";
  } else if (vitalsStatus === "degraded") {
    workerStatus = "degraded";
  } else {
    // vitals.status === 'ok' — check staleness via trail or corpus
    const lastActivity = status.trail?.at ?? status.corpus?.loaded_at ?? null;
    if (lastActivity) {
      const hoursSince = (Date.now() - new Date(lastActivity).getTime()) / (1000 * 60 * 60);
      workerStatus = hoursSince > worker.max_silence_hours ? "stale" : "healthy";
    } else {
      workerStatus = "reachable";
    }
  }

  return {
    worker_id: worker.id,
    http_status: fetchResult.http_status,
    response_time_ms: fetchResult.response_time_ms,
    status: workerStatus,
    last_activity: status.trail?.at ?? null,
    activity_type: status.trail?.type ?? null,
    raw_response: fetchResult.raw_response,
    error_message: null,
  };
}

export function evaluateStaleness(
  body: WorkerHealthResponse,
  worker: WorkerRegistryRow,
): WorkerStatus {
  if (!body.last_activity) {
    return "reachable";
  }

  const lastActivity = new Date(body.last_activity).getTime();
  if (isNaN(lastActivity)) {
    return "reachable";
  }

  const hoursSince = (Date.now() - lastActivity) / (1000 * 60 * 60);
  if (hoursSince > worker.max_silence_hours) {
    return "stale";
  }

  return "healthy";
}

export function shouldAlert(
  currentStatus: WorkerStatus,
  previousStatus: WorkerStatus | null,
): boolean {
  if (previousStatus === null) return false;
  if (currentStatus === "healthy" || currentStatus === "reachable") return false;
  if (previousStatus === currentStatus) return false;

  const wasHealthy = previousStatus === "healthy" || previousStatus === "reachable" || previousStatus === "unknown";
  return wasHealthy;
}

export function computeSystemHealth(
  rule: string,
  memberStatuses: WorkerStatus[],
): SystemStatus {
  if (memberStatuses.length === 0) return "unknown";

  const isHealthy = (s: WorkerStatus) => s === "healthy" || s === "reachable";
  const isDown = (s: WorkerStatus) => s === "down" || s === "timeout";

  if (rule === "all") {
    if (memberStatuses.every(isHealthy)) return "healthy";
    if (memberStatuses.some(isDown)) return "broken";
    return "degraded";
  }

  // 'any' — at least one healthy means system is healthy
  if (memberStatuses.some(isHealthy)) return "healthy";
  return "broken";
}
