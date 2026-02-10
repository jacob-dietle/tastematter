/**
 * Shared type contracts for Tastematter Intelligence Service
 *
 * IMPORTANT: These Zod schemas MUST match Rust serde serialization exactly.
 * Rust uses #[serde(rename_all = "kebab-case")] for enums.
 *
 * Parity is verified via contract tests against Rust-generated JSON fixtures.
 */

import { z } from "zod";

// ============================================================================
// Enums (must match Rust #[serde(rename_all = "kebab-case")])
// ============================================================================

/**
 * Chain category enum - matches Rust ChainCategory
 * @see core/src/intelligence/types.rs
 */
export const ChainCategorySchema = z.enum([
  "bug-fix",
  "feature",
  "refactor",
  "research",
  "cleanup",
  "documentation",
  "testing",
  "unknown",
]);

export type ChainCategory = z.infer<typeof ChainCategorySchema>;

/**
 * Health status enum
 */
export const HealthStatusSchema = z.enum(["ok", "error"]);

export type HealthStatus = z.infer<typeof HealthStatusSchema>;

// ============================================================================
// Request Schemas
// ============================================================================

/**
 * Chain naming request - sent from Rust IntelClient
 *
 * Enrichment fields (tools_used, first_user_intent, commit_messages) are optional
 * for backward compatibility. When present, they provide much richer context
 * for Haiku to generate meaningful chain names.
 */
export const ChainNamingRequestSchema = z.object({
  chain_id: z.string().min(1, "chain_id must be non-empty"),
  files_touched: z.array(z.string()),
  session_count: z.number().int().positive("session_count must be positive"),
  recent_sessions: z.array(z.string()),

  // Enrichment fields (all optional for backward compatibility)
  /** Aggregated tool usage across sessions: {"Read": 23, "Edit": 12} */
  tools_used: z.record(z.number()).optional(),
  /** User's stated intent - conversation excerpt from first session in chain */
  first_user_intent: z.string().optional(),
  /** Commit messages from the chain: ["feat: Add query engine"] */
  commit_messages: z.array(z.string()).optional(),

  // A/B test fields - explicit separation of first message vs full excerpt
  /** First user message only (for A/B testing) */
  first_user_message: z.string().optional(),
  /** Full conversation excerpt - all user messages (~8K chars) */
  conversation_excerpt: z.string().optional(),
});

export type ChainNamingRequest = z.infer<typeof ChainNamingRequestSchema>;

// ============================================================================
// Response Schemas
// ============================================================================

/**
 * Chain naming response - returned to Rust IntelClient
 */
export const ChainNamingResponseSchema = z.object({
  chain_id: z.string(),
  generated_name: z.string(),
  category: ChainCategorySchema,
  confidence: z.number().min(0).max(1),
  model_used: z.string(),
});

export type ChainNamingResponse = z.infer<typeof ChainNamingResponseSchema>;

/**
 * Chain metadata - stored in SQLite cache
 * All fields except chain_id are nullable (may not be generated yet)
 */
export const ChainMetadataSchema = z.object({
  chain_id: z.string(),
  generated_name: z.string().nullable(),
  category: ChainCategorySchema.nullable(),
  confidence: z.number().min(0).max(1).nullable(),
  generated_at: z.string().datetime().nullable(),
  model_used: z.string().nullable(),
});

export type ChainMetadata = z.infer<typeof ChainMetadataSchema>;

/**
 * Health check response
 */
export const HealthResponseSchema = z.object({
  status: HealthStatusSchema,
  version: z.string(),
});

export type HealthResponse = z.infer<typeof HealthResponseSchema>;

// ============================================================================
// Error Schemas
// ============================================================================

/**
 * API error response
 */
export const ErrorResponseSchema = z.object({
  error: z.string(),
  code: z.string(),
  correlation_id: z.string().optional(),
});

export type ErrorResponse = z.infer<typeof ErrorResponseSchema>;

// ============================================================================
// Risk & Insight Enums (Phase 4 - must match Rust #[serde(rename_all = "kebab-case")])
// ============================================================================

/**
 * Risk level for commit analysis
 */
export const RiskLevelSchema = z.enum(["low", "medium", "high"]);
export type RiskLevel = z.infer<typeof RiskLevelSchema>;

/**
 * Insight types for pattern detection
 */
export const InsightTypeSchema = z.enum([
  "focus-shift",
  "co-occurrence",
  "pending-review",
  "anomaly",
  "continuity",
]);
export type InsightType = z.infer<typeof InsightTypeSchema>;

/**
 * Action types for insight recommendations
 */
export const ActionTypeSchema = z.enum(["navigate", "filter", "external"]);
export type ActionType = z.infer<typeof ActionTypeSchema>;

// ============================================================================
// Commit Analysis Schemas (Phase 4)
// ============================================================================

/**
 * Request for commit analysis
 */
export const CommitAnalysisRequestSchema = z.object({
  commit_hash: z.string().min(1, "commit_hash must be non-empty"),
  message: z.string(),
  author: z.string(),
  diff: z.string(),
  files_changed: z.array(z.string()),
});
export type CommitAnalysisRequest = z.infer<typeof CommitAnalysisRequestSchema>;

