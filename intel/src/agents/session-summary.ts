/**
 * Session Summary Agent
 *
 * Uses Claude Haiku to generate summaries of coding sessions
 * based on files touched, duration, and chain context.
 *
 * Pattern: tool_choice for guaranteed structured JSON output
 * Model: claude-haiku-4-5-20251001 (fast/cheap - summarization is simpler)
 */

import type Anthropic from "@anthropic-ai/sdk";
import type { SessionSummaryRequest, SessionSummaryResponse } from "../types/shared";
import { SessionSummaryResponseSchema } from "../types/shared";

const MODEL = "claude-haiku-4-5-20251001";

/**
 * System prompt with summarization rules
 */
const SESSION_SUMMARY_SYSTEM_PROMPT = `You are a coding session summarizer. Given information about a work session, provide a brief summary.

RULES:
1. Be concise (1-2 sentences for summary)
2. Identify key files that were most important to the session
3. Determine focus area if there's a clear theme (e.g., "Security", "Testing", "UI")
4. If files are from disparate areas with no clear theme, focus_area should be null
5. Prioritize files that appear to be the main work, not just incidental touches

EXAMPLES:
- Files: [auth.ts, login.ts, session.ts] → focus_area: "Authentication"
- Files: [tests/user.test.ts, tests/auth.test.ts] → focus_area: "Testing"
- Files: [README.md, CHANGELOG.md, docs/api.md] → focus_area: "Documentation"
- Files: [random mix of files] → focus_area: null

You MUST use the output_session_summary tool to provide your response.`;

/**
 * Tool definition for structured output
 * Forces Claude to return JSON matching our schema
 */
export const SESSION_SUMMARY_TOOL: Anthropic.Tool = {
  name: "output_session_summary",
  description: "Output the session summary results",
  input_schema: {
    type: "object",
    properties: {
      summary: {
        type: "string",
        description: "Brief summary of what was worked on (1-2 sentences)",
      },
      key_files: {
        type: "array",
        items: { type: "string" },
        description: "Most important files from the session (top 3-5)",
      },
      focus_area: {
        type: ["string", "null"],
        description: "Theme of the work (e.g., 'Security', 'Testing') or null if mixed",
      },
    },
    required: ["summary", "key_files", "focus_area"],
  },
};

/**
 * Build the user prompt with session context
 */
export function buildPrompt(request: SessionSummaryRequest): string {
  const filesSection =
    request.files.length > 0
      ? request.files.map((f) => `- ${f}`).join("\n")
      : "(none)";

  const durationStr = request.duration_seconds !== null
    ? `${request.duration_seconds} seconds`
    : "unknown";

  const chainSection = request.chain_id
    ? `Chain ID: ${request.chain_id}`
    : "Chain ID: (not part of a chain)";

  return `Summarize the following coding session. Use the output_session_summary tool to provide your response.

INPUT:
Session ID: ${request.session_id}
Duration: ${durationStr}
${chainSection}

Files Accessed:
${filesSection}

Analyze this session and use the output_session_summary tool.`;
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
    key_files: string[];
    focus_area: string | null;
  };
}

/**
 * Summarize a coding session using Claude Haiku
 *
 * @param client - Anthropic SDK client
 * @param request - Session summary request with context
 * @returns Validated session summary response
 * @throws Error if response doesn't contain valid tool_use
 */
export async function summarizeSession(
  client: Anthropic,
  request: SessionSummaryRequest
): Promise<SessionSummaryResponse> {
  const response = await client.messages.create({
    model: MODEL,
    max_tokens: 256,
    system: SESSION_SUMMARY_SYSTEM_PROMPT,
    messages: [
      {
        role: "user",
        content: buildPrompt(request),
      },
    ],
    tools: [SESSION_SUMMARY_TOOL],
    tool_choice: { type: "tool", name: "output_session_summary" },
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
    session_id: request.session_id,
    summary: toolUse.input.summary,
    key_files: toolUse.input.key_files,
    focus_area: toolUse.input.focus_area,
    model_used: MODEL,
  };

  // Validate against Zod schema - throws if invalid
  return SessionSummaryResponseSchema.parse(result);
}
