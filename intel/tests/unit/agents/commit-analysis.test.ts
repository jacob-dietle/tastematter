import { describe, test, expect, mock } from "bun:test";
import type { CommitAnalysisRequest } from "@/types/shared";
import { CommitAnalysisResponseSchema } from "@/types/shared";

/**
 * RED Tests for Commit Analysis Agent
 *
 * TDD: Write tests FIRST, then implement agent to make them GREEN
 *
 * Pattern: Follow chain-naming.ts exactly
 * Model: claude-sonnet-4-5-20250929 (requires reasoning)
 */

describe("Commit Analysis Agent", () => {
  describe("COMMIT_ANALYSIS_TOOL definition", () => {
    test("exports COMMIT_ANALYSIS_TOOL with correct name", async () => {
      const { COMMIT_ANALYSIS_TOOL } = await import("@/agents/commit-analysis");
      expect(COMMIT_ANALYSIS_TOOL.name).toBe("output_commit_analysis");
    });

    test("COMMIT_ANALYSIS_TOOL has required properties in schema", async () => {
      const { COMMIT_ANALYSIS_TOOL } = await import("@/agents/commit-analysis");
      const schema = COMMIT_ANALYSIS_TOOL.input_schema as unknown as {
        required: string[];
      };
      expect(schema.required).toContain("is_agent_commit");
      expect(schema.required).toContain("summary");
      expect(schema.required).toContain("risk_level");
      expect(schema.required).toContain("review_focus");
      expect(schema.required).toContain("related_files");
    });

    test("COMMIT_ANALYSIS_TOOL risk_level enum is correct", async () => {
      const { COMMIT_ANALYSIS_TOOL } = await import("@/agents/commit-analysis");
      const schema = COMMIT_ANALYSIS_TOOL.input_schema as unknown as {
        properties: { risk_level: { enum: string[] } };
      };
      expect(schema.properties.risk_level.enum).toEqual(["low", "medium", "high"]);
    });

    test("COMMIT_ANALYSIS_TOOL has is_agent_commit as boolean", async () => {
      const { COMMIT_ANALYSIS_TOOL } = await import("@/agents/commit-analysis");
      const schema = COMMIT_ANALYSIS_TOOL.input_schema as unknown as {
        properties: { is_agent_commit: { type: string } };
      };
      expect(schema.properties.is_agent_commit.type).toBe("boolean");
    });
  });

  describe("buildPrompt function", () => {
    test("buildPrompt includes commit hash", async () => {
      const { buildPrompt } = await import("@/agents/commit-analysis");
      const request: CommitAnalysisRequest = {
        commit_hash: "abc123def",
        message: "Fix bug",
        author: "dev",
        diff: "---",
        files_changed: [],
      };
      const prompt = buildPrompt(request);
      expect(prompt).toContain("abc123def");
    });

    test("buildPrompt includes commit message", async () => {
      const { buildPrompt } = await import("@/agents/commit-analysis");
      const request: CommitAnalysisRequest = {
        commit_hash: "test",
        message: "Fix authentication redirect loop",
        author: "dev",
        diff: "---",
        files_changed: [],
      };
      const prompt = buildPrompt(request);
      expect(prompt).toContain("Fix authentication redirect loop");
    });

    test("buildPrompt includes author", async () => {
      const { buildPrompt } = await import("@/agents/commit-analysis");
      const request: CommitAnalysisRequest = {
        commit_hash: "test",
        message: "Fix",
        author: "claude-code-bot",
        diff: "---",
        files_changed: [],
      };
      const prompt = buildPrompt(request);
      expect(prompt).toContain("claude-code-bot");
    });

    test("buildPrompt includes diff", async () => {
      const { buildPrompt } = await import("@/agents/commit-analysis");
      const request: CommitAnalysisRequest = {
        commit_hash: "test",
        message: "Fix",
        author: "dev",
        diff: "+++ new line\n--- old line\n@@ -1,5 +1,6 @@",
        files_changed: ["src/main.ts"],
      };
      const prompt = buildPrompt(request);
      expect(prompt).toContain("+++ new line");
      expect(prompt).toContain("@@ -1,5 +1,6 @@");
    });

    test("buildPrompt includes files changed", async () => {
      const { buildPrompt } = await import("@/agents/commit-analysis");
      const request: CommitAnalysisRequest = {
        commit_hash: "test",
        message: "Fix",
        author: "dev",
        diff: "---",
        files_changed: ["src/auth.ts", "src/login.ts", "tests/auth.test.ts"],
      };
      const prompt = buildPrompt(request);
      expect(prompt).toContain("src/auth.ts");
      expect(prompt).toContain("src/login.ts");
      expect(prompt).toContain("tests/auth.test.ts");
    });

    test("buildPrompt instructs to use the tool", async () => {
      const { buildPrompt } = await import("@/agents/commit-analysis");
      const request: CommitAnalysisRequest = {
        commit_hash: "test",
        message: "Fix",
        author: "dev",
        diff: "---",
        files_changed: [],
      };
      const prompt = buildPrompt(request);
      expect(prompt).toContain("output_commit_analysis");
    });
  });

  describe("analyzeCommit function", () => {
    test("analyzeCommit returns valid CommitAnalysisResponse", async () => {
      const { analyzeCommit } = await import("@/agents/commit-analysis");

      const mockClient = {
        messages: {
          create: mock(() =>
            Promise.resolve({
              content: [
                {
                  type: "tool_use",
                  id: "test-id",
                  name: "output_commit_analysis",
                  input: {
                    is_agent_commit: false,
                    summary: "Fixed authentication bug",
                    risk_level: "low",
                    review_focus: "Security validation",
                    related_files: ["src/auth.ts"],
                  },
                },
              ],
            })
          ),
        },
      };

      const request: CommitAnalysisRequest = {
        commit_hash: "abc123",
        message: "Fix auth bug",
        author: "dev",
        diff: "---",
        files_changed: ["src/auth.ts"],
      };

      const result = await analyzeCommit(mockClient as never, request);
      const validation = CommitAnalysisResponseSchema.safeParse(result);
      expect(validation.success).toBe(true);
    });

    test("analyzeCommit preserves commit_hash from request", async () => {
      const { analyzeCommit } = await import("@/agents/commit-analysis");

      const mockClient = {
        messages: {
          create: mock(() =>
            Promise.resolve({
              content: [
                {
                  type: "tool_use",
                  id: "test-id",
                  name: "output_commit_analysis",
                  input: {
                    is_agent_commit: true,
                    summary: "Test",
                    risk_level: "medium",
                    review_focus: "Test",
                    related_files: [],
                  },
                },
              ],
            })
          ),
        },
      };

      const result = await analyzeCommit(mockClient as never, {
        commit_hash: "my-specific-commit-hash",
        message: "test",
        author: "dev",
        diff: "---",
        files_changed: [],
      });

      expect(result.commit_hash).toBe("my-specific-commit-hash");
    });

    test("analyzeCommit uses claude-sonnet-4-5-20250929 model", async () => {
      const { analyzeCommit } = await import("@/agents/commit-analysis");

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
                  name: "output_commit_analysis",
                  input: {
                    is_agent_commit: false,
                    summary: "Test",
                    risk_level: "low",
                    review_focus: "Test",
                    related_files: [],
                  },
                },
              ],
            });
          }),
        },
      };

      await analyzeCommit(mockClient as never, {
        commit_hash: "test",
        message: "test",
        author: "dev",
        diff: "---",
        files_changed: [],
      });

      expect(capturedOptions?.model).toBe("claude-sonnet-4-5-20250929");
    });

    test("analyzeCommit uses tool_choice pattern with correct tool name", async () => {
      const { analyzeCommit } = await import("@/agents/commit-analysis");

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
                  name: "output_commit_analysis",
                  input: {
                    is_agent_commit: false,
                    summary: "Test",
                    risk_level: "low",
                    review_focus: "Test",
                    related_files: [],
                  },
                },
              ],
            });
          }),
        },
      };

      await analyzeCommit(mockClient as never, {
        commit_hash: "test",
        message: "test",
        author: "dev",
        diff: "---",
        files_changed: [],
      });

      expect(capturedOptions?.tool_choice).toEqual({
        type: "tool",
        name: "output_commit_analysis",
      });
    });

    test("analyzeCommit includes model_used in response", async () => {
      const { analyzeCommit } = await import("@/agents/commit-analysis");

      const mockClient = {
        messages: {
          create: mock(() =>
            Promise.resolve({
              content: [
                {
                  type: "tool_use",
                  id: "test-id",
                  name: "output_commit_analysis",
                  input: {
                    is_agent_commit: true,
                    summary: "Agent made this commit",
                    risk_level: "high",
                    review_focus: "Review all logic",
                    related_files: [],
                  },
                },
              ],
            })
          ),
        },
      };

      const result = await analyzeCommit(mockClient as never, {
        commit_hash: "test",
        message: "test",
        author: "dev",
        diff: "---",
        files_changed: [],
      });

      expect(result.model_used).toBe("claude-sonnet-4-5-20250929");
    });
  });

  describe("error handling", () => {
    test("analyzeCommit throws if no tool_use in response", async () => {
      const { analyzeCommit } = await import("@/agents/commit-analysis");

      const mockClient = {
        messages: {
          create: mock(() =>
            Promise.resolve({
              content: [{ type: "text", text: "I cannot analyze this" }],
            })
          ),
        },
      };

      await expect(
        analyzeCommit(mockClient as never, {
          commit_hash: "test",
          message: "test",
          author: "dev",
          diff: "---",
          files_changed: [],
        })
      ).rejects.toThrow();
    });

    test("analyzeCommit throws if tool_use has invalid risk_level", async () => {
      const { analyzeCommit } = await import("@/agents/commit-analysis");

      const mockClient = {
        messages: {
          create: mock(() =>
            Promise.resolve({
              content: [
                {
                  type: "tool_use",
                  id: "test-id",
                  name: "output_commit_analysis",
                  input: {
                    is_agent_commit: false,
                    summary: "Test",
                    risk_level: "critical", // Invalid
                    review_focus: "Test",
                    related_files: [],
                  },
                },
              ],
            })
          ),
        },
      };

      await expect(
        analyzeCommit(mockClient as never, {
          commit_hash: "test",
          message: "test",
          author: "dev",
          diff: "---",
          files_changed: [],
        })
      ).rejects.toThrow();
    });
  });
});
