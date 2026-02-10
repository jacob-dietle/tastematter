import { describe, test, expect } from "bun:test";

describe("ContextSynthesisRequestSchema", () => {
  test("validates complete request", async () => {
    const { ContextSynthesisRequestSchema } = await import("@/types/shared");

    const validRequest = {
      query: "nickel",
      status: "healthy",
      work_tempo: "active",
      clusters: [
        {
          files: ["src/auth.rs", "src/login.rs"],
          access_pattern: "high_access_high_session",
          pmi_score: 2.5,
        },
      ],
      suggested_reads: [
        { path: "specs/README.md", priority: 1, surprise: false },
      ],
      context_package_content: "# Context Package 35\n...",
      key_metrics: { files_in_scope: 20 },
      evidence_sources: ["specs/README.md", "CLAUDE.md"],
    };

    const result = ContextSynthesisRequestSchema.safeParse(validRequest);
    expect(result.success).toBe(true);
  });

  test("validates minimal request (optional fields omitted)", async () => {
    const { ContextSynthesisRequestSchema } = await import("@/types/shared");

    const minimalRequest = {
      query: "test",
      status: "unknown",
      work_tempo: "dormant",
      clusters: [],
      suggested_reads: [],
      evidence_sources: [],
    };

    const result = ContextSynthesisRequestSchema.safeParse(minimalRequest);
    expect(result.success).toBe(true);
  });

  test("rejects missing required fields", async () => {
    const { ContextSynthesisRequestSchema } = await import("@/types/shared");

    // Missing query
    expect(
      ContextSynthesisRequestSchema.safeParse({
        status: "healthy",
        work_tempo: "active",
        clusters: [],
        suggested_reads: [],
        evidence_sources: [],
      }).success
    ).toBe(false);

    // Missing clusters
    expect(
      ContextSynthesisRequestSchema.safeParse({
        query: "test",
        status: "healthy",
        work_tempo: "active",
        suggested_reads: [],
        evidence_sources: [],
      }).success
    ).toBe(false);
  });
});

describe("ContextSynthesisResponseSchema", () => {
  test("validates complete response", async () => {
    const { ContextSynthesisResponseSchema } = await import("@/types/shared");

    const validResponse = {
      one_liner: "Nickel transcript worker is production-ready with 4 providers",
      narrative: "You built a multi-provider ingestion system. It currently handles HubSpot, Intercom, Gong, and Fireflies. The system is deployed to Cloudflare Workers.",
      cluster_names: ["Core Pipeline", "Type Contracts"],
      cluster_interpretations: [
        "Active development files that move together",
        "Shared type definitions across providers",
      ],
      suggested_read_reasons: [
        "Latest context package — start here to resume",
        "Core worker entry point with all provider routes",
      ],
      model_used: "claude-haiku-4-5-20251001",
    };

    const result = ContextSynthesisResponseSchema.safeParse(validResponse);
    expect(result.success).toBe(true);
  });

  test("rejects response missing required fields", async () => {
    const { ContextSynthesisResponseSchema } = await import("@/types/shared");

    // Missing one_liner
    expect(
      ContextSynthesisResponseSchema.safeParse({
        narrative: "test",
        cluster_names: [],
        cluster_interpretations: [],
        suggested_read_reasons: [],
        model_used: "test",
      }).success
    ).toBe(false);
  });

  test("validates empty arrays (edge case: no clusters)", async () => {
    const { ContextSynthesisResponseSchema } = await import("@/types/shared");

    const emptyArrays = {
      one_liner: "Empty project with no activity",
      narrative: "No recent work detected.",
      cluster_names: [],
      cluster_interpretations: [],
      suggested_read_reasons: [],
      model_used: "claude-haiku-4-5-20251001",
    };

    const result = ContextSynthesisResponseSchema.safeParse(emptyArrays);
    expect(result.success).toBe(true);
  });
});
