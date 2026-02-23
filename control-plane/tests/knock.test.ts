import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { triggerSystemAlert, buildAlertSummary } from "../src/knock.js";
import type { SystemRegistryRow, WorkerWithStatus } from "../src/types.js";

// --- Helpers ---

function makeSystem(overrides: Partial<SystemRegistryRow> = {}): SystemRegistryRow {
  return {
    id: "intel-pipeline",
    display_name: "Intel Pipeline",
    description: null,
    health_rule: "all",
    current_status: "healthy",
    status_changed_at: null,
    created_at: "2026-02-19T00:00:00Z",
    ...overrides,
  };
}

function makeWorkerWithStatus(overrides: Partial<WorkerWithStatus> = {}): WorkerWithStatus {
  return {
    id: "test-worker",
    display_name: "Test Worker",
    health_url: "https://test.workers.dev/health",
    expected_cadence: "4h",
    max_silence_hours: 24,
    auth_type: "none",
    tags: null,
    enabled: 1,
    system_id: "intel-pipeline",
    account_id: null,
    status_url: null,
    created_at: "2026-02-19T00:00:00Z",
    updated_at: "2026-02-19T00:00:00Z",
    current_status: "healthy",
    last_checked: null,
    last_activity: null,
    last_response_time_ms: null,
    error_message: null,
    raw_response: null,
    ...overrides,
  };
}

// --- Unit Tests: triggerSystemAlert ---

describe("triggerSystemAlert", () => {
  const originalFetch = globalThis.fetch;

  afterEach(() => {
    globalThis.fetch = originalFetch;
  });

  const payload = {
    recipients: ["owner-123"],
    data: {
      system_id: "intel-pipeline",
      system_name: "Intel Pipeline",
      previous_status: "healthy",
      current_status: "broken",
      changed_at: "2026-02-20T00:00:00Z",
      affected_workers: [{ name: "Alert Worker", status: "down", error: "Connection refused" }],
      summary: "Intel Pipeline is BROKEN. Alert Worker down.",
    },
  };

  it("returns workflow_run_id on success", async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({ workflow_run_id: "wfr_abc123" }),
    });

    const result = await triggerSystemAlert("sk-knock-test", "system-health-alert", payload);

    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.workflow_run_id).toBe("wfr_abc123");
    }

    expect(globalThis.fetch).toHaveBeenCalledWith(
      "https://api.knock.app/v1/workflows/system-health-alert/trigger",
      expect.objectContaining({
        method: "POST",
        headers: expect.objectContaining({
          Authorization: "Bearer sk-knock-test",
          "Content-Type": "application/json",
        }),
      }),
    );
  });

  it("returns error on HTTP failure", async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: false,
      status: 500,
      text: async () => "Internal Server Error",
    });

    const result = await triggerSystemAlert("sk-knock-test", "system-health-alert", payload);

    expect(result.success).toBe(false);
    if (!result.success) {
      expect(result.error).toBe("Knock API 500: Internal Server Error");
    }
  });

  it("returns error on network failure", async () => {
    globalThis.fetch = vi.fn().mockRejectedValue(new Error("Network unreachable"));

    const result = await triggerSystemAlert("sk-knock-test", "system-health-alert", payload);

    expect(result.success).toBe(false);
    if (!result.success) {
      expect(result.error).toContain("Network unreachable");
    }
  });

  it("handles missing workflow_run_id in response", async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({}),
    });

    const result = await triggerSystemAlert("sk-knock-test", "system-health-alert", payload);

    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.workflow_run_id).toBe("unknown");
    }
  });
});

// --- Unit Tests: buildAlertSummary ---

describe("buildAlertSummary", () => {
  it("builds broken summary with down worker names", () => {
    const system = makeSystem({ display_name: "Intel Pipeline" });
    const members = [
      makeWorkerWithStatus({ display_name: "Alert Worker", current_status: "down" }),
      makeWorkerWithStatus({ display_name: "Newsletter Worker", current_status: "healthy" }),
      makeWorkerWithStatus({ display_name: "Transcript Worker", current_status: "timeout" }),
    ];

    const summary = buildAlertSummary(system, "broken", members);

    expect(summary).toBe("Intel Pipeline is BROKEN. Alert Worker, Transcript Worker down.");
  });

  it("builds recovery summary", () => {
    const system = makeSystem({ display_name: "Intel Pipeline" });
    const members = [
      makeWorkerWithStatus({ display_name: "Alert Worker", current_status: "healthy" }),
    ];

    const summary = buildAlertSummary(system, "healthy", members);

    expect(summary).toBe("Intel Pipeline recovered to healthy.");
  });

  it("builds degraded summary as recovery", () => {
    const system = makeSystem({ display_name: "Platform" });
    const members = [
      makeWorkerWithStatus({ display_name: "Worker A", current_status: "stale" }),
    ];

    const summary = buildAlertSummary(system, "degraded", members);

    expect(summary).toBe("Platform recovered to degraded.");
  });

  it("handles broken with no down workers gracefully", () => {
    const system = makeSystem({ display_name: "Platform" });
    const members: WorkerWithStatus[] = [];

    const summary = buildAlertSummary(system, "broken", members);

    expect(summary).toBe("Platform is BROKEN.  down.");
  });
});

