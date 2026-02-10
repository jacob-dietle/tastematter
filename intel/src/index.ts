/**
 * Tastematter Intelligence Service
 *
 * TypeScript + Elysia HTTP server providing AI-powered intelligence
 * for chain naming, commit analysis, session summaries, and insights.
 *
 * Port: 3002 (Rust core runs on 3001)
 *
 * Endpoints:
 * - GET  /api/intel/health           - Health check
 * - POST /api/intel/name-chain       - Generate chain names (Phase 2)
 * - POST /api/intel/name-chain-ab    - A/B test chain naming quality (Phase 5)
 * - POST /api/intel/summarize-chain  - Generate chain summary with workstream tags (Phase 5)
 * - POST /api/intel/analyze-commit   - Analyze commits (Phase 4)
 * - POST /api/intel/summarize-session - Summarize sessions (Phase 4)
 * - POST /api/intel/generate-insights - Generate insights (Phase 4)
 * - POST /api/intel/gitops-decide    - Intelligent GitOps decisions (GitOps Level 0)
 * - POST /api/intel/synthesize-context - Context synthesis for restore Phase 2
 */

import { Elysia } from "elysia";
import Anthropic from "@anthropic-ai/sdk";
import { correlationMiddleware } from "./middleware/correlation";
import { withOperationLogging } from "./middleware/operation-logger";
import { nameChain, nameChainAB } from "./agents/chain-naming";
import { analyzeCommit } from "./agents/commit-analysis";
import { summarizeSession } from "./agents/session-summary";
import { generateInsights } from "./agents/insights";
import { summarizeChain } from "./agents/chain-summary";
import { decideGitOps } from "./agents/gitops-decision";
import { synthesizeContext } from "./agents/context-synthesis";
import { log } from "./services/logger";
import {
  ChainNamingRequestSchema,
  ChainSummaryRequestSchema,
  CommitAnalysisRequestSchema,
  SessionSummaryRequestSchema,
  InsightsRequestSchema,
  GitOpsSignalsSchema,
  ContextSynthesisRequestSchema,
  type HealthResponse,
  type ChainNamingResponse,
  type ABTestResult,
  type ChainSummaryResponse,
  type CommitAnalysisResponse,
  type SessionSummaryResponse,
  type InsightsResponse,
  type GitOpsDecision,
  type ContextSynthesisResponse,
} from "./types/shared";

const VERSION = "0.1.0";
const DEFAULT_PORT = 3002;

/**
 * Classify Anthropic SDK errors into appropriate HTTP status codes.
 * Enables graceful degradation - Rust client can distinguish error types.
 */
export function classifyError(error: unknown): { status: number; code: string } {
  // Check by error name (robust across ESM/CJS)
  if (error && typeof error === "object" && "name" in error) {
    const name = (error as { name: string }).name;

    // Connection errors (no HTTP status)
    if (name === "APIConnectionError" || name === "APIConnectionTimeoutError") {
      return { status: 503, code: "SERVICE_UNAVAILABLE" };
    }

    // API errors with status codes
    if ("status" in error && typeof (error as { status: unknown }).status === "number") {
      const status = (error as { status: number }).status;
      switch (status) {
        case 401:
          return { status: 401, code: "AUTHENTICATION_ERROR" };
        case 429:
          return { status: 429, code: "RATE_LIMIT_ERROR" };
        case 400:
          return { status: 400, code: "BAD_REQUEST" };
        case 500:
        case 502:
        case 503:
        case 529: // Anthropic overloaded
          return { status: 502, code: "UPSTREAM_ERROR" };
      }
    }
  }

  return { status: 500, code: "INTERNAL_ERROR" };
}

// Initialize Anthropic client (lazy - only when needed)
let anthropicClient: Anthropic | null = null;

function getAnthropicClient(): Anthropic {
  if (!anthropicClient) {
    const apiKey = process.env.ANTHROPIC_API_KEY;
    if (!apiKey) {
      throw new Error(
        "ANTHROPIC_API_KEY not set. Create apps/tastematter/intel/.env with your API key."
      );
    }
    anthropicClient = new Anthropic({ apiKey });
  }
  return anthropicClient;
}

