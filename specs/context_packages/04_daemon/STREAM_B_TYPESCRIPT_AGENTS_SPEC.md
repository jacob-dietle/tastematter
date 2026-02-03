# Stream B: TypeScript Remaining Agents - Complete TDD Specification

**Mission:** Implement 3 remaining AI agents (commit-analysis, session-summary, insights) following the proven chain-naming pattern, plus cost-guard middleware and endpoint integration.

**Methodology:** TDD - Write RED tests first → GREEN implementation → REFACTOR

---

## Prerequisites

Before starting, verify:
- Chain naming agent exists at `intel/src/agents/chain-naming.ts` (126 lines)
- 48 tests currently passing (`bun test`)
- TypeScript service running on port 3002
- Understand `tool_choice` pattern from chain-naming implementation

---

## Critical Reference Files

Read these FIRST before any implementation:

```
apps/tastematter/intel/
├── src/
│   ├── agents/chain-naming.ts      # PATTERN TO FOLLOW - 126 lines
│   ├── types/shared.ts             # ADD new schemas here
│   ├── middleware/correlation.ts   # Existing middleware pattern
│   └── index.ts                    # ADD new endpoints here
├── tests/
│   ├── unit/agents/chain-naming.test.ts  # TEST PATTERN TO FOLLOW
│   └── integration/chain-naming.test.ts  # INTEGRATION PATTERN
```

---

## Files to Create/Modify

```
apps/tastematter/intel/
├── src/
│   ├── types/shared.ts           # MODIFY: Add 10 new schemas
│   ├── agents/
│   │   ├── chain-naming.ts       # EXISTS ✅
│   │   ├── commit-analysis.ts    # NEW
│   │   ├── session-summary.ts    # NEW
│   │   └── insights.ts           # NEW
│   ├── middleware/
│   │   ├── correlation.ts        # EXISTS ✅
│   │   └── cost-guard.ts         # NEW
│   ├── services/
│   │   └── logger.ts             # NEW
│   └── index.ts                  # MODIFY: Add 3 endpoints
└── tests/
    ├── unit/
    │   ├── types/new-schemas.test.ts    # NEW
    │   └── agents/
    │       ├── chain-naming.test.ts     # EXISTS ✅
    │       ├── commit-analysis.test.ts  # NEW
    │       ├── session-summary.test.ts  # NEW
    │       └── insights.test.ts         # NEW
    └── integration/
        ├── chain-naming.test.ts         # EXISTS ✅
        ├── commit-analysis.test.ts      # NEW
        ├── session-summary.test.ts      # NEW
        └── insights.test.ts             # NEW
```

---

## TDD Execution Order

### Cycle 1: Add Zod Schemas (RED → GREEN)

**File:** `intel/src/types/shared.ts`

