/**
 * Chain Naming Agent
 *
 * Uses Claude Haiku to generate meaningful names for session chains
 * based on files touched, session count, and context.
 *
 * Pattern: tool_choice for guaranteed structured JSON output
 */

import type Anthropic from "@anthropic-ai/sdk";
import type {
  ChainNamingRequest,
  ChainNamingResponse,
  ABTestResult,
  QualityComparison,
} from "../types/shared";
import { ChainNamingResponseSchema, ABTestResultSchema } from "../types/shared";

/**
 * System prompt with naming rules from canonical spec
 */
const CHAIN_NAMING_SYSTEM_PROMPT = `You are a session naming specialist. Given information about a conversation chain, generate a descriptive name.

PRIORITY SIGNAL (when available):
- user_conversation_excerpt: This is the BEST signal - actual user messages from the session.
  Extract the user's intent from their words. Example: If user says "Help me fix the login redirect bug",
  name it "Fixed login redirect bug" not "Authentication work".

NAMING RULES:
1. Be SPECIFIC. "Fixed bug" is bad. "Fixed auth redirect loop" is good.
2. If user_conversation_excerpt is provided, prioritize it over file paths.
3. Use file paths to infer context when no user intent is available:
   - Files in "tests/" → likely testing
   - Files in "docs/" → likely documentation
   - Mix of src and test → likely feature or refactor
4. If files are from multiple unrelated areas, use the dominant theme.
5. If unclear, set confidence < 0.5 and use "unknown" category.

EXAMPLES:
- user_conversation_excerpt: "Port the Python indexer to Rust" → "Python to Rust indexer port" (feature, 0.95)
- user_conversation_excerpt: "Help me debug why auth is failing" → "Fixed authentication issue" (bug-fix, 0.9)
- Files only: [auth.py, login.py, tests/test_auth.py] → "Authentication flow work" (refactor, 0.7)
- Files: [README.md, CHANGELOG.md] → "Updated documentation" (documentation, 0.95)
- Files: [many disparate files] → "General codebase work" (unknown, 0.3)`;

/**
 * Tool definition for structured output
 * Forces Claude to return JSON matching our schema
 */
export const CHAIN_NAMING_TOOL: Anthropic.Tool = {
  name: "output_chain_name",
  description: "Output the chain naming analysis results",
  input_schema: {
    type: "object",
    properties: {
      generated_name: {
        type: "string",
        description: "Short descriptive name (3-6 words)",
      },
      category: {
        type: "string",
        enum: [
          "bug-fix",
          "feature",
          "refactor",
          "research",
          "cleanup",
          "documentation",
          "testing",
          "unknown",
        ],
        description: "Category of work performed",
      },
      confidence: {
        type: "number",
        minimum: 0,
        maximum: 1,
        description: "Confidence in the naming (0.0-1.0)",
      },
    },
    required: ["generated_name", "category", "confidence"],
  },
};

/**
 * Build the user prompt with chain context
 *
 * When first_user_intent is available, it provides the BEST signal for naming:
 * this is what the user actually said they wanted to do in the first session.
 */
export function buildPrompt(request: ChainNamingRequest): string {
  // Build enrichment section if any enrichment fields are present
  const enrichmentLines: string[] = [];

  if (request.first_user_intent) {
    // This is the highest-signal field - user's actual words
    enrichmentLines.push(`- user_conversation_excerpt: """${request.first_user_intent}"""`);
  }

  if (request.tools_used && Object.keys(request.tools_used).length > 0) {
    enrichmentLines.push(`- tools_used: ${JSON.stringify(request.tools_used)}`);
  }

  if (request.commit_messages && request.commit_messages.length > 0) {
    enrichmentLines.push(`- commit_messages: ${JSON.stringify(request.commit_messages)}`);
  }

  const enrichmentSection = enrichmentLines.length > 0
    ? `\nENRICHMENT (high-signal context):\n${enrichmentLines.join("\n")}\n`
    : "";

  return `Analyze this conversation chain and provide a descriptive name.

INPUT:
- chain_id: ${request.chain_id}
- files_touched: ${JSON.stringify(request.files_touched)}
- session_count: ${request.session_count}
- recent_sessions: ${JSON.stringify(request.recent_sessions)}
${enrichmentSection}
PRIORITY: If user_conversation_excerpt is available, prioritize it over file paths for naming.
The excerpt shows what the user actually said they wanted to do.

Use the output_chain_name tool to provide your analysis.`;
}

/**
 * Interface for tool_use content block
 */
interface ToolUseBlock {
  type: "tool_use";
  id: string;
  name: string;
  input: {
    generated_name: string;
    category: string;
    confidence: number;
  };
}

/**
 * Generate a meaningful name for a chain using Claude Haiku
 *
 * @param client - Anthropic SDK client
 * @param request - Chain naming request with context
 * @returns Validated chain naming response
 * @throws Error if response doesn't contain valid tool_use
 */
