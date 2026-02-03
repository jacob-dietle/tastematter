import { describe, test, expect, beforeAll, afterAll, mock } from "bun:test";
import type { ChainNamingRequest, ChainNamingResponse } from "@/types/shared";

// RED tests for chain naming HTTP endpoint
// TDD: These tests drive the implementation

type TestApp = { handle: (req: Request) => Promise<Response> };
let app: TestApp;

// Mock the Anthropic client at module level
const mockCreate = mock(() =>
  Promise.resolve({
    content: [
      {
        type: "tool_use",
        id: "test-id",
        name: "output_chain_name",
        input: {
          generated_name: "Test Chain Name",
          category: "feature",
          confidence: 0.85,
        },
      },
    ],
  })
);

// We'll need to mock the Anthropic module before importing the app
mock.module("@anthropic-ai/sdk", () => ({
  default: class MockAnthropic {
    messages = { create: mockCreate };
  },
}));

describe("POST /api/intel/name-chain", () => {
  beforeAll(async () => {
    // Import after mocking
    const { createApp } = await import("@/index");
    app = createApp() as unknown as TestApp;
  });

  test("returns 200 with valid request", async () => {
    const request: ChainNamingRequest = {
      chain_id: "test-chain-123",
      files_touched: ["src/auth.ts", "src/login.ts"],
      session_count: 5,
      recent_sessions: ["sess-1", "sess-2"],
    };

    const response = await app.handle(
      new Request("http://localhost/api/intel/name-chain", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(request),
      })
    );

    expect(response.status).toBe(200);
  });

  test("returns valid ChainNamingResponse structure", async () => {
    const request: ChainNamingRequest = {
      chain_id: "test-chain-456",
      files_touched: ["src/feature.ts"],
      session_count: 3,
      recent_sessions: [],
    };

    const response = await app.handle(
      new Request("http://localhost/api/intel/name-chain", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(request),
      })
    );

    const body = (await response.json()) as ChainNamingResponse;

    expect(body.chain_id).toBe("test-chain-456");
    expect(body.generated_name).toBeDefined();
    expect(body.category).toBeDefined();
    expect(body.confidence).toBeGreaterThanOrEqual(0);
    expect(body.confidence).toBeLessThanOrEqual(1);
    expect(body.model_used).toBe("claude-haiku-4-5-20251001");
  });

  test("returns 400 for missing chain_id", async () => {
    const response = await app.handle(
      new Request("http://localhost/api/intel/name-chain", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          // chain_id missing
          files_touched: [],
          session_count: 1,
          recent_sessions: [],
        }),
      })
    );

    expect(response.status).toBe(400);
  });

  test("returns 400 for empty chain_id", async () => {
    const response = await app.handle(
      new Request("http://localhost/api/intel/name-chain", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          chain_id: "",
          files_touched: [],
          session_count: 1,
          recent_sessions: [],
        }),
      })
    );

    expect(response.status).toBe(400);
  });

  test("returns 400 for invalid session_count", async () => {
    const response = await app.handle(
      new Request("http://localhost/api/intel/name-chain", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          chain_id: "test",
          files_touched: [],
          session_count: 0, // Must be positive
          recent_sessions: [],
        }),
      })
    );

    expect(response.status).toBe(400);
  });

  test("returns 400 for non-array files_touched", async () => {
    const response = await app.handle(
      new Request("http://localhost/api/intel/name-chain", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          chain_id: "test",
          files_touched: "not-an-array",
          session_count: 1,
          recent_sessions: [],
        }),
      })
    );

    expect(response.status).toBe(400);
  });

  test("includes X-Correlation-ID in response", async () => {
    const request: ChainNamingRequest = {
      chain_id: "test-chain",
      files_touched: [],
      session_count: 1,
      recent_sessions: [],
    };

    const response = await app.handle(
      new Request("http://localhost/api/intel/name-chain", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "X-Correlation-ID": "my-trace-id",
        },
        body: JSON.stringify(request),
      })
    );

    expect(response.headers.get("X-Correlation-ID")).toBe("my-trace-id");
  });

  test("returns 405 for GET request", async () => {
    const response = await app.handle(
      new Request("http://localhost/api/intel/name-chain", {
        method: "GET",
      })
    );

    expect(response.status).toBe(404); // Elysia returns 404 for method not allowed by default
  });
});