**RED Tests First:** Create `tests/unit/types/new-schemas.test.ts`
```typescript
import { describe, test, expect } from "bun:test";
import {
  RiskLevelSchema,
  InsightTypeSchema,
  ActionTypeSchema,
  CommitAnalysisRequestSchema,
  CommitAnalysisResponseSchema,
  InsightSchema,
  InsightsRequestSchema,
  InsightsResponseSchema,
  SessionSummaryRequestSchema,
  SessionSummaryResponseSchema,
} from "@/types/shared";

describe("New Zod Schemas", () => {
  describe("RiskLevelSchema", () => {
    test("accepts valid risk levels", () => {
      expect(RiskLevelSchema.parse("low")).toBe("low");
      expect(RiskLevelSchema.parse("medium")).toBe("medium");
      expect(RiskLevelSchema.parse("high")).toBe("high");
    });

    test("rejects invalid risk levels", () => {
      expect(() => RiskLevelSchema.parse("critical")).toThrow();
    });
  });

  describe("InsightTypeSchema", () => {
    test("accepts all insight types", () => {
      const types = ["focus-shift", "co-occurrence", "pending-review", "anomaly", "continuity"];
      types.forEach(t => expect(InsightTypeSchema.parse(t)).toBe(t));
    });
  });

  describe("CommitAnalysisRequestSchema", () => {
    test("validates complete request", () => {
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

    test("rejects missing fields", () => {
      const result = CommitAnalysisRequestSchema.safeParse({ commit_hash: "abc" });
      expect(result.success).toBe(false);
    });
  });

  describe("CommitAnalysisResponseSchema", () => {
    test("validates complete response", () => {
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
  });

  describe("InsightsRequestSchema", () => {
    test("validates request with chain and file data", () => {
      const request = {
        time_range: "7d",
        chain_data: [{
          chain_id: "chain-1",
          name: "Auth refactor",
          session_count: 5,
          file_count: 10,
          recent_activity: "2h ago",
        }],
        file_patterns: [{
          file_path: "src/auth.ts",
          access_count: 15,
          co_accessed_with: ["src/login.ts"],
        }],
      };
      const result = InsightsRequestSchema.safeParse(request);
      expect(result.success).toBe(true);
    });
  });

  describe("InsightsResponseSchema", () => {
    test("validates response with insights array", () => {
      const response = {
        insights: [{
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
        }],
        model_used: "claude-sonnet-4-5-20250929",
      };
      const result = InsightsResponseSchema.safeParse(response);
      expect(result.success).toBe(true);
    });

    test("validates response with null action", () => {
      const response = {
        insights: [{
          id: "insight-1",
          insight_type: "anomaly",
          title: "Unusual pattern",
          description: "Something unusual",
          evidence: ["Evidence 1"],
          action: null,
        }],
        model_used: "sonnet",
      };
      const result = InsightsResponseSchema.safeParse(response);
      expect(result.success).toBe(true);
    });
  });

  describe("SessionSummaryRequestSchema", () => {
    test("validates complete request", () => {
      const request = {
        session_id: "sess-123",
        files: ["src/main.ts", "src/auth.ts"],
        duration_seconds: 3600,
        chain_id: "chain-1",
      };
      const result = SessionSummaryRequestSchema.safeParse(request);
      expect(result.success).toBe(true);
    });

    test("allows null optional fields", () => {
      const request = {
        session_id: "sess-123",
        files: [],
        duration_seconds: null,
        chain_id: null,
      };
      const result = SessionSummaryRequestSchema.safeParse(request);
      expect(result.success).toBe(true);
    });
  });

  describe("SessionSummaryResponseSchema", () => {
    test("validates complete response", () => {
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
  });
});
```

**GREEN Implementation:** Add to `intel/src/types/shared.ts`:
```typescript
// === Risk & Insight Enums ===

export const RiskLevelSchema = z.enum(["low", "medium", "high"]);
export type RiskLevel = z.infer<typeof RiskLevelSchema>;

export const InsightTypeSchema = z.enum([
  "focus-shift",
  "co-occurrence",
  "pending-review",
  "anomaly",
  "continuity",
]);
export type InsightType = z.infer<typeof InsightTypeSchema>;

export const ActionTypeSchema = z.enum(["navigate", "filter", "external"]);
export type ActionType = z.infer<typeof ActionTypeSchema>;

// === Commit Analysis ===

export const CommitAnalysisRequestSchema = z.object({
  commit_hash: z.string().min(1),
  message: z.string(),
  author: z.string(),
  diff: z.string(),
  files_changed: z.array(z.string()),
});
export type CommitAnalysisRequest = z.infer<typeof CommitAnalysisRequestSchema>;

export const CommitAnalysisResponseSchema = z.object({
  commit_hash: z.string(),
  is_agent_commit: z.boolean(),
  summary: z.string(),
  risk_level: RiskLevelSchema,
  review_focus: z.string(),
  related_files: z.array(z.string()),
  model_used: z.string(),
});
export type CommitAnalysisResponse = z.infer<typeof CommitAnalysisResponseSchema>;

// === Insights ===

export const InsightActionSchema = z.object({
  label: z.string(),
  action_type: ActionTypeSchema,
  payload: z.record(z.unknown()),
});
export type InsightAction = z.infer<typeof InsightActionSchema>;

export const InsightSchema = z.object({
  id: z.string(),
  insight_type: InsightTypeSchema,
  title: z.string(),
  description: z.string(),
  evidence: z.array(z.string()),
  action: InsightActionSchema.nullable(),
});
export type Insight = z.infer<typeof InsightSchema>;

export const ChainDataSchema = z.object({
  chain_id: z.string(),
  name: z.string().nullable(),
  session_count: z.number().int(),
  file_count: z.number().int(),
  recent_activity: z.string(),
});

export const FilePatternSchema = z.object({
  file_path: z.string(),
  access_count: z.number().int(),
  co_accessed_with: z.array(z.string()),
});

export const InsightsRequestSchema = z.object({
  time_range: z.string(),
  chain_data: z.array(ChainDataSchema),
  file_patterns: z.array(FilePatternSchema),
});
export type InsightsRequest = z.infer<typeof InsightsRequestSchema>;

export const InsightsResponseSchema = z.object({
  insights: z.array(InsightSchema),
  model_used: z.string(),
});
export type InsightsResponse = z.infer<typeof InsightsResponseSchema>;

// === Session Summary ===

export const SessionSummaryRequestSchema = z.object({
  session_id: z.string().min(1),
  files: z.array(z.string()),
  duration_seconds: z.number().int().nullable(),
  chain_id: z.string().nullable(),
});
export type SessionSummaryRequest = z.infer<typeof SessionSummaryRequestSchema>;

export const SessionSummaryResponseSchema = z.object({
  session_id: z.string(),
  summary: z.string(),
  key_files: z.array(z.string()),
  focus_area: z.string().nullable(),
  model_used: z.string(),
});
export type SessionSummaryResponse = z.infer<typeof SessionSummaryResponseSchema>;
```

