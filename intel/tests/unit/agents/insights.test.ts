import { describe, test, expect, mock } from "bun:test";
import type { InsightsRequest } from "@/types/shared";
import { InsightsResponseSchema } from "@/types/shared";

/**
 * RED Tests for Insights Agent
 *
 * TDD: Write tests FIRST, then implement agent to make them GREEN
 *
 * Pattern: Follow chain-naming.ts exactly
 * Model: claude-sonnet-4-5-20250929 (needs reasoning for pattern detection)
 *
 * This is the most complex agent - outputs multiple insights with actions
 */

describe("Insights Agent", () => {
  describe("INSIGHTS_TOOL definition", () => {
    test("exports INSIGHTS_TOOL with correct name", async () => {
      const { INSIGHTS_TOOL } = await import("@/agents/insights");
      expect(INSIGHTS_TOOL.name).toBe("output_insights");
    });

    test("INSIGHTS_TOOL has insights array property", async () => {
      const { INSIGHTS_TOOL } = await import("@/agents/insights");
      const schema = INSIGHTS_TOOL.input_schema as unknown as {
        properties: { insights: { type: string } };
      };
      expect(schema.properties.insights.type).toBe("array");
    });

    test("INSIGHTS_TOOL has required insights property", async () => {
      const { INSIGHTS_TOOL } = await import("@/agents/insights");
      const schema = INSIGHTS_TOOL.input_schema as unknown as {
        required: string[];
      };
      expect(schema.required).toContain("insights");
    });

    test("INSIGHTS_TOOL insight items have all required fields", async () => {
      const { INSIGHTS_TOOL } = await import("@/agents/insights");
      const schema = INSIGHTS_TOOL.input_schema as unknown as {
        properties: {
          insights: {
            items: {
              required: string[];
            };
          };
        };
      };
      const itemRequired = schema.properties.insights.items.required;
      expect(itemRequired).toContain("id");
      expect(itemRequired).toContain("insight_type");
      expect(itemRequired).toContain("title");
      expect(itemRequired).toContain("description");
      expect(itemRequired).toContain("evidence");
    });

    test("INSIGHTS_TOOL insight_type enum is correct", async () => {
      const { INSIGHTS_TOOL } = await import("@/agents/insights");
      const schema = INSIGHTS_TOOL.input_schema as unknown as {
        properties: {
          insights: {
            items: {
              properties: {
                insight_type: { enum: string[] };
              };
            };
          };
        };
      };
      const typeEnum = schema.properties.insights.items.properties.insight_type.enum;
      expect(typeEnum).toContain("focus-shift");
      expect(typeEnum).toContain("co-occurrence");
      expect(typeEnum).toContain("pending-review");
      expect(typeEnum).toContain("anomaly");
      expect(typeEnum).toContain("continuity");
    });
  });

  describe("buildPrompt function", () => {
    test("buildPrompt includes time_range", async () => {
      const { buildPrompt } = await import("@/agents/insights");
      const request: InsightsRequest = {
        time_range: "7d",
        chain_data: [],
        file_patterns: [],
      };
      const prompt = buildPrompt(request);
      expect(prompt).toContain("7d");
    });

    test("buildPrompt includes chain data", async () => {
      const { buildPrompt } = await import("@/agents/insights");
      const request: InsightsRequest = {
        time_range: "24h",
        chain_data: [
          {
            chain_id: "chain-auth-refactor",
            name: "Auth Refactor",
            session_count: 5,
            file_count: 12,
            recent_activity: "2h ago",
          },
        ],
        file_patterns: [],
      };
      const prompt = buildPrompt(request);
      expect(prompt).toContain("chain-auth-refactor");
      expect(prompt).toContain("Auth Refactor");
      expect(prompt).toContain("5");
      expect(prompt).toContain("12");
    });

    test("buildPrompt includes file patterns", async () => {
      const { buildPrompt } = await import("@/agents/insights");
      const request: InsightsRequest = {
        time_range: "7d",
        chain_data: [],
        file_patterns: [
          {
            file_path: "src/auth.ts",
            access_count: 15,
            co_accessed_with: ["src/login.ts", "src/session.ts"],
          },
        ],
      };
      const prompt = buildPrompt(request);
      expect(prompt).toContain("src/auth.ts");
      expect(prompt).toContain("15");
      expect(prompt).toContain("src/login.ts");
    });

    test("buildPrompt instructs to use the tool", async () => {
      const { buildPrompt } = await import("@/agents/insights");
      const request: InsightsRequest = {
        time_range: "7d",
        chain_data: [],
        file_patterns: [],
      };
      const prompt = buildPrompt(request);
      expect(prompt).toContain("output_insights");
    });
  });

  describe("generateInsights function", () => {
    test("generateInsights returns valid InsightsResponse", async () => {
      const { generateInsights } = await import("@/agents/insights");

      const mockClient = {
        messages: {
          create: mock(() =>
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
                        evidence: [
                          "5 sessions touched auth.ts in last 24h",
                          "3 new test files for authentication",
                        ],
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
          ),
        },
      };

      const request: InsightsRequest = {
        time_range: "7d",
        chain_data: [],
        file_patterns: [],
      };

      const result = await generateInsights(mockClient as never, request);
      const validation = InsightsResponseSchema.safeParse(result);
      expect(validation.success).toBe(true);
    });

    test("generateInsights returns empty insights array when no patterns found", async () => {
      const { generateInsights } = await import("@/agents/insights");

      const mockClient = {
        messages: {
          create: mock(() =>
            Promise.resolve({
              content: [
                {
                  type: "tool_use",
                  id: "test-id",
                  name: "output_insights",
                  input: {
                    insights: [],
                  },
                },
              ],
            })
          ),
        },
      };

      const result = await generateInsights(mockClient as never, {
        time_range: "1h",
        chain_data: [],
        file_patterns: [],
      });

      expect(result.insights).toEqual([]);
    });

    test("generateInsights uses claude-sonnet-4-5-20250929 model", async () => {
      const { generateInsights } = await import("@/agents/insights");

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
                  name: "output_insights",
                  input: {
                    insights: [],
                  },
                },
              ],
            });
          }),
        },
      };

      await generateInsights(mockClient as never, {
        time_range: "7d",
        chain_data: [],
        file_patterns: [],
      });

      expect(capturedOptions?.model).toBe("claude-sonnet-4-5-20250929");
    });

    test("generateInsights uses tool_choice pattern with correct tool name", async () => {
      const { generateInsights } = await import("@/agents/insights");

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
                  name: "output_insights",
                  input: {
                    insights: [],
                  },
                },
              ],
            });
          }),
        },
      };

      await generateInsights(mockClient as never, {
        time_range: "7d",
        chain_data: [],
        file_patterns: [],
      });

      expect(capturedOptions?.tool_choice).toEqual({
        type: "tool",
        name: "output_insights",
      });
    });

    test("generateInsights includes model_used in response", async () => {
      const { generateInsights } = await import("@/agents/insights");

      const mockClient = {
        messages: {
          create: mock(() =>
            Promise.resolve({
              content: [
                {
                  type: "tool_use",
                  id: "test-id",
                  name: "output_insights",
                  input: {
                    insights: [],
                  },
                },
              ],
            })
          ),
        },
      };

      const result = await generateInsights(mockClient as never, {
        time_range: "7d",
        chain_data: [],
        file_patterns: [],
      });

      expect(result.model_used).toBe("claude-sonnet-4-5-20250929");
    });

    test("generateInsights handles insight with null action", async () => {
      const { generateInsights } = await import("@/agents/insights");

      const mockClient = {
        messages: {
          create: mock(() =>
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
                        insight_type: "anomaly",
                        title: "Unusual file access pattern",
                        description: "Some files accessed at unusual times",
                        evidence: ["Evidence 1"],
                        action: null,
                      },
                    ],
                  },
                },
              ],
            })
          ),
        },
      };

      const result = await generateInsights(mockClient as never, {
        time_range: "7d",
        chain_data: [],
        file_patterns: [],
      });

      expect(result.insights[0].action).toBeNull();
    });

    test("generateInsights handles multiple insight types", async () => {
      const { generateInsights } = await import("@/agents/insights");

      const mockClient = {
        messages: {
          create: mock(() =>
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
                        title: "Focus changed",
                        description: "Desc 1",
                        evidence: ["E1"],
                        action: null,
                      },
                      {
                        id: "insight-2",
                        insight_type: "co-occurrence",
                        title: "Files accessed together",
                        description: "Desc 2",
                        evidence: ["E2"],
                        action: {
                          label: "View files",
                          action_type: "filter",
                          payload: {},
                        },
                      },
                      {
                        id: "insight-3",
                        insight_type: "pending-review",
                        title: "Review needed",
                        description: "Desc 3",
                        evidence: ["E3"],
                        action: null,
                      },
                    ],
                  },
                },
              ],
            })
          ),
        },
      };

      const result = await generateInsights(mockClient as never, {
        time_range: "7d",
        chain_data: [],
        file_patterns: [],
      });

      expect(result.insights).toHaveLength(3);
      expect(result.insights[0].insight_type).toBe("focus-shift");
      expect(result.insights[1].insight_type).toBe("co-occurrence");
      expect(result.insights[2].insight_type).toBe("pending-review");
    });
  });

  describe("error handling", () => {
    test("generateInsights throws if no tool_use in response", async () => {
      const { generateInsights } = await import("@/agents/insights");

      const mockClient = {
        messages: {
          create: mock(() =>
            Promise.resolve({
              content: [{ type: "text", text: "I cannot generate insights" }],
            })
          ),
        },
      };

      await expect(
        generateInsights(mockClient as never, {
          time_range: "7d",
          chain_data: [],
          file_patterns: [],
        })
      ).rejects.toThrow();
    });

    test("generateInsights throws if insight has invalid type", async () => {
      const { generateInsights } = await import("@/agents/insights");

      const mockClient = {
        messages: {
          create: mock(() =>
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
                        insight_type: "invalid-type", // Invalid
                        title: "Test",
                        description: "Test",
                        evidence: [],
                        action: null,
                      },
                    ],
                  },
                },
              ],
            })
          ),
        },
      };

      await expect(
        generateInsights(mockClient as never, {
          time_range: "7d",
          chain_data: [],
          file_patterns: [],
        })
      ).rejects.toThrow();
    });
  });
});
