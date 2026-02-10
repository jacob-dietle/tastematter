/**
 * Context Synthesis Agent
 *
 * Uses Claude Haiku to fill 5 Option<String> fields in context restore output:
 * - one_liner: <120 char project summary
 * - narrative: 2-4 sentence state description
 * - cluster_names: 2-4 word labels per cluster
 * - cluster_interpretations: what each cluster means
 * - suggested_read_reasons: why to read each file
 *
 * Pattern: tool_choice for guaranteed structured JSON output
 * Model: claude-haiku-4-5-20251001 (same as chain-summary)
 */

import type Anthropic from "@anthropic-ai/sdk";
import type { ContextSynthesisRequest, ContextSynthesisResponse } from "../types/shared";
import { ContextSynthesisResponseSchema } from "../types/shared";

const MODEL = "claude-haiku-4-5-20251001";

/**
 * Build system prompt with expected array lengths
 */
function buildSystemPrompt(clusterCount: number, readCount: number): string {
  return `You are a context analyst for a developer's project. Given deterministic data about their recent work, synthesize human-readable summaries.

OUTPUT RULES:
- one_liner: Under 120 characters. Factual summary of project state. Example: "Nickel transcript worker is production-ready with 4 providers"
- narrative: 2-4 sentences. Ground every claim in the evidence provided. Start with what was built, then current state, then what's next.
- cluster_names: Exactly ${clusterCount} names, each 2-4 words. Describe what the file group does. Example: "Core Pipeline", "Type Contracts"
- cluster_interpretations: Exactly ${clusterCount} interpretations. One sentence each explaining why these files move together.
- suggested_read_reasons: Exactly ${readCount} reasons. One sentence each explaining why the developer should read this file to resume work.

GROUNDING RULES:
- Only reference files, metrics, and evidence provided in the input
- If evidence is thin, keep summaries brief rather than speculating
- Use developer-facing language, not marketing language

You MUST use the output_context_synthesis tool to provide your response.`;
}

/**
 * Tool definition for structured output
 */
export const CONTEXT_SYNTHESIS_TOOL: Anthropic.Tool = {
  name: "output_context_synthesis",
  description: "Output synthesized context for a developer's project",
  input_schema: {
    type: "object",
    properties: {
      one_liner: {
        type: "string",
        description: "Under 120 character project state summary",
      },
      narrative: {
        type: "string",
        description: "2-4 sentence description of current project state",
      },
      cluster_names: {
        type: "array",
        items: { type: "string" },
        description: "2-4 word name for each work cluster (index-matched)",
      },
      cluster_interpretations: {
        type: "array",
        items: { type: "string" },
        description: "One sentence interpretation per cluster (index-matched)",
      },
      suggested_read_reasons: {
        type: "array",
        items: { type: "string" },
        description: "One sentence reason per suggested read (index-matched)",
      },
    },
    required: [
      "one_liner",
      "narrative",
      "cluster_names",
      "cluster_interpretations",
      "suggested_read_reasons",
    ],
  },
};

/**
 * Build user prompt from synthesis request
 */
export function buildPrompt(request: ContextSynthesisRequest): string {
  // Numbered clusters
  const clustersSection =
    request.clusters.length > 0
      ? request.clusters
          .map(
            (c, i) =>
              `Cluster ${i + 1} (${c.access_pattern}, PMI=${c.pmi_score.toFixed(2)}):\n  Files: ${c.files.slice(0, 8).join(", ")}${c.files.length > 8 ? ` (+${c.files.length - 8} more)` : ""}`
          )
          .join("\n")
      : "(no clusters)";

  // Numbered reads
  const readsSection =
    request.suggested_reads.length > 0
      ? request.suggested_reads
          .map(
            (r, i) =>
              `Read ${i + 1}: ${r.path} (priority=${r.priority}${r.surprise ? ", surprise" : ""})`
          )
          .join("\n")
      : "(no suggested reads)";

  // Context package content (truncated)
  const contextSection = request.context_package_content
    ? `Context Package Content (most recent):\n"""\n${request.context_package_content.slice(0, 3000)}\n"""\n`
    : "";

  // Evidence sources
  const evidenceSection =
    request.evidence_sources.length > 0
      ? request.evidence_sources.join(", ")
      : "(none)";

  return `Synthesize context for the following project data. Use the output_context_synthesis tool.

QUERY: "${request.query}"
STATUS: ${request.status}
WORK TEMPO: ${request.work_tempo}

WORK CLUSTERS (${request.clusters.length} total):
${clustersSection}

SUGGESTED READS (${request.suggested_reads.length} total):
${readsSection}

${contextSection}EVIDENCE SOURCES: ${evidenceSection}

Analyze this data and use the output_context_synthesis tool.`;
}

/**
 * Tool use content block interface
 */
interface ToolUseBlock {
  type: "tool_use";
  id: string;
  name: string;
  input: {
    one_liner: string;
    narrative: string;
    cluster_names: string[];
    cluster_interpretations: string[];
    suggested_read_reasons: string[];
  };
}

/**
 * Synthesize context for a project from curated deterministic data
 */
export async function synthesizeContext(
  client: Anthropic,
  request: ContextSynthesisRequest
): Promise<ContextSynthesisResponse> {
  const response = await client.messages.create({
    model: MODEL,
    max_tokens: 1024,
    system: buildSystemPrompt(request.clusters.length, request.suggested_reads.length),
    messages: [
      {
        role: "user",
        content: buildPrompt(request),
      },
    ],
    tools: [CONTEXT_SYNTHESIS_TOOL],
    tool_choice: { type: "tool", name: "output_context_synthesis" },
  });

  const toolUse = response.content.find(
    (block): block is ToolUseBlock => block.type === "tool_use"
  );

  if (!toolUse) {
    throw new Error("No tool_use block in response - expected structured output");
  }

  const result = {
    one_liner: toolUse.input.one_liner,
    narrative: toolUse.input.narrative,
    cluster_names: toolUse.input.cluster_names,
    cluster_interpretations: toolUse.input.cluster_interpretations,
    suggested_read_reasons: toolUse.input.suggested_read_reasons,
    model_used: MODEL,
  };

  return ContextSynthesisResponseSchema.parse(result);
}
