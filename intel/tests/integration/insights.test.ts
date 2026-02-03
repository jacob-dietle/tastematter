import { describe, test, expect, beforeAll, mock } from "bun:test";
import type { InsightsRequest, InsightsResponse } from "@/types/shared";

/**
 * Integration tests for POST /api/intel/generate-insights
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
        name: "output_insights",
        input: {
          insights: [
            {
              id: "insight-1",
              insight_type: "focus-shift",
              title: "Work shifted to authentication",
              description: "Recent activity shows significant focus on auth modules",
              evidence: ["5 sessions touched auth.ts", "3 new test files for auth"],
              action: {
                label: "View auth files",
                action_type: "filter",
                payload: { files: ["src/auth.ts"] },
              },
            },
          ],
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

describe("POST /api/intel/generate-insights", () => {
  beforeAll(async () => {
    // Import after mocking
    const { createApp } = await import("@/index");
    app = createApp() as unknown as TestApp;
  });

  test("returns 200 with valid request", async () => {
    const request: InsightsRequest = {
      time_range: "7d",
      chain_data: [
        {
          chain_id: "chain-1",
          name: "Auth Refactor",
          session_count: 5,
          file_count: 12,
          recent_activity: "2h ago",
        },
      ],
      file_patterns: [
        {
          file_path: "src/auth.ts",
          access_count: 15,
          co_accessed_with: ["src/login.ts"],
        },
      ],
    };

    const response = await app.handle(
      new Request("http://localhost/api/intel/generate-insights", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(request),
      })
    );

    expect(response.status).toBe(200);
  });

  test("returns valid InsightsResponse structure", async () => {
    const request: InsightsRequest = {
      time_range: "24h",
      chain_data: [],
      file_patterns: [],
    };

    const response = await app.handle(
      new Request("http://localhost/api/intel/generate-insights", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(request),
      })
    );

    const body = (await response.json()) as InsightsResponse;

    expect(Array.isArray(body.insights)).toBe(true);
    expect(body.model_used).toBe("claude-sonnet-4-5-20250929");

    // Check insight structure if present
    if (body.insights.length > 0) {
      const insight = body.insights[0];
      expect(insight.id).toBeDefined();
      expect(insight.insight_type).toBeDefined();
      expect(insight.title).toBeDefined();
      expect(insight.description).toBeDefined();
      expect(Array.isArray(insight.evidence)).toBe(true);
    }
  });

  test("returns 400 for missing time_range", async () => {
    const response = await app.handle(
      new Request("http://localhost/api/intel/generate-insights", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          // time_range missing
          chain_data: [],
          file_patterns: [],
        }),
      })
    );

    expect(response.status).toBe(400);
  });

  test("returns 400 for missing chain_data array", async () => {
    const response = await app.handle(
      new Request("http://localhost/api/intel/generate-insights", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          time_range: "7d",
          // chain_data missing
          file_patterns: [],
        }),
      })
    );

    expect(response.status).toBe(400);
  });

  test("accepts request with empty arrays", async () => {
    const request: InsightsRequest = {
      time_range: "30d",
      chain_data: [],
      file_patterns: [],
    };

    const response = await app.handle(
      new Request("http://localhost/api/intel/generate-insights", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(request),
      })
    );

    expect(response.status).toBe(200);
  });

  test("validates chain_data structure", async () => {
    const response = await app.handle(
      new Request("http://localhost/api/intel/generate-insights", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          time_range: "7d",
          chain_data: [
            {
              chain_id: "chain-1",
              // Missing required fields: name, session_count, file_count, recent_activity
            },
          ],
          file_patterns: [],
        }),
      })
    );

    expect(response.status).toBe(400);
  });

  test("includes X-Correlation-ID in response", async () => {
    const request: InsightsRequest = {
      time_range: "7d",
      chain_data: [],
      file_patterns: [],
    };

    const response = await app.handle(
      new Request("http://localhost/api/intel/generate-insights", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "X-Correlation-ID": "insights-trace-456",
        },
        body: JSON.stringify(request),
      })
    );

    expect(response.headers.get("X-Correlation-ID")).toBe("insights-trace-456");
  });
});
