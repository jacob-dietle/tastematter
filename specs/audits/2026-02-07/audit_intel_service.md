# Intel Service Audit Report

**Date:** 2026-02-07
**Service:** `@tastematter/intel` v0.1.0
**Path:** `apps/tastematter/intel/`
**Auditor:** claude-opus-4-6 (automated)

---

## 1. Endpoint Inventory

All routes are defined in `src/index.ts`. The Elysia app is exported via `createApp()` for testability.

| # | Method | Path | Request Schema | Response Type | Logging Pattern | Phase |
|---|--------|------|---------------|---------------|-----------------|-------|
| 1 | GET | `/api/intel/health` | None | `HealthResponse` | None (inline return) | Core |
| 2 | POST | `/api/intel/name-chain` | `ChainNamingRequestSchema` | `ChainNamingResponse` | `withOperationLogging` | Phase 2 |
| 3 | POST | `/api/intel/name-chain-ab` | `ChainNamingRequestSchema` | `ABTestResult` | `withOperationLogging` | Phase 5 |
| 4 | POST | `/api/intel/summarize-chain` | `ChainSummaryRequestSchema` | `ChainSummaryResponse` | `withOperationLogging` | Phase 5 |
| 5 | POST | `/api/intel/analyze-commit` | `CommitAnalysisRequestSchema` | `CommitAnalysisResponse` | Manual (inline try/catch) | Phase 4 |
| 6 | POST | `/api/intel/summarize-session` | `SessionSummaryRequestSchema` | `SessionSummaryResponse` | Manual (inline try/catch) | Phase 4 |
| 7 | POST | `/api/intel/generate-insights` | `InsightsRequestSchema` | `InsightsResponse` | Manual (inline try/catch) | Phase 4 |
| 8 | POST | `/api/intel/gitops-decide` | `GitOpsSignalsSchema` | `GitOpsDecision` | Manual (inline try/catch) | GitOps L0 |

**Observations:**
- Endpoints 2-4 use the `withOperationLogging` middleware. Endpoints 5-8 use manual inline try/catch with identical log patterns (start, success, error). This is an inconsistency -- the earlier endpoints were refactored to use the middleware but the later ones were not retrofitted.
- All POST endpoints follow the same pattern: Zod `safeParse` for request validation, 400 on failure, then delegate to agent function.
- The server listens on port 3002 by default (`INTEL_PORT` env override). Rust core runs on 3001.
- `createApp()` is separate from `startServer()` for clean test isolation.

### Shared Infrastructure

- **Correlation ID middleware** (`src/middleware/correlation.ts`): Elysia plugin that reads `X-Correlation-ID` header or generates UUID v4. Propagated to response headers. Available as `correlationId` in handler context.
- **Error classification** (`classifyError` in `src/index.ts`): Maps Anthropic SDK errors to HTTP status codes. Uses duck-typing (`name` + `status` properties) for ESM/CJS compatibility. Maps: 401->AUTHENTICATION_ERROR, 429->RATE_LIMIT_ERROR, 400->BAD_REQUEST, 500/502/503/529->UPSTREAM_ERROR(502), connection errors->SERVICE_UNAVAILABLE(503), unknown->INTERNAL_ERROR(500).
- **Anthropic client**: Lazy singleton via `getAnthropicClient()`. Reads `ANTHROPIC_API_KEY` from environment.

---

## 2. Agent Inventory

All agents live in `src/agents/`. Each follows the identical pattern:
1. Define a system prompt constant
2. Define an Anthropic `Tool` for structured output
3. Export a `buildPrompt()` function
4. Export an async agent function that calls `client.messages.create()` with `tool_choice`
5. Extract `tool_use` block from response
6. Validate with Zod schema before returning

### 2.1 chain-naming.ts

| Property | Value |
|----------|-------|
| **Model** | `claude-haiku-4-5-20251001` |
| **max_tokens** | 256 |
| **Tool name** | `output_chain_name` |
| **Exported functions** | `nameChain`, `nameChainAB`, `buildPrompt`, `buildPromptWithIntent`, `CHAIN_NAMING_TOOL` |
| **Input** | `ChainNamingRequest` (chain_id, files_touched, session_count, recent_sessions, optional enrichment: tools_used, first_user_intent, commit_messages, first_user_message, conversation_excerpt) |
| **Output** | `ChainNamingResponse` (chain_id, generated_name, category, confidence, model_used) |
| **Prompt pattern** | System prompt with naming rules + examples. User prompt with structured INPUT section. Priority signal: `first_user_intent` > file paths. ENRICHMENT section conditionally included. |
| **A/B testing** | `nameChainAB` runs two parallel Haiku calls (first_user_message vs conversation_excerpt) and compares confidence + name length. Returns `ABTestResult` with `QualityComparison`. |