/**
 * Response from commit analysis
 */
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

// ============================================================================
// Insights Schemas (Phase 4)
// ============================================================================

/**
 * Action associated with an insight
 */
export const InsightActionSchema = z.object({
  label: z.string(),
  action_type: ActionTypeSchema,
  payload: z.record(z.unknown()),
});
export type InsightAction = z.infer<typeof InsightActionSchema>;

/**
 * Single insight
 */
export const InsightSchema = z.object({
  id: z.string(),
  insight_type: InsightTypeSchema,
  title: z.string(),
  description: z.string(),
  evidence: z.array(z.string()),
  action: InsightActionSchema.nullable(),
});
export type Insight = z.infer<typeof InsightSchema>;

/**
 * Chain data for insights request
 */
export const ChainDataSchema = z.object({
  chain_id: z.string(),
  name: z.string().nullable(),
  session_count: z.number().int(),
  file_count: z.number().int(),
  recent_activity: z.string(),
});
export type ChainData = z.infer<typeof ChainDataSchema>;

/**
 * File pattern data for insights request
 */
export const FilePatternSchema = z.object({
  file_path: z.string(),
  access_count: z.number().int(),
  co_accessed_with: z.array(z.string()),
});
export type FilePattern = z.infer<typeof FilePatternSchema>;

/**
 * Request for insights generation
 */
export const InsightsRequestSchema = z.object({
  time_range: z.string(),
  chain_data: z.array(ChainDataSchema),
  file_patterns: z.array(FilePatternSchema),
});
export type InsightsRequest = z.infer<typeof InsightsRequestSchema>;

/**
 * Response from insights generation
 */
export const InsightsResponseSchema = z.object({
  insights: z.array(InsightSchema),
  model_used: z.string(),
});
export type InsightsResponse = z.infer<typeof InsightsResponseSchema>;

// ============================================================================
// Session Summary Schemas (Phase 4)
// ============================================================================

/**
 * Request for session summary
 */
export const SessionSummaryRequestSchema = z.object({
  session_id: z.string().min(1, "session_id must be non-empty"),
  files: z.array(z.string()),
  duration_seconds: z.number().int().nonnegative().nullable(),
  chain_id: z.string().nullable(),
});
export type SessionSummaryRequest = z.infer<typeof SessionSummaryRequestSchema>;

/**
 * Response from session summary
 */
export const SessionSummaryResponseSchema = z.object({
  session_id: z.string(),
  summary: z.string(),
  key_files: z.array(z.string()),
  focus_area: z.string().nullable(),
  model_used: z.string(),
});
export type SessionSummaryResponse = z.infer<typeof SessionSummaryResponseSchema>;

// ============================================================================
// A/B Test Schemas (Chain Naming Quality Comparison)
// ============================================================================

/**
 * Quality comparison between first_message and full_excerpt naming
 */
export const QualityComparisonSchema = z.object({
  winner: z.enum(["first_message", "full_excerpt", "tie"]),
  confidence_delta: z.number(), // full_excerpt confidence - first_message confidence
  name_length_delta: z.number(), // Difference in name specificity (char count)
});
export type QualityComparison = z.infer<typeof QualityComparisonSchema>;

/**
 * A/B test result for chain naming quality comparison
 */
export const ABTestResultSchema = z.object({
  chain_id: z.string(),
  first_message_result: ChainNamingResponseSchema,
  full_excerpt_result: ChainNamingResponseSchema,
  quality_comparison: QualityComparisonSchema,
});
export type ABTestResult = z.infer<typeof ABTestResultSchema>;

// ============================================================================
// Chain Summary Schemas (Workstream Tagging)
// ============================================================================

/**
 * Work status for chain tracking
 */
export const WorkStatusSchema = z.enum([
  "in_progress",
  "complete",
  "paused",
  "abandoned",
]);
export type WorkStatus = z.infer<typeof WorkStatusSchema>;

/**
 * Workstream tag with source tracking
 */
export const WorkstreamTagSchema = z.object({
  tag: z.string(),
  source: z.enum(["existing", "generated"]), // existing = from workstreams.yaml
});
export type WorkstreamTag = z.infer<typeof WorkstreamTagSchema>;

/**
 * Request for chain summary with workstream tagging
 */
export const ChainSummaryRequestSchema = z.object({
  chain_id: z.string().min(1, "chain_id must be non-empty"),
  conversation_excerpt: z.string().optional(),
  files_touched: z.array(z.string()),
  session_count: z.number().int().positive(),
  duration_seconds: z.number().int().nonnegative().nullable(),
  /** Existing workstreams from workstreams.yaml for matching */
  existing_workstreams: z.array(z.string()).optional(),
});
export type ChainSummaryRequest = z.infer<typeof ChainSummaryRequestSchema>;

/**
 * Response from chain summary with workstream tags
 */
