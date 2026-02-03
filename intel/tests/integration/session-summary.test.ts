import { describe, test, expect, beforeAll, mock } from "bun:test";
import type { SessionSummaryRequest, SessionSummaryResponse } from "@/types/shared";

/**
 * Integration tests for POST /api/intel/summarize-session
 *
 * TDD: These tests drive the implementation
 */

type TestApp = { handle: (req: Request) => Promise<Response> };
let app: TestApp;

// Mock the Anthropic client at module level
const mockCreate = mock(() =>
  Promise.resolve({
    content: [
      {
        type: "tool_use",
        id: "test-id",
        name: "output_session_summary",
        input: {
          summary: "Worked on authentication improvements and testing",
          key_files: ["src/auth.ts", "tests/auth.test.ts"],
          focus_area: "Security",
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

describe("POST /api/intel/summarize-session", () => {
  beforeAll(async () => {
    // Import after mocking
    const { createApp } = await import("@/index");
    app = createApp() as unknown as TestApp;
  });

  test("returns 200 with valid request", async () => {
    const request: SessionSummaryRequest = {
      session_id: "sess-12345",
      files: ["src/auth.ts", "src/login.ts"],
      duration_seconds: 3600,
      chain_id: "chain-auth-work",
    };

    const response = await app.handle(
      new Request("http://localhost/api/intel/summarize-session", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(request),
      })
    );

    expect(response.status).toBe(200);
  });

  test("returns valid SessionSummaryResponse structure", async () => {
    const request: SessionSummaryRequest = {
      session_id: "my-session-id",
      files: ["src/main.ts"],
      duration_seconds: 1800,
      chain_id: null,
    };

    const response = await app.handle(
      new Request("http://localhost/api/intel/summarize-session", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(request),
      })
    );

    const body = (await response.json()) as SessionSummaryResponse;

    expect(body.session_id).toBe("my-session-id");
    expect(body.summary).toBeDefined();
    expect(Array.isArray(body.key_files)).toBe(true);
    // focus_area can be string or null
    expect(body.focus_area === null || typeof body.focus_area === "string").toBe(true);
    expect(body.model_used).toBe("claude-haiku-4-5-20251001");
  });

  test("returns 400 for missing session_id", async () => {
    const response = await app.handle(
      new Request("http://localhost/api/intel/summarize-session", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          // session_id missing
          files: [],
          duration_seconds: null,
          chain_id: null,
        }),
      })
    );

    expect(response.status).toBe(400);
  });

  test("returns 400 for empty session_id", async () => {
    const response = await app.handle(
      new Request("http://localhost/api/intel/summarize-session", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          session_id: "",
          files: [],
          duration_seconds: null,
          chain_id: null,
        }),
      })
    );

    expect(response.status).toBe(400);
  });

  test("accepts request with null optional fields", async () => {
    const request: SessionSummaryRequest = {
      session_id: "sess-minimal",
      files: [],
      duration_seconds: null,
      chain_id: null,
    };

    const response = await app.handle(
      new Request("http://localhost/api/intel/summarize-session", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(request),
      })
    );

    expect(response.status).toBe(200);
  });

  test("includes X-Correlation-ID in response", async () => {
    const request: SessionSummaryRequest = {
      session_id: "test",
      files: [],
      duration_seconds: null,
      chain_id: null,
    };

    const response = await app.handle(
      new Request("http://localhost/api/intel/summarize-session", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "X-Correlation-ID": "trace-abc-123",
        },
        body: JSON.stringify(request),
      })
    );

    expect(response.headers.get("X-Correlation-ID")).toBe("trace-abc-123");
  });
});