### 2.2 chain-summary.ts

| Property | Value |
|----------|-------|
| **Model** | `claude-haiku-4-5-20251001` |
| **max_tokens** | 512 |
| **Tool name** | `output_chain_summary` |
| **Exported functions** | `summarizeChain`, `buildPrompt`, `CHAIN_SUMMARY_TOOL` |
| **Input** | `ChainSummaryRequest` (chain_id, conversation_excerpt?, files_touched, session_count, duration_seconds, existing_workstreams?) |
| **Output** | `ChainSummaryResponse` (chain_id, summary, accomplishments[], status, key_files[], workstream_tags[], model_used) |
| **Prompt pattern** | Dynamic system prompt that injects existing workstreams for hybrid tagging. Tagging rules: match existing first (source="existing"), generate new semantic tags otherwise (source="generated"). Files truncated at 30 entries. Conversation excerpt truncated at 3000 chars. |

### 2.3 commit-analysis.ts

| Property | Value |
|----------|-------|
| **Model** | `claude-sonnet-4-5-20250929` |
| **max_tokens** | 512 |
| **Tool name** | `output_commit_analysis` |
| **Exported functions** | `analyzeCommit`, `buildPrompt`, `COMMIT_ANALYSIS_TOOL` |
| **Input** | `CommitAnalysisRequest` (commit_hash, message, author, diff, files_changed[]) |
| **Output** | `CommitAnalysisResponse` (commit_hash, is_agent_commit, summary, risk_level, review_focus, related_files[], model_used) |
| **Prompt pattern** | System prompt with rules for agent commit detection (Co-Authored-By signatures, systematic patterns), risk assessment (low/medium/high), and related file inference. Diff included in code fence. |

**Note:** This is the only agent besides `insights` that uses Sonnet instead of Haiku, because it requires reasoning about diffs.

### 2.4 session-summary.ts

| Property | Value |
|----------|-------|
| **Model** | `claude-haiku-4-5-20251001` |
| **max_tokens** | 256 |
| **Tool name** | `output_session_summary` |
| **Exported functions** | `summarizeSession`, `buildPrompt`, `SESSION_SUMMARY_TOOL` |
| **Input** | `SessionSummaryRequest` (session_id, files[], duration_seconds|null, chain_id|null) |
| **Output** | `SessionSummaryResponse` (session_id, summary, key_files[], focus_area|null, model_used) |
| **Prompt pattern** | System prompt with focus area examples. Null focus_area when files are from disparate areas. Minimal input -- files list, duration, chain context. |

### 2.5 insights.ts

| Property | Value |
|----------|-------|
| **Model** | `claude-sonnet-4-5-20250929` |
| **max_tokens** | 1024 |
| **Tool name** | `output_insights` |
| **Exported functions** | `generateInsights`, `buildPrompt`, `INSIGHTS_TOOL` |
| **Input** | `InsightsRequest` (time_range, chain_data[], file_patterns[]) |
| **Output** | `InsightsResponse` (insights[], model_used) where each insight has: id, insight_type, title, description, evidence[], action|null |
| **Prompt pattern** | Most complex agent. System prompt defines 5 insight types (focus-shift, co-occurrence, pending-review, anomaly, continuity). Outputs array of insights, each with optional actionable payload. Uses Sonnet for pattern reasoning. |

### 2.6 gitops-decision.ts

| Property | Value |
|----------|-------|
| **Model** | `claude-3-5-haiku-latest` |
| **max_tokens** | 1024 |
| **Tool name** | `output_decision` |
| **Exported functions** | `decideGitOps`, `buildGitOpsPrompt`, `GITOPS_DECISION_TOOL` |
| **Input** | `GitOpsSignals` (uncommitted_files[], unpushed_commits, current_branch, timestamps, recent_session|null, active_chain|null, user_rules[], time context) |
| **Output** | `GitOpsDecision` (action, reason, urgency, suggested_commit_message|null, files_to_stage|null, coherence_assessment|null, model_used) |
| **Prompt pattern** | Most detailed system prompt with decision framework (commit/push/notify/wait/ask). Coherence assessment rules. User rules take precedence. Prompt built programmatically from signal fields with conditional sections for session/chain/rules context. |

