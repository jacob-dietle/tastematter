/**
 * Chain Summary Agent
 *
 * Uses Claude Haiku to generate summaries of conversation chains
 * with workstream tagging for longitudinal tracking.
 *
 * Pattern: tool_choice for guaranteed structured JSON output
 * Model: claude-haiku-4-5-20251001 (fast/cheap)
 */

import type Anthropic from "@anthropic-ai/sdk";
import type { ChainSummaryRequest, ChainSummaryResponse, WorkstreamTag } from "../types/shared";
import { ChainSummaryResponseSchema } from "../types/shared";

const MODEL = "claude-haiku-4-5-20251001";

/**
 * Build system prompt with existing workstreams for hybrid tagging
 */
function buildSystemPrompt(existingWorkstreams: string[]): string {
  const workstreamList = existingWorkstreams.length > 0
    ? existingWorkstreams.map((w) => `- ${w}`).join("\n")
    : "(none defined)";

  return `You are a work analyst. Given a conversation chain, generate a summary and workstream tags.

EXISTING WORKSTREAMS (check these first for matching):
${workstreamList}

TAGGING RULES:
1. If work matches an existing workstream, use that exact tag and mark source as "existing"
2. If work doesn't match any existing workstream, generate a new semantic tag (kebab-case) and mark source as "generated"
3. Multiple tags allowed if work spans workstreams
4. Tags should be specific enough to track over time (e.g., "linkedin-intelligence" not just "linkedin")

STATUS RULES:
- "in_progress": Work is ongoing, chain is recent/active
- "complete": Work appears finished, clear completion signals
- "paused": Work started but no recent activity
- "abandoned": Work started but incomplete and likely not returning

EXAMPLES:
- LinkedIn pipeline for client → tags: [{tag: "pixee", source: "existing"}, {tag: "linkedin-intelligence", source: "generated"}]
- Building skills for Claude → tags: [{tag: "skill-development", source: "generated"}, {tag: "context-engineering", source: "existing"}]
- General ops work → tags: [{tag: "gtm-ops", source: "generated"}]

You MUST use the output_chain_summary tool to provide your response.`;
}

/**
 * Tool definition for structured output
 * Forces Claude to return JSON matching our schema
 */
export const CHAIN_SUMMARY_TOOL: Anthropic.Tool = {
  name: "output_chain_summary",
  description: "Output the chain summary with workstream tags",
  input_schema: {
    type: "object",
    properties: {
      summary: {
        type: "string",
        description: "2-3 sentence summary of what was accomplished across all sessions",
      },
      accomplishments: {
        type: "array",
        items: { type: "string" },
        description: "List of specific things accomplished (3-7 items)",
      },
      status: {
        type: "string",
        enum: ["in_progress", "complete", "paused", "abandoned"],
        description: "Current status of this work",
      },
      key_files: {
        type: "array",
        items: { type: "string" },
        description: "Top 5-10 most important files across all sessions",
      },
      workstream_tags: {
        type: "array",
        items: {
          type: "object",
          properties: {
            tag: { type: "string", description: "The workstream tag (kebab-case)" },
            source: {
              type: "string",
              enum: ["existing", "generated"],
              description: "Whether tag matched existing workstream or was generated",
            },
          },
          required: ["tag", "source"],
        },
        description: "Workstream tags for tracking (1-3 tags)",
      },
    },
    required: ["summary", "accomplishments", "status", "key_files", "workstream_tags"],
  },
};

/**
 * Build the user prompt with chain context
 */
export function buildPrompt(request: ChainSummaryRequest): string {
  const filesSection =
    request.files_touched.length > 0
      ? request.files_touched.slice(0, 30).map((f) => `- ${f}`).join("\n") +
        (request.files_touched.length > 30 ? `\n... and ${request.files_touched.length - 30} more` : "")
      : "(none)";

  const durationStr = request.duration_seconds !== null
    ? `${Math.round(request.duration_seconds / 60)} minutes`
    : "unknown";

  const excerptSection = request.conversation_excerpt
    ? `User Conversation Excerpt (use this to understand intent):\n"""\n${request.conversation_excerpt.slice(0, 3000)}\n"""\n`
    : "";

  return `Summarize the following conversation chain and identify workstream tags. Use the output_chain_summary tool to provide your response.

INPUT:
Chain ID: ${request.chain_id}
Sessions: ${request.session_count}
Total Duration: ${durationStr}

${excerptSection}
Files Touched:
${filesSection}

Analyze this chain and use the output_chain_summary tool.`;
}

/**
 * Interface for tool_use content block
 */
interface ToolUseBlock {
  type: "tool_use";
  id: string;
  name: string;
  input: {
    summary: string;
    accomplishments: string[];
    status: "in_progress" | "complete" | "paused" | "abandoned";
    key_files: string[];
    workstream_tags: Array<{ tag: string; source: "existing" | "generated" }>;
  };
}

/**
 * Summarize a conversation chain with workstream tagging
 *
 * @param client - Anthropic SDK client
 * @param request - Chain summary request with context
 * @returns Validated chain summary response with workstream tags
 * @throws Error if response doesn't contain valid tool_use
 */
export async function summarizeChain(
  client: Anthropic,
  request: ChainSummaryRequest
): Promise<ChainSummaryResponse> {
  const existingWorkstreams = request.existing_workstreams ?? [];

  const response = await client.messages.create({
    model: MODEL,
    max_tokens: 512,
    system: buildSystemPrompt(existingWorkstreams),
    messages: [
      {
        role: "user",
        content: buildPrompt(request),
      },
    ],
    tools: [CHAIN_SUMMARY_TOOL],
    tool_choice: { type: "tool", name: "output_chain_summary" },
  });

  // Find the tool_use block in the response
  const toolUse = response.content.find(
    (block): block is ToolUseBlock => block.type === "tool_use"
  );

  if (!toolUse) {
    throw new Error("No tool_use block in response - expected structured output");
  }

  // Construct and validate the response
  const result = {
    chain_id: request.chain_id,
    summary: toolUse.input.summary,
    accomplishments: toolUse.input.accomplishments,
    status: toolUse.input.status,
    key_files: toolUse.input.key_files,
    workstream_tags: toolUse.input.workstream_tags as WorkstreamTag[],
    model_used: MODEL,
  };

  // Validate against Zod schema - throws if invalid
  return ChainSummaryResponseSchema.parse(result);
}
