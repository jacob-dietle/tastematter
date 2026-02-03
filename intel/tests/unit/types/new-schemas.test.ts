import { describe, test, expect } from "bun:test";

/**
 * RED Tests for new Zod schemas
 *
 * These tests drive the implementation of:
 * - RiskLevelSchema
 * - InsightTypeSchema
 * - ActionTypeSchema
 * - CommitAnalysisRequest/Response schemas
 * - Insights schemas
 * - SessionSummary schemas
 *
 * TDD: Write tests FIRST, then implement schemas to make them GREEN
 */

describe("RiskLevelSchema", () => {
  test("accepts valid risk levels", async () => {
    const { RiskLevelSchema } = await import("@/types/shared");
    expect(RiskLevelSchema.parse("low")).toBe("low");
    expect(RiskLevelSchema.parse("medium")).toBe("medium");
    expect(RiskLevelSchema.parse("high")).toBe("high");
  });

  test("rejects invalid risk levels", async () => {
    const { RiskLevelSchema } = await import("@/types/shared");
    expect(() => RiskLevelSchema.parse("critical")).toThrow();
    expect(() => RiskLevelSchema.parse("MEDIUM")).toThrow();
    expect(() => RiskLevelSchema.parse("")).toThrow();
  });
});

describe("InsightTypeSchema", () => {
  test("accepts all valid insight types", async () => {
    const { InsightTypeSchema } = await import("@/types/shared");
    const types = [
      "focus-shift",
      "co-occurrence",
      "pending-review",
      "anomaly",
      "continuity",
    ];
    types.forEach((t) => expect(InsightTypeSchema.parse(t)).toBe(t));
  });

  test("rejects invalid insight types", async () => {
    const { InsightTypeSchema } = await import("@/types/shared");
    expect(() => InsightTypeSchema.parse("focus_shift")).toThrow();
    expect(() => InsightTypeSchema.parse("invalid")).toThrow();
  });
});

describe("ActionTypeSchema", () => {
  test("accepts all valid action types", async () => {
    const { ActionTypeSchema } = await import("@/types/shared");
    expect(ActionTypeSchema.parse("navigate")).toBe("navigate");
    expect(ActionTypeSchema.parse("filter")).toBe("filter");
    expect(ActionTypeSchema.parse("external")).toBe("external");
  });

  test("rejects invalid action types", async () => {
    const { ActionTypeSchema } = await import("@/types/shared");
    expect(() => ActionTypeSchema.parse("click")).toThrow();
    expect(() => ActionTypeSchema.parse("")).toThrow();
  });
});

describe("CommitAnalysisRequestSchema", () => {
  test("validates complete request", async () => {
    const { CommitAnalysisRequestSchema } = await import("@/types/shared");
    const request = {
      commit_hash: "abc123",
      message: "Fix authentication bug",
      author: "developer",
      diff: "--- a/auth.ts\n+++ b/auth.ts",
      files_changed: ["src/auth.ts"],
    };
    const result = CommitAnalysisRequestSchema.safeParse(request);
    expect(result.success).toBe(true);
  });

  test("rejects missing required fields", async () => {
    const { CommitAnalysisRequestSchema } = await import("@/types/shared");
    const result = CommitAnalysisRequestSchema.safeParse({
      commit_hash: "abc",
    });
    expect(result.success).toBe(false);
  });

  test("rejects empty commit_hash", async () => {
    const { CommitAnalysisRequestSchema } = await import("@/types/shared");
    const result = CommitAnalysisRequestSchema.safeParse({
      commit_hash: "",
      message: "Fix bug",
      author: "dev",
      diff: "---",
      files_changed: [],
    });
    expect(result.success).toBe(false);
  });

  test("accepts empty files_changed array", async () => {
    const { CommitAnalysisRequestSchema } = await import("@/types/shared");
    const request = {
      commit_hash: "abc123",
      message: "Fix bug",
      author: "dev",
      diff: "---",
      files_changed: [],
    };
    const result = CommitAnalysisRequestSchema.safeParse(request);
    expect(result.success).toBe(true);
  });
});