**Note:** This agent uses `claude-3-5-haiku-latest` (not the pinned `claude-haiku-4-5-20251001` used by other Haiku agents). This is an inconsistency -- may get different behavior as `latest` alias shifts.

### Agent Model Summary

| Agent | Model | Tier | max_tokens |
|-------|-------|------|------------|
| chain-naming | `claude-haiku-4-5-20251001` | Haiku (pinned) | 256 |
| chain-summary | `claude-haiku-4-5-20251001` | Haiku (pinned) | 512 |
| session-summary | `claude-haiku-4-5-20251001` | Haiku (pinned) | 256 |
| commit-analysis | `claude-sonnet-4-5-20250929` | Sonnet (pinned) | 512 |
| insights | `claude-sonnet-4-5-20250929` | Sonnet (pinned) | 1024 |
| gitops-decision | `claude-3-5-haiku-latest` | Haiku (**latest alias**) | 1024 |

---

## 3. tool_choice Pattern

Every agent uses the same structured output pattern. This is the core architectural decision of the Intel service.

### How It Works

1. **Define a tool** as an `Anthropic.Tool` object with a JSON Schema `input_schema`.
2. **Force the model to call it** via `tool_choice: { type: "tool", name: "<tool_name>" }`.
3. **Extract the tool_use block** from `response.content` by filtering for `block.type === "tool_use"`.
4. **Validate with Zod** before returning -- ensures runtime type safety even though the model was "forced" to use the tool schema.

### Example (from chain-naming.ts)

```typescript
// 1. Tool definition
export const CHAIN_NAMING_TOOL: Anthropic.Tool = {
  name: "output_chain_name",
  description: "Output the chain naming analysis results",
  input_schema: {
    type: "object",
    properties: {
      generated_name: { type: "string", description: "Short descriptive name (3-6 words)" },
      category: { type: "string", enum: ["bug-fix", "feature", ...] },
      confidence: { type: "number", minimum: 0, maximum: 1 },
    },
    required: ["generated_name", "category", "confidence"],
  },
};

// 2. Force tool call
const response = await client.messages.create({
  model: "claude-haiku-4-5-20251001",
  max_tokens: 256,
  system: CHAIN_NAMING_SYSTEM_PROMPT,
  messages: [{ role: "user", content: buildPrompt(request) }],
  tools: [CHAIN_NAMING_TOOL],
  tool_choice: { type: "tool", name: "output_chain_name" },
});

// 3. Extract tool_use block
const toolUse = response.content.find(
  (block): block is ToolUseBlock => block.type === "tool_use"
);
if (!toolUse) throw new Error("No tool_use block");

// 4. Validate with Zod
return ChainNamingResponseSchema.parse({
  chain_id: request.chain_id,
  ...toolUse.input,
  model_used: "claude-haiku-4-5-20251001",
});
```

### Key Design Decisions

- **No free-form text**: The model MUST call the tool, so there is no text output to parse. This eliminates JSON parsing failures entirely.
- **Double validation**: Tool schema (enforced by API) + Zod schema (enforced in code). Belt and suspenders.
- **Consistent naming convention**: All tool names follow `output_<entity>` pattern.
- **model_used is added server-side**: The model string is not part of the tool output -- it's appended from the constant.

### Pattern Applied Across All Agents

| Agent | Tool Name | Required Fields |
|-------|-----------|----------------|
| chain-naming | `output_chain_name` | generated_name, category, confidence |
| chain-summary | `output_chain_summary` | summary, accomplishments, status, key_files, workstream_tags |
| commit-analysis | `output_commit_analysis` | is_agent_commit, summary, risk_level, review_focus, related_files |
| session-summary | `output_session_summary` | summary, key_files, focus_area |
| insights | `output_insights` | insights (array of objects) |
| gitops-decision | `output_decision` | action, reason, urgency |

---

## 4. Test Coverage

### Test Structure

```
tests/
  unit/
    types/
      shared.test.ts          - Zod schema validation (Phase 2 types)
      new-schemas.test.ts      - Zod schema validation (Phase 4 types)
    agents/
      chain-naming.test.ts     - Tool definition, buildPrompt, nameChain with mocks
      commit-analysis.test.ts  - Tool definition, buildPrompt, analyzeCommit with mocks
      insights.test.ts         - Tool definition, buildPrompt, generateInsights with mocks
      session-summary.test.ts  - Tool definition, buildPrompt, summarizeSession with mocks
    middleware/
      correlation.test.ts      - Elysia middleware correlation ID propagation
      cost-guard.test.ts       - CostGuard class budget tracking
    error-handling.test.ts     - classifyError function (Anthropic SDK error mapping)
    file-logger.test.ts        - FileLogService (writes to temp dir)
    operation-logger.test.ts   - withOperationLogging middleware (captures console output)
  integration/
    health.test.ts             - GET /health endpoint with Elysia app.handle()
    chain-naming.test.ts       - POST /name-chain with mocked Anthropic client
    commit-analysis.test.ts    - POST /analyze-commit with mocked Anthropic client
    insights.test.ts           - POST /generate-insights with mocked Anthropic client
    session-summary.test.ts    - POST /summarize-session with mocked Anthropic client
```