export async function nameChain(
  client: Anthropic,
  request: ChainNamingRequest
): Promise<ChainNamingResponse> {
  const response = await client.messages.create({
    model: "claude-haiku-4-5-20251001",
    max_tokens: 256,
    system: CHAIN_NAMING_SYSTEM_PROMPT,
    messages: [
      {
        role: "user",
        content: buildPrompt(request),
      },
    ],
    tools: [CHAIN_NAMING_TOOL],
    tool_choice: { type: "tool", name: "output_chain_name" },
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
    generated_name: toolUse.input.generated_name,
    category: toolUse.input.category,
    confidence: toolUse.input.confidence,
    model_used: "claude-haiku-4-5-20251001",
  };

  // Validate against Zod schema - throws if invalid
  return ChainNamingResponseSchema.parse(result);
}

// ============================================================================
// A/B Test Functions
// ============================================================================

/**
 * Build prompt with explicit user intent override
 * Used for A/B testing different input sources
 */
export function buildPromptWithIntent(
  request: ChainNamingRequest,
  userIntent: string | undefined
): string {
  const enrichmentLines: string[] = [];

  if (userIntent) {
    enrichmentLines.push(`- user_conversation_excerpt: """${userIntent}"""`);
  }

  if (request.tools_used && Object.keys(request.tools_used).length > 0) {
    enrichmentLines.push(`- tools_used: ${JSON.stringify(request.tools_used)}`);
  }

  if (request.commit_messages && request.commit_messages.length > 0) {
    enrichmentLines.push(`- commit_messages: ${JSON.stringify(request.commit_messages)}`);
  }

  const enrichmentSection = enrichmentLines.length > 0
    ? `\nENRICHMENT (high-signal context):\n${enrichmentLines.join("\n")}\n`
    : "";

  return `Analyze this conversation chain and provide a descriptive name.

INPUT:
- chain_id: ${request.chain_id}
- files_touched: ${JSON.stringify(request.files_touched)}
- session_count: ${request.session_count}
- recent_sessions: ${JSON.stringify(request.recent_sessions)}
${enrichmentSection}
PRIORITY: If user_conversation_excerpt is available, prioritize it over file paths for naming.
The excerpt shows what the user actually said they wanted to do.

Use the output_chain_name tool to provide your analysis.`;
}

/**
 * Name a chain with explicit user intent string
 * Helper for A/B testing
 */
async function nameChainWithIntent(
  client: Anthropic,
  request: ChainNamingRequest,
  userIntent: string | undefined
): Promise<ChainNamingResponse> {
  const response = await client.messages.create({
    model: "claude-haiku-4-5-20251001",
    max_tokens: 256,
    system: CHAIN_NAMING_SYSTEM_PROMPT,
    messages: [
      {
        role: "user",
        content: buildPromptWithIntent(request, userIntent),
      },
    ],
    tools: [CHAIN_NAMING_TOOL],
    tool_choice: { type: "tool", name: "output_chain_name" },
  });

  const toolUse = response.content.find(
    (block): block is ToolUseBlock => block.type === "tool_use"
  );

  if (!toolUse) {
    throw new Error("No tool_use block in response - expected structured output");
  }

  const result = {
    chain_id: request.chain_id,
    generated_name: toolUse.input.generated_name,
    category: toolUse.input.category,
    confidence: toolUse.input.confidence,
    model_used: "claude-haiku-4-5-20251001",
  };

  return ChainNamingResponseSchema.parse(result);
}

/**
 * Compare quality between two naming results
 */
function compareQuality(
  firstResult: ChainNamingResponse,
  fullResult: ChainNamingResponse
): QualityComparison {
  const confidenceDelta = fullResult.confidence - firstResult.confidence;
  const nameLengthDelta =
    fullResult.generated_name.length - firstResult.generated_name.length;

  // Determine winner based on confidence (primary) and name length (secondary)
  let winner: "first_message" | "full_excerpt" | "tie";

  if (Math.abs(confidenceDelta) > 0.1) {
    // Significant confidence difference
    winner = confidenceDelta > 0 ? "full_excerpt" : "first_message";
  } else if (Math.abs(nameLengthDelta) > 10) {
    // Longer names tend to be more specific (within reason)
    winner = nameLengthDelta > 0 ? "full_excerpt" : "first_message";
  } else {
    winner = "tie";
  }

  return {
    winner,
    confidence_delta: confidenceDelta,
    name_length_delta: nameLengthDelta,
  };
}

/**
 * Run A/B test comparing first_message vs full_excerpt naming quality
 *
 * Calls Haiku twice with different inputs and compares results.
 * Used to validate whether full conversation context improves naming.
 *
 * @param client - Anthropic SDK client
 * @param request - Chain naming request with both first_user_message and conversation_excerpt
 * @returns A/B test result with both responses and quality comparison
 */
export async function nameChainAB(
  client: Anthropic,
  request: ChainNamingRequest
): Promise<ABTestResult> {
  // Run both naming approaches
  const [firstMessageResult, fullExcerptResult] = await Promise.all([
    nameChainWithIntent(client, request, request.first_user_message),
    nameChainWithIntent(client, request, request.conversation_excerpt),
  ]);

  const result = {
    chain_id: request.chain_id,
    first_message_result: firstMessageResult,
    full_excerpt_result: fullExcerptResult,
    quality_comparison: compareQuality(firstMessageResult, fullExcerptResult),
  };

  return ABTestResultSchema.parse(result);
}
