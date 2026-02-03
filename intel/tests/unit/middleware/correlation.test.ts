import { describe, test, expect, beforeEach } from "bun:test";
import { Elysia } from "elysia";

// RED tests - these will fail until we implement the middleware
// TDD: Write tests first, then implementation

// Helper type for test app - avoids complex Elysia generic inference
type TestApp = { handle: (req: Request) => Promise<Response> };

describe("Correlation ID Middleware", () => {
  let app: TestApp;

  beforeEach(async () => {
    const { correlationMiddleware } = await import("@/middleware/correlation");

    app = new Elysia()
      .use(correlationMiddleware())
      .get("/test", ({ correlationId }) => ({
        correlation_id: correlationId,
      }));
  });

  test("generates correlation ID if not provided", async () => {
    const response = await app.handle(
      new Request("http://localhost/test")
    );

    // Response should have X-Correlation-ID header
    const correlationId = response.headers.get("X-Correlation-ID");
    expect(correlationId).toBeTruthy();
    expect(correlationId).toMatch(/^[a-f0-9-]{36}$/); // UUID format
  });

  test("uses provided X-Correlation-ID header", async () => {
    const providedId = "test-correlation-123";
    const response = await app.handle(
      new Request("http://localhost/test", {
        headers: { "X-Correlation-ID": providedId },
      })
    );

    const correlationId = response.headers.get("X-Correlation-ID");
    expect(correlationId).toBe(providedId);
  });

  test("exposes correlation ID in store for handlers", async () => {
    const providedId = "store-test-456";
    const response = await app.handle(
      new Request("http://localhost/test", {
        headers: { "X-Correlation-ID": providedId },
      })
    );

    const body = (await response.json()) as { correlation_id: string };
    expect(body.correlation_id).toBe(providedId);
  });

  test("uses lowercase header name for lookup (case insensitive)", async () => {
    const providedId = "case-test-789";
    const response = await app.handle(
      new Request("http://localhost/test", {
        headers: { "x-correlation-id": providedId },
      })
    );

    const correlationId = response.headers.get("X-Correlation-ID");
    expect(correlationId).toBe(providedId);
  });

  test("generates different IDs for different requests", async () => {
    const response1 = await app.handle(
      new Request("http://localhost/test")
    );
    const response2 = await app.handle(
      new Request("http://localhost/test")
    );

    const id1 = response1.headers.get("X-Correlation-ID");
    const id2 = response2.headers.get("X-Correlation-ID");

    expect(id1).toBeTruthy();
    expect(id2).toBeTruthy();
    expect(id1).not.toBe(id2);
  });
});

describe("getCorrelationId helper", () => {
  test("extracts correlation ID from store", async () => {
    const { getCorrelationId } = await import("@/middleware/correlation");

    const mockStore = { correlationId: "helper-test-id" };
    expect(getCorrelationId(mockStore)).toBe("helper-test-id");
  });

  test("returns undefined if store has no correlation ID", async () => {
    const { getCorrelationId } = await import("@/middleware/correlation");

    const mockStore = {};
    expect(getCorrelationId(mockStore)).toBeUndefined();
  });
});