### Test Methodology

- **TDD approach**: All test files have "RED tests" comments, written before implementation.
- **Unit tests**: Mock the Anthropic client with `bun:test` `mock()`. Verify:
  - Tool definitions have correct names and required fields
  - `buildPrompt` includes all input fields
  - Agent functions return valid Zod-parseable responses
  - Agent functions use correct model and tool_choice pattern
  - Error cases (no tool_use block, invalid tool input) throw
- **Integration tests**: Use `createApp()` + Elysia's `app.handle(new Request(...))` for HTTP-level testing. Mock the `@anthropic-ai/sdk` module with `mock.module()` before importing the app.
- **No contract tests in the tree** despite `test:contract` script in package.json.

### Coverage Gaps

| Gap | Description |
|-----|-------------|
| **chain-summary agent** | No unit tests for `summarizeChain`, `buildPrompt`, or `CHAIN_SUMMARY_TOOL` |
| **gitops-decision agent** | No unit tests for `decideGitOps`, `buildGitOpsPrompt`, or `GITOPS_DECISION_TOOL` |
| **name-chain-ab endpoint** | No integration test for the A/B test endpoint |
| **summarize-chain endpoint** | No integration test for chain summary |
| **gitops-decide endpoint** | No integration test for gitops decision |
| **Contract tests** | Script exists (`test:contract`) but no test files in `tests/contract/` |
| **Cost guard integration** | CostGuard is unit tested but never wired into any endpoint |

---

## 5. Dependencies

### package.json

```json
{
  "name": "@tastematter/intel",
  "version": "0.1.0",
  "type": "module",
  "dependencies": {
    "@anthropic-ai/sdk": "^0.32.0",
    "elysia": "^1.1.0",
    "zod": "^3.23.0"
  },
  "devDependencies": {
    "@types/bun": "latest",
    "typescript": "^5.4.0"
  }
}
```

### Framework

- **Runtime**: Bun (not Node.js). Uses `bun:test`, `import.meta.main`, and Bun's built-in TypeScript support.
- **HTTP Framework**: Elysia (Bun-native web framework). Plugin-based middleware via `.use()`.
- **AI SDK**: `@anthropic-ai/sdk` for Claude API calls.
- **Validation**: Zod for request/response schema validation.
- **Logging**: Custom structured JSON logger writing to console + `~/.tastematter/logs/intel-YYYY-MM-DD.jsonl`.

### Build & Run

| Command | Description |
|---------|-------------|
| `bun run dev` | Watch mode (`bun run --watch src/index.ts`) |
| `bun run build` | Compile to single binary (`dist/tastematter-intel`) |
| `bun test` | All tests |
| `bun test tests/unit` | Unit tests only |
| `bun test tests/integration` | Integration tests only |
| `bun run typecheck` | TypeScript type checking (`tsc --noEmit`) |

### Config Files

- `tsconfig.json`: ESNext target, bundler module resolution, strict mode, path alias `@/*` -> `src/*`.
- `bunfig.toml`: Exact versions for installs, 30s test timeout, path alias matching tsconfig.

---

## 6. What synthesize-context Would Need

Based on the established agent patterns, a new context synthesis agent would follow this template:

### Reusable Infrastructure (100% reuse)

1. **Elysia app scaffolding** in `index.ts` -- new `.post()` route
2. **Correlation middleware** -- already global
3. **`withOperationLogging` middleware** -- just add config
4. **`classifyError` function** -- works for any Anthropic error
5. **Zod validation pattern** -- add new request/response schemas to `types/shared.ts`
6. **Structured logger** -- already writes to JSONL
7. **Anthropic client singleton** -- shared

### New Code Required

1. **Schema definitions** in `types/shared.ts`:
   - `ContextSynthesisRequestSchema` -- inputs (session data, chain data, file patterns, time range)
   - `ContextSynthesisResponseSchema` -- output (synthesized context blob, key themes, suggested next actions)

