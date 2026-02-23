import { describe, it, expect } from "vitest";
import type { Env } from "../src/types.js";
import { createMockD1 } from "./helpers.js";
import worker from "../src/index.js";

function makeEnv(overrides?: Partial<Env>): Env {
  return {
    ALERTS_DB: createMockD1() as unknown as D1Database,
    KNOCK_API_KEY: "sk_test_key",
    OWNER_ID: "founder",
    ...overrides,
  };
}

function makeRequest(path: string, method = "GET"): Request {
  return new Request(`https://worker.test${path}`, { method });
}

describe("GET /status", () => {
  it("returns valid WorkerStatusResponse shape", async () => {
    const env = makeEnv();
    const resp = await worker.fetch(makeRequest("/status"), env);

    expect(resp.status).toBe(200);
    const body = (await resp.json()) as any;

    // identity
    expect(body.identity).toBeDefined();
    expect(body.identity.worker).toBe("tastematter-alert-worker");
    expect(body.identity.display_name).toBe("Tastematter Alert Worker");
    expect(body.identity.system_id).toBe("tastematter-platform");
    expect(body.identity.account_id).toBe("4c8353a21e0bfc69a1e036e223cba4d8");

    // vitals
    expect(body.vitals).toBeDefined();
    expect(body.vitals.status).toBe("ok");
    expect(body.vitals.features).toBeDefined();
    expect(body.vitals.features.alerting).toBe(true);

    // schedule
    expect(body.schedule).toBeDefined();
    expect(body.schedule.cron).toBe("0 */4 * * *");
  });

  it("includes d1_health with correct counts", async () => {
    const alertRows = [
      {
        id: 1, engagement_id: "pixee", rule_name: "new-intel",
        trigger_type: "content_change", fired_at: "2026-02-20T12:00:00Z",
        knock_workflow_run_id: null, payload: null, success: 1, error_message: null,
      },
      {
        id: 2, engagement_id: "pixee", rule_name: "new-intel",
        trigger_type: "content_change", fired_at: "2026-02-20T08:00:00Z",
        knock_workflow_run_id: null, payload: null, success: 0, error_message: "Knock timeout",
      },
    ];
    const mock = createMockD1({ allResults: alertRows });
    const env = makeEnv({ ALERTS_DB: mock as unknown as D1Database });

    const resp = await worker.fetch(makeRequest("/status"), env);
    expect(resp.status).toBe(200);
    const body = (await resp.json()) as any;

    expect(body.d1_health).toBeDefined();
    expect(body.d1_health.total_executions).toBe(2);
    expect(body.d1_health.total_failures).toBe(1);
    expect(body.d1_health.failure_rate).toBe("50.0%");
    expect(body.d1_health.last_execution).toBeDefined();
    expect(body.d1_health.last_execution.status).toBe("completed");
    expect(body.d1_health.last_failure).toBeDefined();
    expect(body.d1_health.last_failure.error).toBe("Knock timeout");
  });

  it("includes trail from last alert", async () => {
    const alertRows = [
      {
        id: 1, engagement_id: "pixee", rule_name: "new-intel",
        trigger_type: "content_change", fired_at: "2026-02-20T12:00:00Z",
        knock_workflow_run_id: "run_abc", payload: null, success: 1, error_message: null,
      },
    ];
    const mock = createMockD1({ allResults: alertRows });
    const env = makeEnv({ ALERTS_DB: mock as unknown as D1Database });

    const resp = await worker.fetch(makeRequest("/status"), env);
    expect(resp.status).toBe(200);
    const body = (await resp.json()) as any;

    expect(body.trail).toBeDefined();
    expect(body.trail.last_deposit).toBe("alert_fired: new-intel");
    expect(body.trail.at).toBe("2026-02-20T12:00:00Z");
    expect(body.trail.type).toBe("content_change");
    expect(body.trail.detail).toContain("pixee");
  });

  it("handles missing ContextDO gracefully (no corpus)", async () => {
    const env = makeEnv();
    // No CONTEXT_DO in env (undefined by default in test)
    const resp = await worker.fetch(makeRequest("/status"), env);
    expect(resp.status).toBe(200);
    const body = (await resp.json()) as any;

    // corpus should be absent/undefined (serialized as not present)
    expect(body.corpus).toBeUndefined();
    expect(body.vitals.features.publishing).toBe(false);
  });

  it("handles empty alert history (no trail, zero d1_health)", async () => {
    const mock = createMockD1({ allResults: [] });
    const env = makeEnv({ ALERTS_DB: mock as unknown as D1Database });

    const resp = await worker.fetch(makeRequest("/status"), env);
    expect(resp.status).toBe(200);
    const body = (await resp.json()) as any;

    expect(body.trail).toBeUndefined();
    expect(body.d1_health).toBeDefined();
    expect(body.d1_health.total_executions).toBe(0);
    expect(body.d1_health.total_failures).toBe(0);
    expect(body.d1_health.failure_rate).toBe("0%");
  });

  it("GET /health still works (backwards compatibility)", async () => {
    const env = makeEnv();
    const resp = await worker.fetch(makeRequest("/health"), env);

    expect(resp.status).toBe(200);
    const body = (await resp.json()) as any;
    expect(body.status).toBe("ok");
    expect(body.worker).toBe("tastematter-alert-worker");
  });
});
