import { describe, it, expect, vi } from "vitest";
import type { Env } from "../src/types.js";

// Mock health-checker before importing worker
vi.mock("../src/health-checker.js", () => ({
  checkWorkerHealth: vi.fn(),
  shouldAlert: vi.fn(() => false),
  computeSystemHealth: vi.fn(() => "healthy"),
}));

// Mock knock before importing worker
vi.mock("../src/knock.js", () => ({
  triggerSystemAlert: vi.fn(),
  buildAlertSummary: vi.fn(() => ""),
}));

import worker from "../src/index.js";

function mockD1() {
  const results: any[] = [];

  const stmt = {
    bind: (..._args: any[]) => stmt,
    all: async <T>() => ({ results: results as T[] }),
    first: async <T>() => (results[0] as T) ?? null,
    run: async () => ({ success: true }),
  };

  const d1 = {
    prepare: vi.fn(() => stmt),
    _setResults: (r: any[]) => {
      results.length = 0;
      results.push(...r);
    },
  };

  return d1 as unknown as D1Database & { _setResults: (r: any[]) => void };
}

function makeEnv(overrides?: Partial<Env>): Env {
  return {
    DB: mockD1() as unknown as D1Database,
    OWNER_ID: "founder",
    KNOCK_API_KEY: "sk_test_key",
    KNOCK_WORKFLOW_KEY: "system-health-alert",
    CF_ACCESS_CLIENT_ID: "test-id",
    CF_ACCESS_CLIENT_SECRET: "test-secret",
    ...overrides,
  };
}

function makeRequest(path: string, method = "GET"): Request {
  return new Request(`https://worker.test${path}`, { method });
}

describe("GET /status", () => {
  it("returns valid WorkerStatusResponse shape", async () => {
    const env = makeEnv();
    const ctx = { waitUntil: vi.fn(), passThroughOnException: vi.fn() } as unknown as ExecutionContext;
    const resp = await worker.fetch(makeRequest("/status"), env, ctx);

    expect(resp.status).toBe(200);
    const body = (await resp.json()) as any;

    // identity
    expect(body.identity).toBeDefined();
    expect(body.identity.worker).toBe("tastematter-control-plane");
    expect(body.identity.display_name).toBe("Control Plane");
    expect(body.identity.system_id).toBe("tastematter-platform");

    // vitals
    expect(body.vitals).toBeDefined();
    expect(body.vitals.status).toBe("ok");
    expect(body.vitals.features.health_polling).toBe(true);
    expect(body.vitals.features.system_grouping).toBe(true);
    expect(body.vitals.features.sync_tracking).toBe(true);

    // schedule
    expect(body.schedule).toBeDefined();
    expect(body.schedule.cron).toBe("0 * * * *");
  });

  it("d1_health reflects worker count", async () => {
    const d1 = mockD1();
    d1._setResults([
      { id: "worker-1", display_name: "Worker 1", enabled: 1 },
      { id: "worker-2", display_name: "Worker 2", enabled: 1 },
      { id: "worker-3", display_name: "Worker 3", enabled: 1 },
    ]);
    const env = makeEnv({ DB: d1 as unknown as D1Database });
    const ctx = { waitUntil: vi.fn(), passThroughOnException: vi.fn() } as unknown as ExecutionContext;
    const resp = await worker.fetch(makeRequest("/status"), env, ctx);

    expect(resp.status).toBe(200);
    const body = (await resp.json()) as any;

    expect(body.d1_health).toBeDefined();
    expect(body.d1_health.total_executions).toBe(3);
    expect(body.d1_health.total_failures).toBe(0);
    expect(body.d1_health.failure_rate).toBe("0%");
  });

  it("GET /health still works (backwards compatibility)", async () => {
    const env = makeEnv();
    const ctx = { waitUntil: vi.fn(), passThroughOnException: vi.fn() } as unknown as ExecutionContext;
    const resp = await worker.fetch(makeRequest("/health"), env, ctx);

    expect(resp.status).toBe(200);
    const body = (await resp.json()) as any;
    expect(body.status).toBe("ok");
    expect(body.worker).toBe("tastematter-control-plane");
  });
});
