import { describe, it, expect, vi, beforeEach } from "vitest";
import type { Env } from "../src/types.js";
import { createMockD1 } from "./helpers.js";

// We import the worker default export
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

describe("fetch handler", () => {
  it("GET /health returns 200 with status ok", async () => {
    const env = makeEnv();
    const resp = await worker.fetch(makeRequest("/health"), env);

    expect(resp.status).toBe(200);
    const body = (await resp.json()) as any;
    expect(body.status).toBe("ok");
    expect(body.worker).toBe("tastematter-alert-worker");
  });

  it("GET /alert/history returns alert history", async () => {
    const mockRow = {
      id: 1,
      engagement_id: "pixee",
      rule_name: "new-intel",
      trigger_type: "content_change",
      fired_at: "2026-01-01",
      knock_workflow_run_id: null,
      payload: null,
      success: 1,
      error_message: null,
    };
    const mock = createMockD1({ allResults: [mockRow] });
    const env = makeEnv({ ALERTS_DB: mock as unknown as D1Database });

    const resp = await worker.fetch(
      makeRequest("/alert/history?engagement_id=pixee"),
      env
    );

    expect(resp.status).toBe(200);
    const body = (await resp.json()) as any;
    expect(body.data).toHaveLength(1);
    expect(body.data[0].engagement_id).toBe("pixee");
  });

  it("POST /alert/trigger calls processAlertRules", async () => {
    // Mock with no engagements so it runs quickly
    const mock = createMockD1({ allResults: [] });
    const env = makeEnv({ ALERTS_DB: mock as unknown as D1Database });

    const resp = await worker.fetch(
      makeRequest("/alert/trigger", "POST"),
      env
    );

    expect(resp.status).toBe(200);
    const body = (await resp.json()) as any;
    expect(body.data).toBeDefined();
    expect(body.data.checked).toBe(0);
    expect(body.data.fired).toBe(0);
  });

  it("unknown route returns 404", async () => {
    const env = makeEnv();
    const resp = await worker.fetch(makeRequest("/unknown"), env);

    expect(resp.status).toBe(404);
    const body = (await resp.json()) as any;
    expect(body.error).toBe("Not found");
  });

  it("GET /alert/history with limit param", async () => {
    const mock = createMockD1({ allResults: [] });
    const env = makeEnv({ ALERTS_DB: mock as unknown as D1Database });

    const resp = await worker.fetch(
      makeRequest("/alert/history?limit=5"),
      env
    );

    expect(resp.status).toBe(200);
    const body = (await resp.json()) as any;
    expect(body.data).toEqual([]);
  });
});

describe("scheduled handler", () => {
  it("calls processAlertRules via waitUntil", async () => {
    const mock = createMockD1({ allResults: [] });
    const env = makeEnv({ ALERTS_DB: mock as unknown as D1Database });

    const waitUntilPromises: Promise<unknown>[] = [];
    const ctx = {
      waitUntil: (p: Promise<unknown>) => {
        waitUntilPromises.push(p);
      },
      passThroughOnException: () => {},
    } as unknown as ExecutionContext;

    const event = {
      scheduledTime: Date.now(),
      cron: "0 */4 * * *",
    } as ScheduledEvent;

    // Should not throw
    worker.scheduled(event, env, ctx);

    // Wait for all promises to settle
    await Promise.all(waitUntilPromises);
  });
});