2. **Agent file** `src/agents/context-synthesis.ts` following the template:
   - System prompt defining what "context synthesis" means
   - Tool definition (`output_context_synthesis`) with structured schema
   - `buildPrompt()` function assembling input
   - `synthesizeContext()` async function
   - Model choice: Sonnet if reasoning required, Haiku if just assembly

3. **Route** in `index.ts`:
   - `POST /api/intel/synthesize-context`
   - Zod validation, operation logging, error handling

4. **Tests**:
   - Unit: tool definition, buildPrompt, agent function with mocks
   - Integration: HTTP endpoint with mocked Anthropic

### Estimated Effort

~150-200 lines of new code following the established pattern. The pattern is so well-defined that a new agent is essentially fill-in-the-blanks: system prompt, tool schema, prompt builder, and types.

---

## 7. Gaps and Issues

### 7.1 Inconsistencies

| Issue | Location | Details |
|-------|----------|---------|
| **Mixed logging patterns** | `index.ts:119-460` | Endpoints 2-4 use `withOperationLogging` middleware. Endpoints 5-8 use manual inline try/catch with duplicated logging logic. The middleware was created to eliminate this duplication but wasn't applied retroactively. |
| **Model version inconsistency** | `gitops-decision.ts:20` | Uses `claude-3-5-haiku-latest` (floating alias) while all other Haiku agents use pinned `claude-haiku-4-5-20251001`. This means gitops behavior will silently change when Anthropic updates the alias. |
| **ToolUseBlock interface duplication** | All 6 agent files | Each agent file defines its own `ToolUseBlock` interface with identical `type`, `id`, `name` fields but different `input` types. This could be a shared generic. |

### 7.2 Dead/Unused Code

| Item | Location | Details |
|------|----------|---------|
| **CostGuard middleware** | `src/middleware/cost-guard.ts` | Fully implemented and unit tested, but never imported or used by any endpoint or middleware chain. It's wired up in the file system and has tests, but `index.ts` doesn't reference it. |
| **`getCorrelationId` helper** | `src/middleware/correlation.ts:60` | Exported but never imported anywhere in the codebase. The correlation middleware uses `derive` pattern instead, making this helper unnecessary. |
| **Contract test script** | `package.json:10` | `test:contract` script exists but `tests/contract/` directory does not. |

### 7.3 Missing Tests (listed above in Section 4)

- No unit tests for chain-summary agent
- No unit tests for gitops-decision agent
- No integration tests for 3 endpoints (name-chain-ab, summarize-chain, gitops-decide)
- No contract tests despite script

### 7.4 Potential Issues

| Issue | Severity | Details |
|-------|----------|---------|
| **No cost guard enforcement** | Low | The CostGuard exists but isn't wired in. No protection against runaway API costs. In production this means unlimited spend. |
| **No rate limiting** | Low | No request rate limiting on the Elysia app. The Rust daemon is the only client, but if exposed, could be DoSed. |
| **No input size limits** | Medium | `diff` field in CommitAnalysisRequest has no max length. A massive diff could exceed Sonnet's context window or cause high costs. Chain summary truncates conversation_excerpt at 3000 chars and files at 30 entries, but other agents don't truncate. |
| **Synchronous file logging** | Low | `appendFileSync` in `file-logger.ts` blocks the event loop on every log write. For a service primarily called by a local daemon this is fine, but would be a bottleneck at scale. |
| **No retry logic** | Low | If an Anthropic API call fails, the error is classified and returned immediately. No retry with backoff. The Rust client would need to handle retries. |

### 7.5 Architectural Notes

- The Intel service is a thin HTTP wrapper around Anthropic API calls with structured output. It has no state, no database, and no caching.
- The `createApp()` / `startServer()` separation is clean and enables easy testing.
- The `tool_choice` pattern for structured output is well-established and consistent.
- Type contracts in `types/shared.ts` serve as the interface contract with the Rust side. Comments explicitly note `"MUST match Rust serde serialization"`.

---

## Summary Statistics

| Metric | Count |
|--------|-------|
| Source files | 13 (6 agents, 3 middleware, 2 services, 1 types, 1 index) |
| Test files | 16 (11 unit, 5 integration) |
| Endpoints | 8 (1 GET, 7 POST) |
| Agents | 6 |
| Zod schemas | ~30 (types/shared.ts) |
| Dependencies | 3 runtime, 2 dev |
| Lines of source | ~1,500 (estimated) |
| Lines of tests | ~2,500 (estimated) |
