import { describe, test, expect } from "bun:test";
import type { ContextSynthesisRequest } from "@/types/shared";

describe("Context Synthesis Agent", () => {
  describe("CONTEXT_SYNTHESIS_TOOL definition", () => {
    test("exports CONTEXT_SYNTHESIS_TOOL with correct name", async () => {
      const { CONTEXT_SYNTHESIS_TOOL } = await import("@/agents/context-synthesis");
      expect(CONTEXT_SYNTHESIS_TOOL.name).toBe("output_context_synthesis");
    });

    test("CONTEXT_SYNTHESIS_TOOL has all 5 required output fields", async () => {
      const { CONTEXT_SYNTHESIS_TOOL } = await import("@/agents/context-synthesis");
      const schema = CONTEXT_SYNTHESIS_TOOL.input_schema as unknown as {
        required: string[];
        properties: Record<string, unknown>;
      };
      expect(schema.required).toContain("one_liner");
      expect(schema.required).toContain("narrative");
      expect(schema.required).toContain("cluster_names");
      expect(schema.required).toContain("cluster_interpretations");
      expect(schema.required).toContain("suggested_read_reasons");
    });

    test("CONTEXT_SYNTHESIS_TOOL has array types for indexed fields", async () => {
      const { CONTEXT_SYNTHESIS_TOOL } = await import("@/agents/context-synthesis");
      const schema = CONTEXT_SYNTHESIS_TOOL.input_schema as unknown as {
        properties: Record<string, { type: string }>;
      };
      expect(schema.properties.cluster_names.type).toBe("array");
      expect(schema.properties.cluster_interpretations.type).toBe("array");
      expect(schema.properties.suggested_read_reasons.type).toBe("array");
      expect(schema.properties.one_liner.type).toBe("string");
      expect(schema.properties.narrative.type).toBe("string");
    });
  });

  describe("buildPrompt function", () => {
    test("buildPrompt includes query", async () => {
      const { buildPrompt } = await import("@/agents/context-synthesis");
      const request: ContextSynthesisRequest = {
        query: "nickel-transcript",
        status: "healthy",
        work_tempo: "active",
        clusters: [],
        suggested_reads: [],
        evidence_sources: [],
      };
      const prompt = buildPrompt(request);
      expect(prompt).toContain("nickel-transcript");
    });

    test("buildPrompt includes status and tempo", async () => {
      const { buildPrompt } = await import("@/agents/context-synthesis");
      const request: ContextSynthesisRequest = {
        query: "test",
        status: "warning",
        work_tempo: "cooling",
        clusters: [],
        suggested_reads: [],
        evidence_sources: [],
      };
      const prompt = buildPrompt(request);
      expect(prompt).toContain("warning");
      expect(prompt).toContain("cooling");
    });

    test("buildPrompt includes numbered clusters with files", async () => {
      const { buildPrompt } = await import("@/agents/context-synthesis");
      const request: ContextSynthesisRequest = {
        query: "test",
        status: "healthy",
        work_tempo: "active",
        clusters: [
          {
            files: ["src/auth.rs", "src/login.rs"],
            access_pattern: "high_access_high_session",
            pmi_score: 2.5,
          },
          {
            files: ["tests/test_auth.rs"],
            access_pattern: "low_access_low_session",
            pmi_score: 1.2,
          },
        ],
        suggested_reads: [],
        evidence_sources: [],
      };
      const prompt = buildPrompt(request);
      expect(prompt).toContain("Cluster 1");
      expect(prompt).toContain("Cluster 2");
      expect(prompt).toContain("src/auth.rs");
      expect(prompt).toContain("2.50"); // PMI score formatted
    });

    test("buildPrompt includes numbered reads", async () => {
      const { buildPrompt } = await import("@/agents/context-synthesis");
      const request: ContextSynthesisRequest = {
        query: "test",
        status: "healthy",
        work_tempo: "active",
        clusters: [],
        suggested_reads: [
          { path: "specs/README.md", priority: 1, surprise: false },
          { path: "src/weird_file.rs", priority: 2, surprise: true },
        ],
        evidence_sources: [],
      };
      const prompt = buildPrompt(request);
      expect(prompt).toContain("Read 1");
      expect(prompt).toContain("Read 2");
      expect(prompt).toContain("specs/README.md");
      expect(prompt).toContain("surprise");
    });

    test("buildPrompt includes context package content when present", async () => {
      const { buildPrompt } = await import("@/agents/context-synthesis");
      const request: ContextSynthesisRequest = {
        query: "test",
        status: "healthy",
        work_tempo: "active",
        clusters: [],
        suggested_reads: [],
        context_package_content: "# Context Package 35\nDB auto-init complete.",
        evidence_sources: [],
      };
      const prompt = buildPrompt(request);
      expect(prompt).toContain("Context Package 35");
      expect(prompt).toContain("DB auto-init complete");
    });

    test("buildPrompt omits context section when not present", async () => {
      const { buildPrompt } = await import("@/agents/context-synthesis");
      const request: ContextSynthesisRequest = {
        query: "test",
        status: "healthy",
        work_tempo: "active",
        clusters: [],
        suggested_reads: [],
        evidence_sources: [],
      };
      const prompt = buildPrompt(request);
      expect(prompt).not.toContain("Context Package Content");
    });

    test("buildPrompt truncates context package content at 3000 chars", async () => {
      const { buildPrompt } = await import("@/agents/context-synthesis");
      const longContent = "A".repeat(5000);
      const request: ContextSynthesisRequest = {
        query: "test",
        status: "healthy",
        work_tempo: "active",
        clusters: [],
        suggested_reads: [],
        context_package_content: longContent,
        evidence_sources: [],
      };
      const prompt = buildPrompt(request);
      // Should not contain the full 5000 chars
      const contentMatch = prompt.match(/A+/g);
      const longestRun = contentMatch
        ? Math.max(...contentMatch.map((m) => m.length))
        : 0;
      expect(longestRun).toBeLessThanOrEqual(3000);
    });
  });
});
