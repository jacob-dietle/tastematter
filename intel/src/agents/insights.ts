/**
 * Insights Agent
 *
 * Uses Claude Sonnet to detect patterns in coding activity and generate
 * actionable insights based on chain data and file access patterns.
 *
 * Pattern: tool_choice for guaranteed structured JSON output
 * Model: claude-sonnet-4-5-20250929 (needs reasoning for pattern detection)
 *
 * This is the most complex agent - outputs multiple insights with optional actions
 */

import type Anthropic from "@anthropic-ai/sdk";
import type { InsightsRequest, InsightsResponse, Insight } from "../types/shared";
import { InsightsResponseSchema } from "../types/shared";

const MODEL = "claude-sonnet-4-5-20250929";

/**
 * System prompt with pattern detection rules
 */
const INSIGHTS_SYSTEM_PROMPT = `You are a coding activity analyst. Given chain data and file access patterns, identify meaningful insights.

INSIGHT TYPES:
1. focus-shift: Work focus changed significantly between periods
2. co-occurrence: Files that are frequently accessed together
3. pending-review: Work that may need review (large changes, many files)
4. anomaly: Unusual patterns (late-night work, spikes in activity)
5. continuity: Related work that spans multiple sessions or chains

RULES:
1. Only generate insights that are genuinely useful - quality over quantity
2. Each insight must have specific evidence (not vague claims)
3. If there are no meaningful patterns, return an empty insights array
4. Actions should be actionable - navigate to files, filter views, etc.
5. Generate unique IDs for each insight (e.g., "insight-1", "insight-2")

ACTION TYPES:
- navigate: Go to a specific file or location
- filter: Apply a filter to the view (e.g., show only certain files)
- external: Link to external resource (e.g., PR, issue)

You MUST use the output_insights tool to provide your response.`;

/**
 * Tool definition for structured output
 * Forces Claude to return JSON matching our schema
 */
export const INSIGHTS_TOOL: Anthropic.Tool = {
  name: "output_insights",
  description: "Output the detected insights",
  input_schema: {
    type: "object",
    properties: {
      insights: {
        type: "array",
        description: "Array of detected insights",
        items: {
          type: "object",
          properties: {
            id: {
              type: "string",
              description: "Unique identifier for this insight",
            },
            insight_type: {
              type: "string",
              enum: ["focus-shift", "co-occurrence", "pending-review", "anomaly", "continuity"],
              description: "Type of insight detected",
            },
            title: {
              type: "string",
              description: "Short title for the insight",
            },
            description: {
              type: "string",
              description: "Detailed description of the insight",
            },
            evidence: {
              type: "array",
              items: { type: "string" },
              description: "Specific evidence supporting this insight",
            },
            action: {
              type: ["object", "null"],
              description: "Optional action the user can take",
              properties: {
                label: { type: "string" },
                action_type: {
                  type: "string",
                  enum: ["navigate", "filter", "external"],
                },
                payload: {
                  type: "object",
                  additionalProperties: true,
                },
              },
              required: ["label", "action_type", "payload"],
            },
          },
          required: ["id", "insight_type", "title", "description", "evidence"],
        },
      },
    },
    required: ["insights"],
  },
};

/**
 * Build the user prompt with activity context
 */
export function buildPrompt(request: InsightsRequest): string {
  const chainSection = request.chain_data.length > 0
    ? request.chain_data.map((c) =>
        `- ${c.chain_id}: "${c.name ?? 'Unnamed'}" (${c.session_count} sessions, ${c.file_count} files, active ${c.recent_activity})`
      ).join("\n")
    : "(no chain data)";

  const fileSection = request.file_patterns.length > 0
    ? request.file_patterns.map((f) =>
        `- ${f.file_path}: accessed ${f.access_count} times, co-accessed with: [${f.co_accessed_with.join(", ")}]`
      ).join("\n")
    : "(no file patterns)";

  return `Analyze the following coding activity and detect meaningful patterns. Use the output_insights tool to provide your response.

INPUT:
Time Range: ${request.time_range}

Chains:
${chainSection}

File Patterns:
${fileSection}

Analyze this activity data and use the output_insights tool. If there are no meaningful patterns, return an empty insights array.`;
}

/**
 * Interface for insight action from tool response
 */
interface RawInsightAction {
  label: string;
  action_type: string;
  payload: Record<string, unknown>;
}

/**
 * Interface for insight from tool response
 */
interface RawInsight {
  id: string;
  insight_type: string;
  title: string;
  description: string;
  evidence: string[];
  action?: RawInsightAction | null;
}

/**
 * Interface for tool_use content block
 */
interface ToolUseBlock {
  type: "tool_use";
  id: string;
  name: string;
  input: {
    insights: RawInsight[];
  };
}

/**
 * Generate insights from coding activity using Claude Sonnet
 *
 * @param client - Anthropic SDK client
 * @param request - Insights request with chain and file data
 * @returns Validated insights response
 * @throws Error if response doesn't contain valid tool_use
 */
export async function generateInsights(
  client: Anthropic,
  request: InsightsRequest
): Promise<InsightsResponse> {
  const response = await client.messages.create({
    model: MODEL,
    max_tokens: 1024,
    system: INSIGHTS_SYSTEM_PROMPT,
    messages: [
      {
        role: "user",
        content: buildPrompt(request),
      },
    ],
    tools: [INSIGHTS_TOOL],
    tool_choice: { type: "tool", name: "output_insights" },
  });

  // Find the tool_use block in the response
  const toolUse = response.content.find(
    (block): block is ToolUseBlock => block.type === "tool_use"
  );

  if (!toolUse) {
    throw new Error("No tool_use block in response - expected structured output");
  }

  // Transform and validate insights
  const insights: Insight[] = toolUse.input.insights.map((raw) => ({
    id: raw.id,
    insight_type: raw.insight_type as Insight["insight_type"],
    title: raw.title,
    description: raw.description,
    evidence: raw.evidence,
    action: raw.action ? {
      label: raw.action.label,
      action_type: raw.action.action_type as "navigate" | "filter" | "external",
      payload: raw.action.payload,
    } : null,
  }));

  // Construct and validate the response
  const result = {
    insights,
    model_used: MODEL,
  };

  // Validate against Zod schema - throws if invalid
  return InsightsResponseSchema.parse(result);
}
