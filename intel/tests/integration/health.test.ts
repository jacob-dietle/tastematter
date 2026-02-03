import { describe, test, expect, beforeAll, afterAll } from "bun:test";
import type { HealthResponse } from "@/types/shared";

// RED tests - Integration tests for health endpoint
// TDD: Write tests first, then implementation

describe("Health Endpoint", () => {
  // Use ReturnType to get correct Elysia type inference
  let app: ReturnType<typeof import("@/index").createApp>;

  beforeAll(async () => {
    const { createApp } = await import("@/index");
    app = createApp();
  });

  test("GET /api/intel/health returns ok status", async () => {
    const response = await app.handle(
      new Request("http://localhost/api/intel/health")
    );

    expect(response.status).toBe(200);

    const body = (await response.json()) as HealthResponse;
    expect(body.status).toBe("ok");
  });

  test("GET /api/intel/health returns version", async () => {
    const response = await app.handle(
      new Request("http://localhost/api/intel/health")
    );

    const body = (await response.json()) as HealthResponse;
    expect(body.version).toBe("0.1.0");
  });

  test("GET /api/intel/health includes correlation ID header", async () => {
    const response = await app.handle(
      new Request("http://localhost/api/intel/health")
    );

    const correlationId = response.headers.get("X-Correlation-ID");
    expect(correlationId).toBeTruthy();
    expect(correlationId).toMatch(/^[a-f0-9-]{36}$/);
  });

  test("GET /api/intel/health propagates provided correlation ID", async () => {
    const providedId = "health-check-123";
    const response = await app.handle(
      new Request("http://localhost/api/intel/health", {
        headers: { "X-Correlation-ID": providedId },
      })
    );

    const correlationId = response.headers.get("X-Correlation-ID");
    expect(correlationId).toBe(providedId);
  });

  test("GET /api/intel/health returns valid JSON content type", async () => {
    const response = await app.handle(
      new Request("http://localhost/api/intel/health")
    );

    const contentType = response.headers.get("Content-Type");
    expect(contentType).toContain("application/json");
  });
});

describe("404 Handling", () => {
  let app: ReturnType<typeof import("@/index").createApp>;

  beforeAll(async () => {
    const { createApp } = await import("@/index");
    app = createApp();
  });

  test("unknown routes return 404", async () => {
    const response = await app.handle(
      new Request("http://localhost/api/intel/unknown-route")
    );

    expect(response.status).toBe(404);
  });
});