/**
 * Create the Elysia application
 * Exported for testing
 */
export function createApp() {
  return new Elysia()
    // Apply correlation ID middleware globally
    .use(correlationMiddleware())

    // Health endpoint
    .get("/api/intel/health", (): HealthResponse => ({
      status: "ok",
      version: VERSION,
    }))

    // Chain naming endpoint (Phase 2) - with operation logging
    .post("/api/intel/name-chain", async ({ body, set, correlationId }) => {
      // Validate request body
      const validation = ChainNamingRequestSchema.safeParse(body);
      if (!validation.success) {
        set.status = 400;
        return {
          error: "Invalid request",
          details: validation.error.flatten().fieldErrors,
        };
      }

      // Use operation logging middleware for consistent observability
      return withOperationLogging(
        {
          operation: "name_chain",
          getInputMetrics: () => ({
            chain_id: validation.data.chain_id,
            files_count: validation.data.files_touched.length,
            session_count: validation.data.session_count,
          }),
          getOutputMetrics: (result) => ({
            generated_name: (result as ChainNamingResponse).generated_name,
            category: (result as ChainNamingResponse).category,
            confidence: (result as ChainNamingResponse).confidence,
            model_used: (result as ChainNamingResponse).model_used,
          }),
        },
        async () => {
          const client = getAnthropicClient();
          return await nameChain(client, validation.data);
        }
      )({ correlationId, body, set });
    })

    // Chain naming A/B test endpoint (Phase 5)
    .post("/api/intel/name-chain-ab", async ({ body, set, correlationId }) => {
      // Validate request body
      const validation = ChainNamingRequestSchema.safeParse(body);
      if (!validation.success) {
        set.status = 400;
        return {
          error: "Invalid request",
          details: validation.error.flatten().fieldErrors,
        };
      }

      // Use operation logging middleware for consistent observability
      return withOperationLogging(
        {
          operation: "name_chain_ab",
          getInputMetrics: () => ({
            chain_id: validation.data.chain_id,
            files_count: validation.data.files_touched.length,
            session_count: validation.data.session_count,
            has_first_message: !!validation.data.first_user_message,
            has_full_excerpt: !!validation.data.conversation_excerpt,
          }),
          getOutputMetrics: (result) => ({
            first_message_name: (result as ABTestResult).first_message_result.generated_name,
            full_excerpt_name: (result as ABTestResult).full_excerpt_result.generated_name,
            winner: (result as ABTestResult).quality_comparison.winner,
            confidence_delta: (result as ABTestResult).quality_comparison.confidence_delta,
          }),
        },
        async () => {
          const client = getAnthropicClient();
          return await nameChainAB(client, validation.data);
        }
      )({ correlationId, body, set });
    })

    // Chain summary endpoint (Phase 5) - with workstream tagging
    .post("/api/intel/summarize-chain", async ({ body, set, correlationId }) => {
      // Validate request body
      const validation = ChainSummaryRequestSchema.safeParse(body);
      if (!validation.success) {
        set.status = 400;
        return {
          error: "Invalid request",
          details: validation.error.flatten().fieldErrors,
        };
      }

      // Use operation logging middleware for consistent observability
      return withOperationLogging(
        {
          operation: "summarize_chain",
          getInputMetrics: () => ({
            chain_id: validation.data.chain_id,
            files_count: validation.data.files_touched.length,
            session_count: validation.data.session_count,
            has_excerpt: !!validation.data.conversation_excerpt,
            workstream_count: validation.data.existing_workstreams?.length ?? 0,
          }),
          getOutputMetrics: (result) => ({
            status: (result as ChainSummaryResponse).status,
            accomplishments_count: (result as ChainSummaryResponse).accomplishments.length,
            workstream_tags: (result as ChainSummaryResponse).workstream_tags.map(t => t.tag),
            model_used: (result as ChainSummaryResponse).model_used,
          }),
        },
        async () => {
          const client = getAnthropicClient();
          return await summarizeChain(client, validation.data);
        }
      )({ correlationId, body, set });
    })

    // Commit analysis endpoint (Phase 4)
    .post("/api/intel/analyze-commit", async ({ body, set, correlationId }) => {
      const startTime = Date.now();

      // Validate request body
      const validation = CommitAnalysisRequestSchema.safeParse(body);
      if (!validation.success) {
        set.status = 400;
        return {
          error: "Invalid request",
          details: validation.error.flatten().fieldErrors,
        };
      }

      log.info({
        correlation_id: correlationId,
        operation: "analyze_commit",
        commit_hash: validation.data.commit_hash,
        files_count: validation.data.files_changed.length,
        message: "Starting commit analysis",
      });

      try {
        const client = getAnthropicClient();
        const result = await analyzeCommit(client, validation.data);

        log.info({
          correlation_id: correlationId,
          operation: "analyze_commit",
          duration_ms: Date.now() - startTime,
          success: true,
          is_agent_commit: result.is_agent_commit,
          risk_level: result.risk_level,
          model_used: result.model_used,
          message: "Commit analysis completed",
        });

        return result as CommitAnalysisResponse;
      } catch (error) {
        const { status, code } = classifyError(error);
        log.error({
          correlation_id: correlationId,
          operation: "analyze_commit",
          duration_ms: Date.now() - startTime,
          error: error instanceof Error ? error.message : "Unknown error",
          error_code: code,
          message: "Commit analysis failed",
        });

        set.status = status;
        return {
          error: "Commit analysis failed",
          code,
          message: error instanceof Error ? error.message : "Unknown error",
        };
      }
    })

    // Session summary endpoint (Phase 4)
    .post("/api/intel/summarize-session", async ({ body, set, correlationId }) => {
      const startTime = Date.now();

      // Validate request body
      const validation = SessionSummaryRequestSchema.safeParse(body);
      if (!validation.success) {
        set.status = 400;
        return {
          error: "Invalid request",
          details: validation.error.flatten().fieldErrors,
        };
      }

      log.info({
        correlation_id: correlationId,
        operation: "summarize_session",
        session_id: validation.data.session_id,
        files_count: validation.data.files.length,
        message: "Starting session summary",
      });

      try {
        const client = getAnthropicClient();
        const result = await summarizeSession(client, validation.data);

        log.info({
          correlation_id: correlationId,
          operation: "summarize_session",
          duration_ms: Date.now() - startTime,
          success: true,
          focus_area: result.focus_area,
          model_used: result.model_used,
          message: "Session summary completed",
        });

        return result as SessionSummaryResponse;
      } catch (error) {
        const { status, code } = classifyError(error);
        log.error({
          correlation_id: correlationId,
          operation: "summarize_session",
          duration_ms: Date.now() - startTime,
          error: error instanceof Error ? error.message : "Unknown error",
          error_code: code,
          message: "Session summary failed",
        });

        set.status = status;
        return {
          error: "Session summary failed",
          code,
          message: error instanceof Error ? error.message : "Unknown error",
        };
      }
    })

    // Insights endpoint (Phase 4)
    .post("/api/intel/generate-insights", async ({ body, set, correlationId }) => {
      const startTime = Date.now();

      // Validate request body
      const validation = InsightsRequestSchema.safeParse(body);
      if (!validation.success) {
        set.status = 400;
        return {
          error: "Invalid request",
          details: validation.error.flatten().fieldErrors,
        };
      }

      log.info({
        correlation_id: correlationId,
        operation: "generate_insights",
        time_range: validation.data.time_range,
        chain_count: validation.data.chain_data.length,
        file_pattern_count: validation.data.file_patterns.length,
        message: "Starting insights generation",
      });

      try {
        const client = getAnthropicClient();
        const result = await generateInsights(client, validation.data);

        log.info({
          correlation_id: correlationId,
          operation: "generate_insights",
          duration_ms: Date.now() - startTime,
          success: true,
          insights_count: result.insights.length,
          model_used: result.model_used,
          message: "Insights generation completed",
        });

        return result as InsightsResponse;
      } catch (error) {
        const { status, code } = classifyError(error);
        log.error({
          correlation_id: correlationId,
          operation: "generate_insights",
          duration_ms: Date.now() - startTime,
          error: error instanceof Error ? error.message : "Unknown error",
          error_code: code,
          message: "Insights generation failed",
        });

        set.status = status;
        return {
          error: "Insights generation failed",
          code,
          message: error instanceof Error ? error.message : "Unknown error",
        };
      }
    })

    // GitOps decision endpoint (Intelligent GitOps Level 0)
    .post("/api/intel/gitops-decide", async ({ body, set, correlationId }) => {
      const startTime = Date.now();

      // Validate request body
      const validation = GitOpsSignalsSchema.safeParse(body);
      if (!validation.success) {
        set.status = 400;
        return {
          error: "Invalid request",
          details: validation.error.flatten().fieldErrors,
        };
      }

      log.info({
        correlation_id: correlationId,
        operation: "gitops_decide",
        uncommitted_count: validation.data.uncommitted_files.length,
        unpushed_count: validation.data.unpushed_commits,
        has_session_context: validation.data.recent_session !== null,
        has_chain_context: validation.data.active_chain !== null,
        rules_count: validation.data.user_rules.length,
        message: "Starting GitOps decision",
      });

      try {
        const client = getAnthropicClient();
        const result = await decideGitOps(client, validation.data);

        log.info({
          correlation_id: correlationId,
          operation: "gitops_decide",
          duration_ms: Date.now() - startTime,
          success: true,
          action: result.action,
          urgency: result.urgency,
          has_commit_message: result.suggested_commit_message !== null,
          model_used: result.model_used,
          message: "GitOps decision completed",
        });

        return result as GitOpsDecision;
      } catch (error) {
        const { status, code } = classifyError(error);
        log.error({
          correlation_id: correlationId,
          operation: "gitops_decide",
          duration_ms: Date.now() - startTime,
          error: error instanceof Error ? error.message : "Unknown error",
          error_code: code,
          message: "GitOps decision failed",
        });

        set.status = status;
        return {
          error: "GitOps decision failed",
          code,
          message: error instanceof Error ? error.message : "Unknown error",
        };
      }
    })

    // Context synthesis endpoint (Context Restore Phase 2)
    .post("/api/intel/synthesize-context", async ({ body, set, correlationId }) => {
      const validation = ContextSynthesisRequestSchema.safeParse(body);
      if (!validation.success) {
        set.status = 400;
        return {
          error: "Invalid request",
          details: validation.error.flatten().fieldErrors,
        };
      }

      return withOperationLogging(
        {
          operation: "synthesize_context",
          getInputMetrics: () => ({
            query: validation.data.query,
            cluster_count: validation.data.clusters.length,
            read_count: validation.data.suggested_reads.length,
            has_context_package: !!validation.data.context_package_content,
          }),
          getOutputMetrics: (result) => ({
            model_used: (result as ContextSynthesisResponse).model_used,
            one_liner_length: (result as ContextSynthesisResponse).one_liner.length,
            cluster_names_count: (result as ContextSynthesisResponse).cluster_names.length,
          }),
        },
        async () => {
          const client = getAnthropicClient();
          return await synthesizeContext(client, validation.data);
        }
      )({ correlationId, body, set });
    })
}

/**
 * Start the server
 * Only runs when executed directly (not imported for tests)
 */
function startServer() {
  const port = parseInt(process.env.INTEL_PORT || String(DEFAULT_PORT));

  const app = createApp().listen(port);

  console.log(`
╔════════════════════════════════════════════════════════════╗
║         Tastematter Intelligence Service v${VERSION}          ║
╠════════════════════════════════════════════════════════════╣
║  Listening on: http://localhost:${port}                       ║
║  Health check: http://localhost:${port}/api/intel/health      ║
╚════════════════════════════════════════════════════════════╝
  `);

  return app;
}

// Start server if running directly
if (import.meta.main) {
  startServer();
}