---

### Cycle 2: Commit Analysis Agent (RED → GREEN)

**File:** `intel/src/agents/commit-analysis.ts`

**RED Tests First:** Create `tests/unit/agents/commit-analysis.test.ts`
```typescript
import { describe, test, expect, mock } from "bun:test";
import type { CommitAnalysisRequest } from "@/types/shared";
import { CommitAnalysisResponseSchema } from "@/types/shared";

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
    });

    test("COMMIT_ANALYSIS_TOOL risk_level enum is correct", async () => {
      const { COMMIT_ANALYSIS_TOOL } = await import("@/agents/commit-analysis");
      const schema = COMMIT_ANALYSIS_TOOL.input_schema as unknown as {
        properties: { risk_level: { enum: string[] } };
      };
      expect(schema.properties.risk_level.enum).toEqual(["low", "medium", "high"]);
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

    test("buildPrompt includes diff", async () => {
      const { buildPrompt } = await import("@/agents/commit-analysis");
      const request: CommitAnalysisRequest = {
        commit_hash: "test",
        message: "Fix",
        author: "dev",
        diff: "+++ new line\n--- old line",
        files_changed: ["src/main.ts"],
      };
      const prompt = buildPrompt(request);
      expect(prompt).toContain("+++ new line");
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
  });
});
```

**GREEN Implementation:** Create `intel/src/agents/commit-analysis.ts`
```typescript
import Anthropic from "@anthropic-ai/sdk";
import {
  CommitAnalysisRequest,
  CommitAnalysisResponse,
  CommitAnalysisResponseSchema,
} from "@/types/shared";

const MODEL = "claude-sonnet-4-5-20250929";

const COMMIT_ANALYSIS_SYSTEM_PROMPT = `You are a commit analysis expert. Your job is to analyze git commits and determine:
1. Whether it was made by an AI agent (Claude, Copilot, etc.) based on patterns
2. A brief summary of what the commit does
3. Risk level for code review (low/medium/high)
4. What the reviewer should focus on
5. Related files that might be affected but weren't changed

Rules:
1. Be concise in summaries (1-2 sentences max)
2. Risk assessment: low = routine changes, medium = logic changes, high = security/data/critical path
3. Agent commits often have patterns: "Co-Authored-By: Claude", systematic refactors, generated code
4. You MUST use the output_commit_analysis tool to provide your response`;

export const COMMIT_ANALYSIS_TOOL: Anthropic.Tool = {
  name: "output_commit_analysis",
  description: "Output the commit analysis results",
  input_schema: {
    type: "object",
    properties: {
      is_agent_commit: {
        type: "boolean",
        description: "Whether this commit appears to be made by an AI agent",
      },
      summary: {
        type: "string",
        description: "Brief summary of what the commit does (1-2 sentences)",
      },
      risk_level: {
        type: "string",
        enum: ["low", "medium", "high"],
        description: "Risk level for code review",
      },
      review_focus: {
        type: "string",
        description: "What the reviewer should focus on",
      },
      related_files: {
        type: "array",
        items: { type: "string" },
        description: "Files that might be affected but weren't changed",
      },
    },
    required: ["is_agent_commit", "summary", "risk_level", "review_focus", "related_files"],
  },
};

export function buildPrompt(request: CommitAnalysisRequest): string {
  return `Analyze the following git commit. Use the output_commit_analysis tool to provide your response.

INPUT:
Commit Hash: ${request.commit_hash}
Author: ${request.author}
Message: ${request.message}

Files Changed:
${request.files_changed.map((f) => `- ${f}`).join("\n") || "(none)"}

Diff:
\`\`\`
${request.diff}
\`\`\`

Analyze this commit and use the output_commit_analysis tool.`;
}