export const ChainSummaryResponseSchema = z.object({
  chain_id: z.string(),
  summary: z.string(), // 2-3 sentence summary
  accomplishments: z.array(z.string()), // What was done
  status: WorkStatusSchema,
  key_files: z.array(z.string()), // Top files across sessions
  workstream_tags: z.array(WorkstreamTagSchema),
  model_used: z.string(),
});
export type ChainSummaryResponse = z.infer<typeof ChainSummaryResponseSchema>;

// ============================================================================
// GitOps Decision Schemas (Intelligent GitOps Level 0)
// ============================================================================

/**
 * GitOps action types
 */
export const GitOpsActionSchema = z.enum([
  "commit", // Ready to commit - suggest message
  "push", // Committed but unpushed - suggest push
  "notify", // Needs attention - explain why
  "wait", // Not ready - explain why waiting
  "ask", // Need user input before deciding
]);
export type GitOpsAction = z.infer<typeof GitOpsActionSchema>;

/**
 * GitOps urgency levels
 */
export const GitOpsUrgencySchema = z.enum(["low", "medium", "high"]);
export type GitOpsUrgency = z.infer<typeof GitOpsUrgencySchema>;

/**
 * Uncommitted file with status
 */
export const UncommittedFileSchema = z.object({
  path: z.string(),
  status: z.enum(["modified", "added", "deleted", "renamed"]),
  lines_changed: z.number().int().nullable(),
});
export type UncommittedFile = z.infer<typeof UncommittedFileSchema>;

/**
 * Recent session context for GitOps decisions
 */
export const RecentSessionContextSchema = z.object({
  session_id: z.string(),
  ended_at: z.string().nullable(), // ISO8601, null if still active
  files_touched: z.array(z.string()),
  duration_minutes: z.number().int(),
  conversation_summary: z.string().nullable(),
});
export type RecentSessionContext = z.infer<typeof RecentSessionContextSchema>;

/**
 * Active chain context for GitOps decisions
 */
export const ActiveChainContextSchema = z.object({
  chain_id: z.string(),
  workstream_tags: z.array(z.string()),
  accomplishments: z.array(z.string()),
  status: z.enum(["in_progress", "complete", "paused"]),
});
export type ActiveChainContext = z.infer<typeof ActiveChainContextSchema>;

/**
 * GitOps signals - input from Rust daemon
 */
export const GitOpsSignalsSchema = z.object({
  // Git state
  uncommitted_files: z.array(UncommittedFileSchema),
  unpushed_commits: z.number().int(),
  current_branch: z.string(),
  last_commit_timestamp: z.string().nullable(), // ISO8601
  last_push_timestamp: z.string().nullable(), // ISO8601

  // Session context
  recent_session: RecentSessionContextSchema.nullable(),
  active_chain: ActiveChainContextSchema.nullable(),

  // User rules (promptable - natural language)
  user_rules: z.array(z.string()),

  // Time context
  hours_since_last_commit: z.number().nullable(),
  hours_since_last_push: z.number().nullable(),
});
export type GitOpsSignals = z.infer<typeof GitOpsSignalsSchema>;

/**
 * GitOps decision - output from agent
 */
export const GitOpsDecisionSchema = z.object({
  action: GitOpsActionSchema,
  reason: z.string(), // Human-readable explanation
  urgency: GitOpsUrgencySchema,

  // Optional details based on action
  suggested_commit_message: z.string().nullable(),
  files_to_stage: z.array(z.string()).nullable(),
  coherence_assessment: z.string().nullable(), // "auth feature", "mixed changes"

  model_used: z.string(),
});
export type GitOpsDecision = z.infer<typeof GitOpsDecisionSchema>;

// ============================================================================
// Context Synthesis Schemas (Context Restore Phase 2)
// ============================================================================

/**
 * Cluster input for context synthesis
 */
export const ClusterInputSchema = z.object({
  files: z.array(z.string()),
  access_pattern: z.string(),
  pmi_score: z.number(),
});
export type ClusterInput = z.infer<typeof ClusterInputSchema>;

/**
 * Suggested read input for context synthesis
 */
export const SuggestedReadInputSchema = z.object({
  path: z.string(),
  priority: z.number().int(),
  surprise: z.boolean(),
});
export type SuggestedReadInput = z.infer<typeof SuggestedReadInputSchema>;

/**
 * Request for context synthesis - curated subset of ContextRestoreResult
 */
export const ContextSynthesisRequestSchema = z.object({
  query: z.string(),
  status: z.string(),
  work_tempo: z.string(),
  clusters: z.array(ClusterInputSchema),
  suggested_reads: z.array(SuggestedReadInputSchema),
  context_package_content: z.string().optional(),
  key_metrics: z.record(z.unknown()).optional(),
  evidence_sources: z.array(z.string()),
});
export type ContextSynthesisRequest = z.infer<typeof ContextSynthesisRequestSchema>;

/**
 * Response from context synthesis - fills 5 None fields
 */
export const ContextSynthesisResponseSchema = z.object({
  one_liner: z.string(),
  narrative: z.string(),
  cluster_names: z.array(z.string()),
  cluster_interpretations: z.array(z.string()),
  suggested_read_reasons: z.array(z.string()),
  model_used: z.string(),
});
export type ContextSynthesisResponse = z.infer<typeof ContextSynthesisResponseSchema>;