describe("CommitAnalysisResponseSchema", () => {
  test("validates complete response", async () => {
    const { CommitAnalysisResponseSchema } = await import("@/types/shared");
    const response = {
      commit_hash: "abc123",
      is_agent_commit: true,
      summary: "Fixed auth bug",
      risk_level: "low",
      review_focus: "Security validation",
      related_files: ["src/auth.ts"],
      model_used: "claude-sonnet-4-5-20250929",
    };
    const result = CommitAnalysisResponseSchema.safeParse(response);
    expect(result.success).toBe(true);
  });

  test("validates all risk levels", async () => {
    const { CommitAnalysisResponseSchema } = await import("@/types/shared");

    const makeResponse = (riskLevel: string) => ({
      commit_hash: "abc",
      is_agent_commit: false,
      summary: "Test",
      risk_level: riskLevel,
      review_focus: "Test",
      related_files: [],
      model_used: "sonnet",
    });

    expect(CommitAnalysisResponseSchema.safeParse(makeResponse("low")).success).toBe(true);
    expect(CommitAnalysisResponseSchema.safeParse(makeResponse("medium")).success).toBe(true);
    expect(CommitAnalysisResponseSchema.safeParse(makeResponse("high")).success).toBe(true);
    expect(CommitAnalysisResponseSchema.safeParse(makeResponse("critical")).success).toBe(false);
  });

  test("accepts empty related_files array", async () => {
    const { CommitAnalysisResponseSchema } = await import("@/types/shared");
    const response = {
      commit_hash: "abc123",
      is_agent_commit: false,
      summary: "Fixed bug",
      risk_level: "low",
      review_focus: "Logic",
      related_files: [],
      model_used: "sonnet",
    };
    const result = CommitAnalysisResponseSchema.safeParse(response);
    expect(result.success).toBe(true);
  });
});

describe("InsightActionSchema", () => {
  test("validates complete action", async () => {
    const { InsightActionSchema } = await import("@/types/shared");
    const action = {
      label: "View auth files",
      action_type: "filter",
      payload: { files: ["src/auth.ts"] },
    };
    const result = InsightActionSchema.safeParse(action);
    expect(result.success).toBe(true);
  });

  test("accepts empty payload", async () => {
    const { InsightActionSchema } = await import("@/types/shared");
    const action = {
      label: "Go to file",
      action_type: "navigate",
      payload: {},
    };
    const result = InsightActionSchema.safeParse(action);
    expect(result.success).toBe(true);
  });
});

describe("InsightSchema", () => {
  test("validates complete insight with action", async () => {
    const { InsightSchema } = await import("@/types/shared");
    const insight = {
      id: "insight-1",
      insight_type: "focus-shift",
      title: "Work shifted to auth",
      description: "Significant focus on authentication",
      evidence: ["5 sessions on auth.ts"],
      action: {
        label: "View auth files",
        action_type: "filter",
        payload: { files: ["src/auth.ts"] },
      },
    };
    const result = InsightSchema.safeParse(insight);
    expect(result.success).toBe(true);
  });

  test("validates insight with null action", async () => {
    const { InsightSchema } = await import("@/types/shared");
    const insight = {
      id: "insight-1",
      insight_type: "anomaly",
      title: "Unusual pattern",
      description: "Something unusual",
      evidence: ["Evidence 1"],
      action: null,
    };
    const result = InsightSchema.safeParse(insight);
    expect(result.success).toBe(true);
  });

  test("validates all insight types", async () => {
    const { InsightSchema } = await import("@/types/shared");

    const makeInsight = (type: string) => ({
      id: "test",
      insight_type: type,
      title: "Test",
      description: "Test desc",
      evidence: [],
      action: null,
    });

    expect(InsightSchema.safeParse(makeInsight("focus-shift")).success).toBe(true);
    expect(InsightSchema.safeParse(makeInsight("co-occurrence")).success).toBe(true);
    expect(InsightSchema.safeParse(makeInsight("pending-review")).success).toBe(true);
    expect(InsightSchema.safeParse(makeInsight("anomaly")).success).toBe(true);
    expect(InsightSchema.safeParse(makeInsight("continuity")).success).toBe(true);
    expect(InsightSchema.safeParse(makeInsight("invalid")).success).toBe(false);
  });
});

