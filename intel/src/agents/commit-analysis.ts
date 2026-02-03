/**
 * Commit Analysis Agent
 *
 * Uses Claude Sonnet to analyze git commits and determine:
 * - Whether it was made by an AI agent
 * - Risk level for code review
 * - What the reviewer should focus on
 *
 * Pattern: tool_choice for guaranteed structured JSON output
 * Model: claude-sonnet-4-5-20250929 (requires reasoning)
 */

import type Anthropic from "@anthropic-ai/sdk";
import type { CommitAnalysisRequest, CommitAnalysisResponse } from "../types/shared";
import { CommitAnalysisResponseSchema } from "../types/shared";

const MODEL = "claude-sonnet-4-5-20250929";

/**
 * System prompt with analysis rules
 */
const COMMIT_ANALYSIS_SYSTEM_PROMPT = `You are a commit analysis expert. Your job is to analyze git commits and determine:
1. Whether it was made by an AI agent (Claude, Copilot, etc.) based on patterns
2. A brief summary of what the commit does
3. Risk level for code review (low/medium/high)
4. What the reviewer should focus on
5. Related files that might be affected but weren't changed

RULES:
1. Be concise in summaries (1-2 sentences max)
2. Risk assessment:
   - low = routine changes, formatting, documentation
   - medium = logic changes, new features, refactoring
   - high = security changes, data handling, critical paths, authentication
3. Agent commit indicators:
   - "Co-Authored-By: Claude" or similar signatures
   - Systematic refactors with consistent patterns
   - Generated code with comments like "// AI-generated"
   - Very uniform formatting or structure
4. For related files, suggest files that might need review based on:
   - Import dependencies
   - Shared types/interfaces
   - Test files for changed modules

You MUST use the output_commit_analysis tool to provide your response.`;

/**
 * Tool definition for structured output
 * Forces Claude to return JSON matching our schema
 */
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

/**
 * Build the user prompt with commit context
 */
export function buildPrompt(request: CommitAnalysisRequest): string {
  const filesSection =
    request.files_changed.length > 0
      ? request.files_changed.map((f) => `- ${f}`).join("\n")
      : "(none)";

  return `Analyze the following git commit. Use the output_commit_analysis tool to provide your response.

INPUT:
Commit Hash: ${request.commit_hash}
Author: ${request.author}
Message: ${request.message}

Files Changed:
${filesSection}

Diff:
\`\`\`
${request.diff}
\`\`\`

Analyze this commit and use the output_commit_analysis tool.`;
}

/**
 * Interface for tool_use content block
 */
interface ToolUseBlock {
  type: "tool_use";
  id: string;
  name: string;
  input: {
    is_agent_commit: boolean;
    summary: string;
    risk_level: string;
    review_focus: string;
    related_files: string[];
  };
}

/**
 * Analyze a git commit using Claude Sonnet
 *
 * @param client - Anthropic SDK client
 * @param request - Commit analysis request with context
 * @returns Validated commit analysis response
 * @throws Error if response doesn't contain valid tool_use
 */
export async function analyzeCommit(
  client: Anthropic,
  request: CommitAnalysisRequest
): Promise<CommitAnalysisResponse> {
  const response = await client.messages.create({
    model: MODEL,
    max_tokens: 512,
    system: COMMIT_ANALYSIS_SYSTEM_PROMPT,
    messages: [
      {
        role: "user",
        content: buildPrompt(request),
      },
    ],
    tools: [COMMIT_ANALYSIS_TOOL],
    tool_choice: { type: "tool", name: "output_commit_analysis" },
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
    commit_hash: request.commit_hash,
    is_agent_commit: toolUse.input.is_agent_commit,
    summary: toolUse.input.summary,
    risk_level: toolUse.input.risk_level,
    review_focus: toolUse.input.review_focus,
    related_files: toolUse.input.related_files,
    model_used: MODEL,
  };

  // Validate against Zod schema - throws if invalid
  return CommitAnalysisResponseSchema.parse(result);
}