export async function analyzeCommit(
  client: Anthropic,
  request: CommitAnalysisRequest
): Promise<CommitAnalysisResponse> {
  const response = await client.messages.create({
    model: MODEL,
    max_tokens: 512,
    system: COMMIT_ANALYSIS_SYSTEM_PROMPT,
    messages: [{ role: "user", content: buildPrompt(request) }],
    tools: [COMMIT_ANALYSIS_TOOL],
    tool_choice: { type: "tool", name: "output_commit_analysis" },
  });

  const toolUse = response.content.find((c) => c.type === "tool_use");
  if (!toolUse || toolUse.type !== "tool_use") {
    throw new Error("No tool_use in response");
  }

  const input = toolUse.input as Record<string, unknown>;
  return CommitAnalysisResponseSchema.parse({
    commit_hash: request.commit_hash,
    is_agent_commit: input.is_agent_commit,
    summary: input.summary,
    risk_level: input.risk_level,
    review_focus: input.review_focus,
    related_files: input.related_files,
    model_used: MODEL,
  });
}
```

---

### Cycle 3: Session Summary Agent (RED → GREEN)

**File:** `intel/src/agents/session-summary.ts`

Follow the same pattern as commit-analysis:
- `SESSION_SUMMARY_TOOL` with tool_choice
- Model: `claude-haiku-4-5-20251001` (fast/cheap)
- `buildPrompt(request)` includes session_id, files, duration
- `summarizeSession(client, request)` returns SessionSummaryResponse

**Key differences from commit-analysis:**
- Uses Haiku (cheaper) since summarization is simpler
- Outputs: summary, key_files, focus_area

---

### Cycle 4: Insights Agent (RED → GREEN)

**File:** `intel/src/agents/insights.ts`

Most complex agent - outputs multiple insights:
- `INSIGHTS_TOOL` with array of insights
- Model: `claude-sonnet-4-5-20250929` (needs reasoning)
- `buildPrompt(request)` includes chain_data, file_patterns, time_range
- `generateInsights(client, request)` returns InsightsResponse

**Key differences:**
- Tool schema has `insights` array property
- Each insight has id, type, title, description, evidence, action
- More complex prompt with pattern detection instructions

---

### Cycle 5: Logger Service (GREEN only - utility)

**File:** `intel/src/services/logger.ts`

```typescript
export const log = {
  info: (event: Record<string, unknown>) => {
    console.log(JSON.stringify({
      level: "info",
      timestamp: new Date().toISOString(),
      ...event,
    }));
  },
  error: (event: Record<string, unknown>) => {
    console.error(JSON.stringify({
      level: "error",
      timestamp: new Date().toISOString(),
      ...event,
    }));
  },
  warn: (event: Record<string, unknown>) => {
    console.warn(JSON.stringify({
      level: "warn",
      timestamp: new Date().toISOString(),
      ...event,
    }));
  },
};
```

---

### Cycle 6: Cost Guard Middleware (RED → GREEN)

**File:** `intel/src/middleware/cost-guard.ts`

**RED Tests First:**
```typescript
describe("Cost Guard Middleware", () => {
  test("allows request when under daily budget", async () => {
    const { CostGuard } = await import("@/middleware/cost-guard");
    const guard = new CostGuard({ dailyBudgetUsd: 1.0 });
    expect(guard.canProceed("haiku", 0.00025)).toBe(true);
  });

  test("blocks request when over daily budget", async () => {
    const { CostGuard } = await import("@/middleware/cost-guard");
    const guard = new CostGuard({ dailyBudgetUsd: 0.001 });
    guard.recordCost("haiku", 0.001);
    expect(guard.canProceed("haiku", 0.00025)).toBe(false);
  });

  test("tracks costs by operation", async () => {
    const { CostGuard } = await import("@/middleware/cost-guard");
    const guard = new CostGuard({ dailyBudgetUsd: 1.0 });
    guard.recordCost("haiku", 0.001);
    guard.recordCost("sonnet", 0.003);
    expect(guard.getTodaySpend()).toBeCloseTo(0.004, 5);
  });
});
```

**GREEN Implementation:**
```typescript
interface CostGuardConfig {
  dailyBudgetUsd: number;
}

interface CostRecord {
  timestamp: Date;
  model: string;
  cost_usd: number;
}

export class CostGuard {
  private config: CostGuardConfig;
  private records: CostRecord[] = [];

  constructor(config: CostGuardConfig) {
    this.config = config;
  }

  canProceed(model: string, estimatedCost: number): boolean {
    const todaySpend = this.getTodaySpend();
    return todaySpend + estimatedCost <= this.config.dailyBudgetUsd;
  }

