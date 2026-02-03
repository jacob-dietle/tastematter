import { describe, test, expect, beforeAll, mock } from "bun:test";
import type { CommitAnalysisRequest, CommitAnalysisResponse } from "@/types/shared";

/**
 * Integration tests for POST /api/intel/analyze-commit
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
        name: "output_commit_analysis",
        input: {
          is_agent_commit: false,
          summary: "Fixed authentication bug in login flow",
          risk_level: "medium",
          review_focus: "Security validation of token handling",
          related_files: ["src/session.ts"],
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

describe("POST /api/intel/analyze-commit", () => {
  beforeAll(async () => {
    // Import after mocking
    const { createApp } = await import("@/index");
    app = createApp() as unknown as TestApp;
  });

  test("returns 200 with valid request", async () => {
    const request: CommitAnalysisRequest = {
      commit_hash: "abc123def456",
      message: "Fix authentication bug",
      author: "developer@example.com",
      diff: "--- a/src/auth.ts\n+++ b/src/auth.ts\n@@ -10,5 +10,6 @@\n+// Fixed token validation",
      files_changed: ["src/auth.ts"],
    };

    const response = await app.handle(
      new Request("http://localhost/api/intel/analyze-commit", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(request),
      })
    );

    expect(response.status).toBe(200);
  });

  test("returns valid CommitAnalysisResponse structure", async () => {
    const request: CommitAnalysisRequest = {
      commit_hash: "test-commit-hash",
      message: "Refactor user service",
      author: "dev",
      diff: "---",
      files_changed: ["src/user.ts"],
    };

    const response = await app.handle(
      new Request("http://localhost/api/intel/analyze-commit", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(request),
      })
    );

    const body = (await response.json()) as CommitAnalysisResponse;

    expect(body.commit_hash).toBe("test-commit-hash");
    expect(typeof body.is_agent_commit).toBe("boolean");
    expect(body.summary).toBeDefined();
    expect(["low", "medium", "high"]).toContain(body.risk_level);
    expect(body.review_focus).toBeDefined();
    expect(Array.isArray(body.related_files)).toBe(true);
    expect(body.model_used).toBe("claude-sonnet-4-5-20250929");
  });

  test("returns 400 for missing commit_hash", async () => {
    const response = await app.handle(
      new Request("http://localhost/api/intel/analyze-commit", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          // commit_hash missing
          message: "Fix bug",
          author: "dev",
          diff: "---",
          files_changed: [],
        }),
      })
    );

    expect(response.status).toBe(400);
  });

  test("returns 400 for empty commit_hash", async () => {
    const response = await app.handle(
      new Request("http://localhost/api/intel/analyze-commit", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          commit_hash: "",
          message: "Fix bug",
          author: "dev",
          diff: "---",
          files_changed: [],
        }),
      })
    );

    expect(response.status).toBe(400);
  });

  test("returns 400 for missing required fields", async () => {
    const response = await app.handle(
      new Request("http://localhost/api/intel/analyze-commit", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          commit_hash: "abc123",
          // message, author, diff, files_changed missing
        }),
      })
    );

    expect(response.status).toBe(400);
  });

  test("includes X-Correlation-ID in response", async () => {
    const request: CommitAnalysisRequest = {
      commit_hash: "test",
      message: "Test",
      author: "dev",
      diff: "---",
      files_changed: [],
    };

    const response = await app.handle(
      new Request("http://localhost/api/intel/analyze-commit", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "X-Correlation-ID": "my-trace-id-123",
        },
        body: JSON.stringify(request),
      })
    );

    expect(response.headers.get("X-Correlation-ID")).toBe("my-trace-id-123");
  });
});
