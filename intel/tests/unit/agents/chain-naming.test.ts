import { describe, test, expect, mock, beforeEach } from "bun:test";
import type { ChainNamingRequest, ChainNamingResponse } from "@/types/shared";
import { ChainNamingResponseSchema } from "@/types/shared";

// RED tests - Write tests FIRST before implementation
// TDD: These tests will fail until we implement the chain naming agent

describe("Chain Naming Agent", () => {
  describe("CHAIN_NAMING_TOOL definition", () => {
    test("exports CHAIN_NAMING_TOOL with correct name", async () => {
      const { CHAIN_NAMING_TOOL } = await import("@/agents/chain-naming");
      expect(CHAIN_NAMING_TOOL.name).toBe("output_chain_name");
    });

    test("CHAIN_NAMING_TOOL has required properties in schema", async () => {
      const { CHAIN_NAMING_TOOL } = await import("@/agents/chain-naming");
      const schema = CHAIN_NAMING_TOOL.input_schema as unknown as {
        required: string[];
        properties: Record<string, unknown>;
      };
      expect(schema.required).toContain("generated_name");
      expect(schema.required).toContain("category");
      expect(schema.required).toContain("confidence");
    });

    test("CHAIN_NAMING_TOOL category enum matches Zod schema", async () => {
      const { CHAIN_NAMING_TOOL } = await import("@/agents/chain-naming");
      const schema = CHAIN_NAMING_TOOL.input_schema as unknown as {
        properties: {
          category: { enum: string[] };
        };
      };
      const expectedCategories = [
        "bug-fix",
        "feature",
        "refactor",
        "research",
        "cleanup",
        "documentation",
        "testing",
        "unknown",
      ];
      expect(schema.properties.category.enum).toEqual(expectedCategories);
    });
  });

  describe("buildPrompt function", () => {
    test("buildPrompt includes chain_id", async () => {
      const { buildPrompt } = await import("@/agents/chain-naming");
      const request: ChainNamingRequest = {
        chain_id: "test-chain-123",
        files_touched: ["src/auth.ts"],
        session_count: 3,
        recent_sessions: [],
      };
      const prompt = buildPrompt(request);
      expect(prompt).toContain("test-chain-123");
    });

    test("buildPrompt includes files_touched", async () => {
      const { buildPrompt } = await import("@/agents/chain-naming");
      const request: ChainNamingRequest = {
        chain_id: "test",
        files_touched: ["src/auth.ts", "src/login.ts", "tests/test_auth.py"],
        session_count: 3,
        recent_sessions: [],
      };
      const prompt = buildPrompt(request);
      expect(prompt).toContain("src/auth.ts");
      expect(prompt).toContain("src/login.ts");
      expect(prompt).toContain("tests/test_auth.py");
    });

    test("buildPrompt includes session_count", async () => {
      const { buildPrompt } = await import("@/agents/chain-naming");
      const request: ChainNamingRequest = {
        chain_id: "test",
        files_touched: [],
        session_count: 42,
        recent_sessions: [],
      };
      const prompt = buildPrompt(request);
      expect(prompt).toContain("42");
    });

    test("buildPrompt instructs to use the tool", async () => {
      const { buildPrompt } = await import("@/agents/chain-naming");
      const request: ChainNamingRequest = {
        chain_id: "test",
        files_touched: [],
        session_count: 1,
        recent_sessions: [],
      };
      const prompt = buildPrompt(request);
      // Should instruct to use the output tool (rules are in system prompt)
      expect(prompt).toContain("output_chain_name");
      expect(prompt).toContain("INPUT:");
    });

    test("buildPrompt includes first_user_intent when provided", async () => {
      const { buildPrompt } = await import("@/agents/chain-naming");
      const request: ChainNamingRequest = {
        chain_id: "test",
        files_touched: ["src/auth.ts"],
        session_count: 1,
        recent_sessions: [],
        first_user_intent: "Help me port the Python indexer to Rust",
      };
      const prompt = buildPrompt(request);
      expect(prompt).toContain("ENRICHMENT");
      expect(prompt).toContain("user_conversation_excerpt");
      expect(prompt).toContain("Help me port the Python indexer to Rust");
    });

    test("buildPrompt includes tools_used when provided", async () => {
      const { buildPrompt } = await import("@/agents/chain-naming");
      const request: ChainNamingRequest = {
        chain_id: "test",
        files_touched: [],
        session_count: 1,
        recent_sessions: [],
        tools_used: { Read: 23, Edit: 12 },
      };
      const prompt = buildPrompt(request);
      expect(prompt).toContain("tools_used");
      expect(prompt).toContain("Read");
      expect(prompt).toContain("23");
    });

    test("buildPrompt includes commit_messages when provided", async () => {
      const { buildPrompt } = await import("@/agents/chain-naming");
      const request: ChainNamingRequest = {
        chain_id: "test",
        files_touched: [],
        session_count: 1,
        recent_sessions: [],
        commit_messages: ["feat: Add query engine", "fix: Handle edge case"],
      };
      const prompt = buildPrompt(request);
      expect(prompt).toContain("commit_messages");
      expect(prompt).toContain("feat: Add query engine");
      expect(prompt).toContain("fix: Handle edge case");
    });

    test("buildPrompt omits ENRICHMENT section when no enrichment fields", async () => {
      const { buildPrompt } = await import("@/agents/chain-naming");
      const request: ChainNamingRequest = {
        chain_id: "test",
        files_touched: [],
        session_count: 1,
        recent_sessions: [],
      };
      const prompt = buildPrompt(request);
      expect(prompt).not.toContain("ENRICHMENT");
    });

    test("buildPrompt includes PRIORITY instruction for user intent", async () => {
      const { buildPrompt } = await import("@/agents/chain-naming");
      const request: ChainNamingRequest = {
        chain_id: "test",
        files_touched: [],
        session_count: 1,
        recent_sessions: [],
        first_user_intent: "Debug authentication",
      };
      const prompt = buildPrompt(request);
      expect(prompt).toContain("PRIORITY");
      expect(prompt).toContain("user_conversation_excerpt");
    });
  });

  describe("nameChain function", () => {
    test("nameChain returns valid ChainNamingResponse structure", async () => {
      const { nameChain } = await import("@/agents/chain-naming");

      // Create a mock Anthropic client
      const mockClient = {
        messages: {
          create: mock(() =>
            Promise.resolve({
              content: [
                {
                  type: "tool_use",
                  id: "test-id",
                  name: "output_chain_name",
                  input: {
                    generated_name: "Fixed authentication flow",
                    category: "bug-fix",
                    confidence: 0.9,
                  },
                },
              ],
            })
          ),
        },
      };

      const request: ChainNamingRequest = {
        chain_id: "test-chain",
        files_touched: ["src/auth.ts"],
        session_count: 5,
        recent_sessions: ["session-1", "session-2"],
      };

      const result = await nameChain(mockClient as never, request);

      // Validate result matches our Zod schema
      const validation = ChainNamingResponseSchema.safeParse(result);
      expect(validation.success).toBe(true);
    });

    test("nameChain preserves chain_id from request", async () => {
      const { nameChain } = await import("@/agents/chain-naming");

      const mockClient = {
        messages: {
          create: mock(() =>
            Promise.resolve({
              content: [
                {
                  type: "tool_use",
                  id: "test-id",
                  name: "output_chain_name",
                  input: {
                    generated_name: "Test name",
                    category: "unknown",
                    confidence: 0.5,
                  },
                },
              ],
            })
          ),
        },
      };

      const request: ChainNamingRequest = {
        chain_id: "my-specific-chain-id",
        files_touched: [],
        session_count: 1,
        recent_sessions: [],
      };

      const result = await nameChain(mockClient as never, request);
      expect(result.chain_id).toBe("my-specific-chain-id");
    });

    test("nameChain uses tool_choice pattern with correct tool name", async () => {
      const { nameChain } = await import("@/agents/chain-naming");

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
                  name: "output_chain_name",
                  input: {
                    generated_name: "Test",
                    category: "unknown",
                    confidence: 0.5,
                  },
                },
              ],
            });
          }),
        },
      };

      await nameChain(mockClient as never, {
        chain_id: "test",
        files_touched: [],
        session_count: 1,
        recent_sessions: [],
      });

      // Verify tool_choice pattern was used
      expect(capturedOptions?.tool_choice).toEqual({
        type: "tool",
        name: "output_chain_name",
      });
    });

    test("nameChain uses claude-haiku-4-5-20251001 model", async () => {
      const { nameChain } = await import("@/agents/chain-naming");

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
                  name: "output_chain_name",
                  input: {
                    generated_name: "Test",
                    category: "unknown",
                    confidence: 0.5,
                  },
                },
              ],
            });
          }),
        },
      };

      await nameChain(mockClient as never, {
        chain_id: "test",
        files_touched: [],
        session_count: 1,
        recent_sessions: [],
      });

      expect(capturedOptions?.model).toBe("claude-haiku-4-5-20251001");
    });

    test("nameChain includes model_used in response", async () => {
      const { nameChain } = await import("@/agents/chain-naming");

      const mockClient = {
        messages: {
          create: mock(() =>
            Promise.resolve({
              content: [
                {
                  type: "tool_use",
                  id: "test-id",
                  name: "output_chain_name",
                  input: {
                    generated_name: "Test",
                    category: "feature",
                    confidence: 0.8,
                  },
                },
              ],
            })
          ),
        },
      };

      const result = await nameChain(mockClient as never, {
        chain_id: "test",
        files_touched: [],
        session_count: 1,
        recent_sessions: [],
      });

      expect(result.model_used).toBe("claude-haiku-4-5-20251001");
    });
  });

  describe("error handling", () => {
    test("nameChain throws if no tool_use in response", async () => {
      const { nameChain } = await import("@/agents/chain-naming");

      const mockClient = {
        messages: {
          create: mock(() =>
            Promise.resolve({
              content: [{ type: "text", text: "I cannot do that" }],
            })
          ),
        },
      };

      await expect(
        nameChain(mockClient as never, {
          chain_id: "test",
          files_touched: [],
          session_count: 1,
          recent_sessions: [],
        })
      ).rejects.toThrow();
    });

    test("nameChain throws if tool_use has invalid input", async () => {
      const { nameChain } = await import("@/agents/chain-naming");

      const mockClient = {
        messages: {
          create: mock(() =>
            Promise.resolve({
              content: [
                {
                  type: "tool_use",
                  id: "test-id",
                  name: "output_chain_name",
                  input: {
                    // Missing required fields
                    generated_name: "Test",
                    // category missing
                    // confidence missing
                  },
                },
              ],
            })
          ),
        },
      };

      await expect(
        nameChain(mockClient as never, {
          chain_id: "test",
          files_touched: [],
          session_count: 1,
          recent_sessions: [],
        })
      ).rejects.toThrow();
    });
  });
});