  recordCost(model: string, cost_usd: number): void {
    this.records.push({ timestamp: new Date(), model, cost_usd });
  }

  getTodaySpend(): number {
    const today = new Date();
    today.setUTCHours(0, 0, 0, 0);
    return this.records
      .filter((r) => r.timestamp >= today)
      .reduce((sum, r) => sum + r.cost_usd, 0);
  }
}
```

---

### Cycle 7: Add Endpoints to index.ts

**MODIFY:** `intel/src/index.ts`

Add imports and endpoints following the chain-naming pattern:

```typescript
// Add imports
import { analyzeCommit } from "@/agents/commit-analysis";
import { summarizeSession } from "@/agents/session-summary";
import { generateInsights } from "@/agents/insights";
import {
  CommitAnalysisRequestSchema,
  SessionSummaryRequestSchema,
  InsightsRequestSchema,
} from "@/types/shared";

// Add endpoint: POST /api/intel/analyze-commit
.post("/api/intel/analyze-commit", async ({ body, set, request }) => {
  const correlationId = request.headers.get("X-Correlation-ID") || crypto.randomUUID();

  const validation = CommitAnalysisRequestSchema.safeParse(body);
  if (!validation.success) {
    set.status = 400;
    return { error: "Invalid request", details: validation.error.issues };
  }

  const result = await analyzeCommit(getAnthropicClient(), validation.data);
  set.headers["X-Correlation-ID"] = correlationId;
  return result;
})

// Add endpoint: POST /api/intel/summarize-session
.post("/api/intel/summarize-session", async ({ body, set, request }) => {
  // Same pattern
})

// Add endpoint: POST /api/intel/generate-insights
.post("/api/intel/generate-insights", async ({ body, set, request }) => {
  // Same pattern
})
```

---

## Observability Requirements

### Log Event Schema

All agent calls MUST log structured events:

```typescript
import { log } from "@/services/logger";

// Request start
log.info({
  correlation_id: id,
  operation: "analyze_commit",
  commit_hash: request.commit_hash,
  files_count: request.files_changed.length,
  message: "Starting commit analysis",
});

// Request complete
log.info({
  correlation_id: id,
  operation: "analyze_commit",
  duration_ms: duration,
  success: true,
  is_agent_commit: response.is_agent_commit,
  risk_level: response.risk_level,
  model_used: response.model_used,
  message: "Commit analysis completed",
});

// Request failed
log.error({
  correlation_id: id,
  operation: "analyze_commit",
  duration_ms: duration,
  error: error.message,
  message: "Commit analysis failed",
});
```

---

## Completion Criteria

- [ ] 10 new Zod schemas in shared.ts with tests (12+ tests)
- [ ] commit-analysis.ts agent with TDD (10+ tests)
- [ ] session-summary.ts agent with TDD (8+ tests)
- [ ] insights.ts agent with TDD (10+ tests)
- [ ] cost-guard.ts middleware with tests (4+ tests)
- [ ] 3 new endpoints in index.ts
- [ ] Logger service implemented
- [ ] All integration tests passing
- [ ] `bun test` passes (~78 tests total)
- [ ] `bun run typecheck` clean

---

## Verification Commands

```bash
cd apps/tastematter/intel

# Run all tests
bun test

# Run just new schema tests
bun test tests/unit/types/new-schemas.test.ts

# Run agent tests
bun test tests/unit/agents/

# Run integration tests
bun test tests/integration/

# Typecheck
bun run typecheck

# Start dev server
bun run dev

# Test endpoints (requires ANTHROPIC_API_KEY)
curl -X POST http://localhost:3002/api/intel/analyze-commit \
  -H "Content-Type: application/json" \
  -d '{"commit_hash":"abc123","message":"Fix bug","author":"dev","diff":"---","files_changed":["src/main.ts"]}'

curl -X POST http://localhost:3002/api/intel/summarize-session \
  -H "Content-Type: application/json" \
  -d '{"session_id":"sess123","files":["src/auth.ts"],"duration_seconds":3600,"chain_id":null}'

curl -X POST http://localhost:3002/api/intel/generate-insights \
  -H "Content-Type: application/json" \
  -d '{"time_range":"7d","chain_data":[],"file_patterns":[]}'
```

---

## Do NOT

- Do NOT modify Rust code (Stream A handles that)
- Do NOT modify chain-naming.ts (it's the reference, already complete)
- Do NOT implement build pipeline (Phase 5)
- Do NOT implement E2E/parity tests (Phase 6)
- Do NOT implement cost tracking persistence (cost-guard is in-memory only for MVP)
