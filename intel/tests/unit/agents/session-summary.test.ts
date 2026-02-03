import { describe, test, expect, mock } from "bun:test";
import type { SessionSummaryRequest } from "@/types/shared";
import { SessionSummaryResponseSchema } from "@/types/shared";

/**
 * RED Tests for Session Summary Agent
 *
 * TDD: Write tests FIRST, then implement agent to make them GREEN
 *
 * Pattern: Follow chain-naming.ts exactly
 * Model: claude-haiku-4-5-20251001 (fast/cheap - summarization is simpler)
 */

describe("Session Summary Agent", () => {
  describe("SESSION_SUMMARY_TOOL definition", () => {
    test("exports SESSION_SUMMARY_TOOL with correct name", async () => {
      const { SESSION_SUMMARY_TOOL } = await import("@/agents/session-summary");
      expect(SESSION_SUMMARY_TOOL.name).toBe("output_session_summary");
    });

    test("SESSION_SUMMARY_TOOL has required properties in schema", async () => {
      const { SESSION_SUMMARY_TOOL } = await import("@/agents/session-summary");
      const schema = SESSION_SUMMARY_TOOL.input_schema as unknown as {
        required: string[];
      };
      expect(schema.required).toContain("summary");
      expect(schema.required).toContain("key_files");
      expect(schema.required).toContain("focus_area");
    });

    test("SESSION_SUMMARY_TOOL has summary as string", async () => {
      const { SESSION_SUMMARY_TOOL } = await import("@/agents/session-summary");
      const schema = SESSION_SUMMARY_TOOL.input_schema as unknown as {
        properties: { summary: { type: string } };
      };
      expect(schema.properties.summary.type).toBe("string");
    });

    test("SESSION_SUMMARY_TOOL has key_files as array", async () => {
      const { SESSION_SUMMARY_TOOL } = await import("@/agents/session-summary");
      const schema = SESSION_SUMMARY_TOOL.input_schema as unknown as {
        properties: { key_files: { type: string } };
      };
      expect(schema.properties.key_files.type).toBe("array");
    });
  });

  describe("buildPrompt function", () => {
    test("buildPrompt includes session_id", async () => {
      const { buildPrompt } = await import("@/agents/session-summary");
      const request: SessionSummaryRequest = {
        session_id: "sess-abc123",
        files: [],
        duration_seconds: 3600,
        chain_id: null,
      };
      const prompt = buildPrompt(request);
      expect(prompt).toContain("sess-abc123");
    });

    test("buildPrompt includes files list", async () => {
      const { buildPrompt } = await import("@/agents/session-summary");
      const request: SessionSummaryRequest = {
        session_id: "sess-123",
        files: ["src/auth.ts", "src/login.ts", "tests/auth.test.ts"],
        duration_seconds: 1800,
        chain_id: null,
      };
      const prompt = buildPrompt(request);
      expect(prompt).toContain("src/auth.ts");
      expect(prompt).toContain("src/login.ts");
      expect(prompt).toContain("tests/auth.test.ts");
    });

    test("buildPrompt includes duration when provided", async () => {
      const { buildPrompt } = await import("@/agents/session-summary");
      const request: SessionSummaryRequest = {
        session_id: "sess-123",
        files: [],
        duration_seconds: 7200,
        chain_id: null,
      };
      const prompt = buildPrompt(request);
      expect(prompt).toContain("7200");
    });

    test("buildPrompt handles null duration", async () => {
      const { buildPrompt } = await import("@/agents/session-summary");
      const request: SessionSummaryRequest = {
        session_id: "sess-123",
        files: [],
        duration_seconds: null,
        chain_id: null,
      };
      const prompt = buildPrompt(request);
      expect(prompt).toContain("unknown");
    });

    test("buildPrompt includes chain_id when provided", async () => {
      const { buildPrompt } = await import("@/agents/session-summary");
      const request: SessionSummaryRequest = {
        session_id: "sess-123",
        files: [],
        duration_seconds: null,
        chain_id: "chain-auth-refactor",
      };
      const prompt = buildPrompt(request);
      expect(prompt).toContain("chain-auth-refactor");
    });

    test("buildPrompt instructs to use the tool", async () => {
      const { buildPrompt } = await import("@/agents/session-summary");
      const request: SessionSummaryRequest = {
        session_id: "sess-123",
        files: [],
        duration_seconds: null,
        chain_id: null,
      };
      const prompt = buildPrompt(request);
      expect(prompt).toContain("output_session_summary");
    });
  });

  describe("summarizeSession function", () => {
    test("summarizeSession returns valid SessionSummaryResponse", async () => {
      const { summarizeSession } = await import("@/agents/session-summary");

      const mockClient = {
        messages: {
          create: mock(() =>
            Promise.resolve({
              content: [
                {
                  type: "tool_use",
                  id: "test-id",
                  name: "output_session_summary",
                  input: {
                    summary: "Worked on authentication refactoring",
                    key_files: ["src/auth.ts", "src/login.ts"],
                    focus_area: "Security",
                  },
                },
              ],
            })
          ),
        },
      };

      const request: SessionSummaryRequest = {
        session_id: "sess-123",
        files: ["src/auth.ts", "src/login.ts"],
        duration_seconds: 3600,
        chain_id: null,
      };

      const result = await summarizeSession(mockClient as never, request);
      const validation = SessionSummaryResponseSchema.safeParse(result);
      expect(validation.success).toBe(true);
    });

    test("summarizeSession preserves session_id from request", async () => {
      const { summarizeSession } = await import("@/agents/session-summary");

      const mockClient = {
        messages: {
          create: mock(() =>
            Promise.resolve({
              content: [
                {
                  type: "tool_use",
                  id: "test-id",
                  name: "output_session_summary",
                  input: {
                    summary: "General work",
                    key_files: [],
                    focus_area: null,
                  },
                },
              ],
            })
          ),
        },
      };

      const result = await summarizeSession(mockClient as never, {
        session_id: "my-unique-session-id",
        files: [],
        duration_seconds: null,
        chain_id: null,
      });

      expect(result.session_id).toBe("my-unique-session-id");
    });

    test("summarizeSession uses claude-haiku-4-5-20251001 model", async () => {
      const { summarizeSession } = await import("@/agents/session-summary");

      let capturedOptions: Record<string, unknown> | undefined;
      const mockClient = {
        messages: {
          create: mock((options: Record<string, unknown>) => {
            capturedOptions = options;
            return Promise.resolve({
              content: [
                {
                  type: "tool_use",
                  id: "test-id",
                  name: "output_session_summary",
                  input: {
                    summary: "Test",
                    key_files: [],
                    focus_area: null,
                  },
                },
              ],
            });
          }),
        },
      };

      await summarizeSession(mockClient as never, {
        session_id: "test",
        files: [],
        duration_seconds: null,
        chain_id: null,
      });

      expect(capturedOptions?.model).toBe("claude-haiku-4-5-20251001");
    });

    test("summarizeSession uses tool_choice pattern with correct tool name", async () => {
      const { summarizeSession } = await import("@/agents/session-summary");

      let capturedOptions: Record<string, unknown> | undefined;
      const mockClient = {
        messages: {
          create: mock((options: Record<string, unknown>) => {
            capturedOptions = options;
            return Promise.resolve({
              content: [
                {
                  type: "tool_use",
                  id: "test-id",
                  name: "output_session_summary",
                  input: {
                    summary: "Test",
                    key_files: [],
                    focus_area: null,
                  },
                },
              ],
            });
          }),
        },
      };

      await summarizeSession(mockClient as never, {
        session_id: "test",
        files: [],
        duration_seconds: null,
        chain_id: null,
      });

      expect(capturedOptions?.tool_choice).toEqual({
        type: "tool",
        name: "output_session_summary",
      });
    });

    test("summarizeSession includes model_used in response", async () => {
      const { summarizeSession } = await import("@/agents/session-summary");

      const mockClient = {
        messages: {
          create: mock(() =>
            Promise.resolve({
              content: [
                {
                  type: "tool_use",
                  id: "test-id",
                  name: "output_session_summary",
                  input: {
                    summary: "Authentication work",
                    key_files: ["src/auth.ts"],
                    focus_area: "Security",
                  },
                },
              ],
            })
          ),
        },
      };

      const result = await summarizeSession(mockClient as never, {
        session_id: "test",
        files: [],
        duration_seconds: null,
        chain_id: null,
      });

      expect(result.model_used).toBe("claude-haiku-4-5-20251001");
    });

    test("summarizeSession handles null focus_area", async () => {
      const { summarizeSession } = await import("@/agents/session-summary");

      const mockClient = {
        messages: {
          create: mock(() =>
            Promise.resolve({
              content: [
                {
                  type: "tool_use",
                  id: "test-id",
                  name: "output_session_summary",
                  input: {
                    summary: "General exploratory work",
                    key_files: [],
                    focus_area: null,
                  },
                },
              ],
            })
          ),
        },
      };

      const result = await summarizeSession(mockClient as never, {
        session_id: "test",
        files: [],
        duration_seconds: null,
        chain_id: null,
      });

      expect(result.focus_area).toBeNull();
    });
  });

  describe("error handling", () => {
    test("summarizeSession throws if no tool_use in response", async () => {
      const { summarizeSession } = await import("@/agents/session-summary");

      const mockClient = {
        messages: {
          create: mock(() =>
            Promise.resolve({
              content: [{ type: "text", text: "I cannot summarize this" }],
            })
          ),
        },
      };

      await expect(
        summarizeSession(mockClient as never, {
          session_id: "test",
          files: [],
          duration_seconds: null,
          chain_id: null,
        })
      ).rejects.toThrow();
    });
  });
});