describe("InsightsRequestSchema", () => {
  test("validates request with chain and file data", async () => {
    const { InsightsRequestSchema } = await import("@/types/shared");
    const request = {
      time_range: "7d",
      chain_data: [
        {
          chain_id: "chain-1",
          name: "Auth refactor",
          session_count: 5,
          file_count: 10,
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
    const result = InsightsRequestSchema.safeParse(request);
    expect(result.success).toBe(true);
  });

  test("accepts null chain name", async () => {
    const { InsightsRequestSchema } = await import("@/types/shared");
    const request = {
      time_range: "24h",
      chain_data: [
        {
          chain_id: "chain-1",
          name: null,
          session_count: 3,
          file_count: 5,
          recent_activity: "1h ago",
        },
      ],
      file_patterns: [],
    };
    const result = InsightsRequestSchema.safeParse(request);
    expect(result.success).toBe(true);
  });

  test("accepts empty arrays", async () => {
    const { InsightsRequestSchema } = await import("@/types/shared");
    const request = {
      time_range: "30d",
      chain_data: [],
      file_patterns: [],
    };
    const result = InsightsRequestSchema.safeParse(request);
    expect(result.success).toBe(true);
  });
});

describe("InsightsResponseSchema", () => {
  test("validates response with insights array", async () => {
    const { InsightsResponseSchema } = await import("@/types/shared");
    const response = {
      insights: [
        {
          id: "insight-1",
          insight_type: "focus-shift",
          title: "Work shifted to auth",
          description: "Significant focus on authentication",
          evidence: ["5 sessions on auth.ts"],
          action: {
            label: "View auth files",
            action_type: "filter",
            payload: { files: ["src/auth.ts"] },
          },
        },
      ],
      model_used: "claude-sonnet-4-5-20250929",
    };
    const result = InsightsResponseSchema.safeParse(response);
    expect(result.success).toBe(true);
  });

  test("validates response with empty insights array", async () => {
    const { InsightsResponseSchema } = await import("@/types/shared");
    const response = {
      insights: [],
      model_used: "sonnet",
    };
    const result = InsightsResponseSchema.safeParse(response);
    expect(result.success).toBe(true);
  });

  test("validates response with null action", async () => {
    const { InsightsResponseSchema } = await import("@/types/shared");
    const response = {
      insights: [
        {
          id: "insight-1",
          insight_type: "anomaly",
          title: "Unusual pattern",
          description: "Something unusual",
          evidence: ["Evidence 1"],
          action: null,
        },
      ],
      model_used: "sonnet",
    };
    const result = InsightsResponseSchema.safeParse(response);
    expect(result.success).toBe(true);
  });
});

describe("SessionSummaryRequestSchema", () => {
  test("validates complete request", async () => {
    const { SessionSummaryRequestSchema } = await import("@/types/shared");
    const request = {
      session_id: "sess-123",
      files: ["src/main.ts", "src/auth.ts"],
      duration_seconds: 3600,
      chain_id: "chain-1",
    };
    const result = SessionSummaryRequestSchema.safeParse(request);
    expect(result.success).toBe(true);
  });

  test("allows null optional fields", async () => {
    const { SessionSummaryRequestSchema } = await import("@/types/shared");
    const request = {
      session_id: "sess-123",
      files: [],
      duration_seconds: null,
      chain_id: null,
    };
    const result = SessionSummaryRequestSchema.safeParse(request);
    expect(result.success).toBe(true);
  });

  test("rejects empty session_id", async () => {
    const { SessionSummaryRequestSchema } = await import("@/types/shared");
    const request = {
      session_id: "",
      files: [],
      duration_seconds: null,
      chain_id: null,
    };
    const result = SessionSummaryRequestSchema.safeParse(request);
    expect(result.success).toBe(false);
  });

  test("rejects negative duration", async () => {
    const { SessionSummaryRequestSchema } = await import("@/types/shared");
    const request = {
      session_id: "sess-123",
      files: [],
      duration_seconds: -1,
      chain_id: null,
    };
    const result = SessionSummaryRequestSchema.safeParse(request);
    expect(result.success).toBe(false);
  });
});

describe("SessionSummaryResponseSchema", () => {
  test("validates complete response", async () => {
    const { SessionSummaryResponseSchema } = await import("@/types/shared");
    const response = {
      session_id: "sess-123",
      summary: "Worked on authentication",
      key_files: ["src/auth.ts"],
      focus_area: "Security",
      model_used: "claude-haiku-4-5-20251001",
    };
    const result = SessionSummaryResponseSchema.safeParse(response);
    expect(result.success).toBe(true);
  });

  test("allows null focus_area", async () => {
    const { SessionSummaryResponseSchema } = await import("@/types/shared");
    const response = {
      session_id: "sess-123",
      summary: "General work",
      key_files: [],
      focus_area: null,
      model_used: "haiku",
    };
    const result = SessionSummaryResponseSchema.safeParse(response);
    expect(result.success).toBe(true);
  });

  test("accepts empty key_files array", async () => {
    const { SessionSummaryResponseSchema } = await import("@/types/shared");
    const response = {
      session_id: "sess-123",
      summary: "Brief session",
      key_files: [],
      focus_area: "Testing",
      model_used: "haiku",
    };
    const result = SessionSummaryResponseSchema.safeParse(response);
    expect(result.success).toBe(true);
  });
});
