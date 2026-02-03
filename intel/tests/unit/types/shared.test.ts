import { describe, test, expect } from "bun:test";

// RED tests - these will fail until we implement the schemas
// TDD: Write tests first, then implementation

describe("ChainCategorySchema", () => {
  test("validates kebab-case enum values matching Rust serde", async () => {
    const { ChainCategorySchema } = await import("@/types/shared");

    // Valid categories (must match Rust #[serde(rename_all = "kebab-case")])
    expect(ChainCategorySchema.safeParse("bug-fix").success).toBe(true);
    expect(ChainCategorySchema.safeParse("feature").success).toBe(true);
    expect(ChainCategorySchema.safeParse("refactor").success).toBe(true);
    expect(ChainCategorySchema.safeParse("research").success).toBe(true);
    expect(ChainCategorySchema.safeParse("cleanup").success).toBe(true);
    expect(ChainCategorySchema.safeParse("documentation").success).toBe(true);
    expect(ChainCategorySchema.safeParse("testing").success).toBe(true);
    expect(ChainCategorySchema.safeParse("unknown").success).toBe(true);
  });

  test("rejects invalid categories", async () => {
    const { ChainCategorySchema } = await import("@/types/shared");

    expect(ChainCategorySchema.safeParse("BugFix").success).toBe(false);
    expect(ChainCategorySchema.safeParse("bug_fix").success).toBe(false);
    expect(ChainCategorySchema.safeParse("invalid").success).toBe(false);
    expect(ChainCategorySchema.safeParse("").success).toBe(false);
    expect(ChainCategorySchema.safeParse(null).success).toBe(false);
  });
});

describe("ChainNamingRequestSchema", () => {
  test("validates complete request", async () => {
    const { ChainNamingRequestSchema } = await import("@/types/shared");

    const validRequest = {
      chain_id: "abc123",
      files_touched: ["src/main.rs", "src/lib.rs"],
      session_count: 5,
      recent_sessions: ["Session 1: Fixed bug", "Session 2: Added feature"]
    };

    const result = ChainNamingRequestSchema.safeParse(validRequest);
    expect(result.success).toBe(true);
  });

  test("requires chain_id to be non-empty", async () => {
    const { ChainNamingRequestSchema } = await import("@/types/shared");

    const emptyChainId = {
      chain_id: "",
      files_touched: [],
      session_count: 1,
      recent_sessions: []
    };

    expect(ChainNamingRequestSchema.safeParse(emptyChainId).success).toBe(false);
  });

  test("requires session_count to be positive integer", async () => {
    const { ChainNamingRequestSchema } = await import("@/types/shared");

    const zeroSessions = {
      chain_id: "abc",
      files_touched: [],
      session_count: 0,
      recent_sessions: []
    };

    const negativeSessions = {
      chain_id: "abc",
      files_touched: [],
      session_count: -1,
      recent_sessions: []
    };

    expect(ChainNamingRequestSchema.safeParse(zeroSessions).success).toBe(false);
    expect(ChainNamingRequestSchema.safeParse(negativeSessions).success).toBe(false);
  });
});

describe("ChainNamingResponseSchema", () => {
  test("validates complete response", async () => {
    const { ChainNamingResponseSchema } = await import("@/types/shared");

    const validResponse = {
      chain_id: "abc123",
      generated_name: "Authentication Bug Fix",
      category: "bug-fix",
      confidence: 0.85,
      model_used: "claude-3-5-haiku-latest"
    };

    const result = ChainNamingResponseSchema.safeParse(validResponse);
    expect(result.success).toBe(true);
  });

  test("validates confidence between 0 and 1", async () => {
    const { ChainNamingResponseSchema } = await import("@/types/shared");

    const makeResponse = (confidence: number) => ({
      chain_id: "abc",
      generated_name: "Test",
      category: "feature",
      confidence,
      model_used: "claude-3-5-haiku-latest"
    });

    expect(ChainNamingResponseSchema.safeParse(makeResponse(0)).success).toBe(true);
    expect(ChainNamingResponseSchema.safeParse(makeResponse(0.5)).success).toBe(true);
    expect(ChainNamingResponseSchema.safeParse(makeResponse(1)).success).toBe(true);
    expect(ChainNamingResponseSchema.safeParse(makeResponse(-0.1)).success).toBe(false);
    expect(ChainNamingResponseSchema.safeParse(makeResponse(1.1)).success).toBe(false);
  });

  test("requires category to be valid ChainCategory", async () => {
    const { ChainNamingResponseSchema } = await import("@/types/shared");

    const invalidCategory = {
      chain_id: "abc",
      generated_name: "Test",
      category: "invalid-category",
      confidence: 0.5,
      model_used: "claude-3-5-haiku-latest"
    };

    expect(ChainNamingResponseSchema.safeParse(invalidCategory).success).toBe(false);
  });
});

describe("ChainMetadataSchema", () => {
  test("validates complete metadata", async () => {
    const { ChainMetadataSchema } = await import("@/types/shared");

    const validMetadata = {
      chain_id: "abc123",
      generated_name: "Auth Feature",
      category: "feature",
      confidence: 0.9,
      generated_at: "2026-01-25T12:00:00Z",
      model_used: "claude-3-5-haiku-latest"
    };

    const result = ChainMetadataSchema.safeParse(validMetadata);
    expect(result.success).toBe(true);
  });

  test("allows nullable fields", async () => {
    const { ChainMetadataSchema } = await import("@/types/shared");

    const minimalMetadata = {
      chain_id: "abc123",
      generated_name: null,
      category: null,
      confidence: null,
      generated_at: null,
      model_used: null
    };

    const result = ChainMetadataSchema.safeParse(minimalMetadata);
    expect(result.success).toBe(true);
  });

  test("validates generated_at as ISO datetime", async () => {
    const { ChainMetadataSchema } = await import("@/types/shared");

    const validDatetime = {
      chain_id: "abc",
      generated_name: "Test",
      category: "feature",
      confidence: 0.5,
      generated_at: "2026-01-25T12:00:00Z",
      model_used: "claude-3-5-haiku-latest"
    };

    const invalidDatetime = {
      chain_id: "abc",
      generated_name: "Test",
      category: "feature",
      confidence: 0.5,
      generated_at: "not-a-date",
      model_used: "claude-3-5-haiku-latest"
    };

    expect(ChainMetadataSchema.safeParse(validDatetime).success).toBe(true);
    expect(ChainMetadataSchema.safeParse(invalidDatetime).success).toBe(false);
  });
});

describe("HealthResponseSchema", () => {
  test("validates health response", async () => {
    const { HealthResponseSchema } = await import("@/types/shared");

    const validResponse = {
      status: "ok",
      version: "0.1.0"
    };

    const result = HealthResponseSchema.safeParse(validResponse);
    expect(result.success).toBe(true);
  });

  test("validates status enum", async () => {
    const { HealthResponseSchema } = await import("@/types/shared");

    expect(HealthResponseSchema.safeParse({ status: "ok", version: "0.1.0" }).success).toBe(true);
    expect(HealthResponseSchema.safeParse({ status: "error", version: "0.1.0" }).success).toBe(true);
    expect(HealthResponseSchema.safeParse({ status: "invalid", version: "0.1.0" }).success).toBe(false);
  });
});