// --- Cron Behavior Tests ---

describe("cron system alerting behavior", () => {
  const originalFetch = globalThis.fetch;
  let fetchCalls: Array<{ url: string; body: any }>;

  beforeEach(() => {
    fetchCalls = [];
    globalThis.fetch = vi.fn(async (url: string | URL | Request, init?: RequestInit) => {
      const urlStr = typeof url === "string" ? url : url instanceof URL ? url.toString() : url.url;
      if (urlStr.includes("api.knock.app")) {
        fetchCalls.push({ url: urlStr, body: JSON.parse(init?.body as string) });
        return { ok: true, json: async () => ({ workflow_run_id: "wfr_test" }) } as Response;
      }
      // Worker health check response
      return {
        ok: true,
        status: 200,
        text: async () => JSON.stringify({ status: "ok", worker: "test" }),
      } as Response;
    });
  });

  afterEach(() => {
    globalThis.fetch = originalFetch;
  });

  // Simulate the cron logic in isolation — this tests the alerting decision logic
  // rather than the full cron handler (which requires D1 mocking)

  async function simulateCronAlertDecision(
    previousSystemStatus: string | null,
    newSystemStatus: string,
    system: SystemRegistryRow,
    members: WorkerWithStatus[],
    env: { KNOCK_API_KEY: string; KNOCK_WORKFLOW_KEY: string; OWNER_ID: string },
  ) {
    if (newSystemStatus !== previousSystemStatus) {
      if (previousSystemStatus !== null && previousSystemStatus !== "unknown") {
        const { triggerSystemAlert, buildAlertSummary } = await import("../src/knock.js");
        await triggerSystemAlert(env.KNOCK_API_KEY, env.KNOCK_WORKFLOW_KEY, {
          recipients: [env.OWNER_ID],
          data: {
            system_id: system.id,
            system_name: system.display_name,
            previous_status: previousSystemStatus,
            current_status: newSystemStatus,
            changed_at: new Date().toISOString(),
            affected_workers: members.map((m) => ({
              name: m.display_name,
              status: m.current_status ?? "unknown",
              error: m.error_message ?? undefined,
            })),
            summary: buildAlertSummary(system, newSystemStatus as any, members),
          },
        });
      }
    }
  }

  const env = { KNOCK_API_KEY: "sk-test", KNOCK_WORKFLOW_KEY: "system-health-alert", OWNER_ID: "owner-1" };
  const system = makeSystem();
  const members = [makeWorkerWithStatus({ current_status: "down" })];

  it("fires Knock on transition healthy -> broken", async () => {
    await simulateCronAlertDecision("healthy", "broken", system, members, env);
    expect(fetchCalls).toHaveLength(1);
    expect(fetchCalls[0].url).toContain("api.knock.app");
    expect(fetchCalls[0].body.data.previous_status).toBe("healthy");
    expect(fetchCalls[0].body.data.current_status).toBe("broken");
  });

  it("skips Knock on continued failure (broken -> broken)", async () => {
    await simulateCronAlertDecision("broken", "broken", system, members, env);
    expect(fetchCalls).toHaveLength(0);
  });

  it("fires Knock on recovery broken -> healthy", async () => {
    const healthyMembers = [makeWorkerWithStatus({ current_status: "healthy" })];
    await simulateCronAlertDecision("broken", "healthy", system, healthyMembers, env);
    expect(fetchCalls).toHaveLength(1);
    expect(fetchCalls[0].body.data.current_status).toBe("healthy");
    expect(fetchCalls[0].body.data.summary).toContain("recovered");
  });

  it("skips Knock on first check (null previous status)", async () => {
    await simulateCronAlertDecision(null, "healthy", system, members, env);
    expect(fetchCalls).toHaveLength(0);
  });

  it("skips Knock when previous status is unknown", async () => {
    await simulateCronAlertDecision("unknown", "broken", system, members, env);
    expect(fetchCalls).toHaveLength(0);
  });
});
