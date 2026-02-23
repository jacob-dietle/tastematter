import { describe, it, expect } from "vitest";
import { evaluateStaleness, shouldAlert, computeSystemHealth, parseStatusResponse } from "../src/health-checker.js";
import type { WorkerRegistryRow, WorkerHealthResponse, WorkerStatus } from "../src/types.js";

function makeWorker(overrides: Partial<WorkerRegistryRow> = {}): WorkerRegistryRow {
  return {
    id: "test-worker",
    display_name: "Test Worker",
    health_url: "https://test.workers.dev/health",
    expected_cadence: "4h",
    max_silence_hours: 24,
    auth_type: "none",
    tags: null,
    enabled: 1,
    system_id: null,
    account_id: null,
    status_url: null,
    created_at: "2026-02-19T00:00:00Z",
    updated_at: "2026-02-19T00:00:00Z",
    ...overrides,
  };
}

describe("evaluateStaleness", () => {
  it("returns 'reachable' when no last_activity", () => {
    const body: WorkerHealthResponse = { status: "ok" };
    expect(evaluateStaleness(body, makeWorker())).toBe("reachable");
  });

  it("returns 'reachable' for invalid date", () => {
    const body: WorkerHealthResponse = { status: "ok", last_activity: "not-a-date" };
    expect(evaluateStaleness(body, makeWorker())).toBe("reachable");
  });

  it("returns 'healthy' when activity is recent", () => {
    const recent = new Date(Date.now() - 1000 * 60 * 30).toISOString();
    const body: WorkerHealthResponse = { status: "ok", last_activity: recent };
    expect(evaluateStaleness(body, makeWorker({ max_silence_hours: 24 }))).toBe("healthy");
  });

  it("returns 'stale' when activity exceeds max_silence_hours", () => {
    const old = new Date(Date.now() - 1000 * 60 * 60 * 50).toISOString();
    const body: WorkerHealthResponse = { status: "ok", last_activity: old };
    expect(evaluateStaleness(body, makeWorker({ max_silence_hours: 24 }))).toBe("stale");
  });

  it("respects custom max_silence_hours", () => {
    const old = new Date(Date.now() - 1000 * 60 * 60 * 5).toISOString();
    const body: WorkerHealthResponse = { status: "ok", last_activity: old };
    expect(evaluateStaleness(body, makeWorker({ max_silence_hours: 4 }))).toBe("stale");
    expect(evaluateStaleness(body, makeWorker({ max_silence_hours: 8 }))).toBe("healthy");
  });
});

describe("shouldAlert", () => {
  it("does not alert on first check (no previous)", () => {
    expect(shouldAlert("down", null)).toBe(false);
  });

  it("does not alert when status is healthy", () => {
    expect(shouldAlert("healthy", "healthy")).toBe(false);
  });

  it("does not alert when status is reachable", () => {
    expect(shouldAlert("reachable", "healthy")).toBe(false);
  });

  it("alerts when transitioning from healthy to down", () => {
    expect(shouldAlert("down", "healthy")).toBe(true);
  });

  it("alerts when transitioning from healthy to stale", () => {
    expect(shouldAlert("stale", "healthy")).toBe(true);
  });

  it("alerts when transitioning from reachable to timeout", () => {
    expect(shouldAlert("timeout", "reachable")).toBe(true);
  });

  it("does not alert on continued failure (down to down)", () => {
    expect(shouldAlert("down", "down")).toBe(false);
  });

  it("does not alert on continued stale (stale to stale)", () => {
    expect(shouldAlert("stale", "stale")).toBe(false);
  });

  it("alerts when transitioning from unknown to down", () => {
    expect(shouldAlert("down", "unknown")).toBe(true);
  });
});

describe("parseStatusResponse", () => {
  it("parses valid /status response", () => {
    const raw = JSON.stringify({
      identity: { worker: "test", display_name: "Test" },
      vitals: { status: "ok" },
    });
    const result = parseStatusResponse(raw);
    expect(result).not.toBeNull();
    expect(result!.identity.worker).toBe("test");
    expect(result!.vitals.status).toBe("ok");
  });

  it("parses full /status response with all fields", () => {
    const raw = JSON.stringify({
      identity: { worker: "alert-worker", display_name: "Alert Worker", system_id: "platform" },
      vitals: { status: "ok", features: { alerting: true } },
      corpus: { commit: "abc123", file_count: 34, loaded_at: "2026-02-20T00:00:00Z" },
      trail: { last_deposit: "alert_fired", at: "2026-02-20T14:00:00Z", type: "content_change" },
      d1_health: { total_executions: 84, total_failures: 0, failure_rate: "0%" },
      schedule: { cron: "0 */4 * * *" },
    });
    const result = parseStatusResponse(raw);
    expect(result).not.toBeNull();
    expect(result!.corpus!.commit).toBe("abc123");
    expect(result!.trail!.type).toBe("content_change");
    expect(result!.d1_health!.total_executions).toBe(84);
  });

  it("returns null for missing identity", () => {
    const raw = JSON.stringify({ vitals: { status: "ok" } });
    expect(parseStatusResponse(raw)).toBeNull();
  });

  it("returns null for missing vitals", () => {
    const raw = JSON.stringify({ identity: { worker: "test" } });
    expect(parseStatusResponse(raw)).toBeNull();
  });

  it("returns null for invalid JSON", () => {
    expect(parseStatusResponse("not json")).toBeNull();
  });

  it("returns null for old /health format", () => {
    const raw = JSON.stringify({ status: "ok", worker: "test" });
    expect(parseStatusResponse(raw)).toBeNull();
  });
});

describe("computeSystemHealth", () => {
  it("returns 'healthy' when all members healthy (rule: all)", () => {
    expect(computeSystemHealth("all", ["healthy", "healthy"])).toBe("healthy");
  });

  it("returns 'healthy' when all members reachable (rule: all)", () => {
    expect(computeSystemHealth("all", ["reachable", "reachable"])).toBe("healthy");
  });

  it("returns 'broken' when any member down (rule: all)", () => {
    expect(computeSystemHealth("all", ["healthy", "down"])).toBe("broken");
  });

  it("returns 'broken' when any member timeout (rule: all)", () => {
    expect(computeSystemHealth("all", ["healthy", "timeout"])).toBe("broken");
  });

  it("returns 'degraded' when member stale but not down (rule: all)", () => {
    expect(computeSystemHealth("all", ["healthy", "stale"])).toBe("degraded");
  });

  it("returns 'healthy' when at least one healthy (rule: any)", () => {
    expect(computeSystemHealth("any", ["healthy", "down"])).toBe("healthy");
  });

  it("returns 'broken' when all down (rule: any)", () => {
    expect(computeSystemHealth("any", ["down", "timeout"])).toBe("broken");
  });

  it("returns 'unknown' when no members", () => {
    expect(computeSystemHealth("all", [])).toBe("unknown");
  });

  it("handles mixed statuses (rule: all)", () => {
    expect(computeSystemHealth("all", ["healthy", "degraded"])).toBe("degraded");
  });
});
