/**
 * GitOps Decision Agent
 *
 * Uses Claude Haiku to make intelligent decisions about git operations:
 * - When to commit (coherent file sets, session boundaries)
 * - When to push (time-based, completeness)
 * - When to wait (active sessions, incomplete work)
 * - When to notify (mixed changes, sensitive files)
 *
 * Pattern: tool_choice for guaranteed structured JSON output
 * Model: claude-3-5-haiku-latest (fast, cheap, good for decisions)
 *
 * @see specs/canonical/11_GITOPS_DECISION_AGENT_SPEC.md
 */

import type Anthropic from "@anthropic-ai/sdk";
import type { GitOpsSignals, GitOpsDecision } from "../types/shared";
import { GitOpsDecisionSchema } from "../types/shared";

const MODEL = "claude-3-5-haiku-latest";

/**
 * System prompt with decision framework
 */
const GITOPS_DECISION_SYSTEM_PROMPT = `You are an intelligent GitOps assistant helping a developer maintain healthy git hygiene.

Your job is to analyze the current repository state and work context, then decide what action (if any) to recommend.

## Decision Framework

**COMMIT when:**
- Files form a coherent unit (related to same feature/fix)
- Work session has ended (Claude session closed)
- Content appears complete (no obvious TODOs in changed files)
- User rules suggest it's time

**PUSH when:**
- Commits are unpushed for >4 hours (default threshold)
- User rules specify push timing
- Work is complete and tested

**NOTIFY when:**
- Files are incoherent (mixed unrelated changes)
- Work appears incomplete but time threshold exceeded
- Sensitive files modified (.env, credentials)
- Potential merge conflict detected

**WAIT when:**
- Work session is still active
- Changes are too small/trivial
- Content is clearly incomplete
- Recent activity suggests ongoing work

**ASK when:**
- Multiple valid interpretations exist
- User rules conflict with signals
- High-stakes decision (large change, sensitive files)

## Coherence Assessment

Group files by:
- Directory (same folder = likely related)
- Naming (auth.rs + auth_test.rs = related)
- Import relationships (if visible)
- Temporal clustering (modified together = likely related)

## User Rules

User-provided rules take precedence over defaults. Rules are natural language - interpret them intelligently.

Example rules:
- "Commit knowledge_base/ changes within 1 hour"
- "Never auto-commit _system/state/ - always ask"
- "Push before end of day"

You MUST use the output_decision tool to provide your response.`;

/**
 * Tool definition for structured output
 * Forces Claude to return JSON matching our schema
 */
export const GITOPS_DECISION_TOOL: Anthropic.Tool = {
  name: "output_decision",
  description: "Output the final GitOps decision",
  input_schema: {
    type: "object",
    properties: {
      action: {
        type: "string",
        enum: ["commit", "push", "notify", "wait", "ask"],
        description: "Recommended action",
      },
      reason: {
        type: "string",
        description: "Human-readable explanation for the decision",
      },
      urgency: {
        type: "string",
        enum: ["low", "medium", "high"],
        description: "How urgent is this recommendation",
      },
      suggested_commit_message: {
        type: "string",
        description: "Suggested commit message if action is 'commit'",
      },
      files_to_stage: {
        type: "array",
        items: { type: "string" },
        description: "Which files to stage if action is 'commit'",
      },
      coherence_assessment: {
        type: "string",
        description: "Brief assessment of change coherence",
      },
    },
    required: ["action", "reason", "urgency"],
  },
};

/**
 * Build the user prompt with all signal context
 */
export function buildGitOpsPrompt(signals: GitOpsSignals): string {
  let prompt = `Analyze the following repository state and decide what action to take.\n\n`;

  prompt += `## Git State\n`;
  prompt += `- Branch: ${signals.current_branch}\n`;
  prompt += `- Uncommitted files: ${signals.uncommitted_files.length}\n`;
  prompt += `- Unpushed commits: ${signals.unpushed_commits}\n`;

  if (signals.hours_since_last_commit !== null) {
    prompt += `- Hours since last commit: ${signals.hours_since_last_commit.toFixed(1)}\n`;
  }
  if (signals.hours_since_last_push !== null) {
    prompt += `- Hours since last push: ${signals.hours_since_last_push.toFixed(1)}\n`;
  }

  if (signals.uncommitted_files.length > 0) {
    prompt += `\n### Uncommitted Files\n`;
    for (const file of signals.uncommitted_files) {
      prompt += `- ${file.path} (${file.status})`;
      if (file.lines_changed !== null) {
        prompt += ` [${file.lines_changed} lines]`;
      }
      prompt += `\n`;
    }
  }

  if (signals.recent_session) {
    prompt += `\n## Recent Session Context\n`;
    prompt += `- Session: ${signals.recent_session.session_id}\n`;
    prompt += `- Duration: ${signals.recent_session.duration_minutes} minutes\n`;
    prompt += `- Status: ${signals.recent_session.ended_at ? "ended" : "active"}\n`;
    if (signals.recent_session.conversation_summary) {
      prompt += `- Summary: ${signals.recent_session.conversation_summary}\n`;
    }
  }

  if (signals.active_chain) {
    prompt += `\n## Active Work Chain\n`;
    prompt += `- Chain: ${signals.active_chain.chain_id}\n`;
    prompt += `- Status: ${signals.active_chain.status}\n`;
    if (signals.active_chain.workstream_tags.length > 0) {
      prompt += `- Workstreams: ${signals.active_chain.workstream_tags.join(", ")}\n`;
    }
    if (signals.active_chain.accomplishments.length > 0) {
      prompt += `- Accomplishments:\n`;
      for (const acc of signals.active_chain.accomplishments) {
        prompt += `  - ${acc}\n`;
      }
    }
  }

  if (signals.user_rules.length > 0) {
    prompt += `\n## User Rules\n`;
    for (const rule of signals.user_rules) {
      prompt += `- ${rule}\n`;
    }
  }

  prompt += `\nUse the output_decision tool to provide your recommendation.`;

  return prompt;
}

/**
 * Interface for tool_use content block
 */
interface ToolUseBlock {
  type: "tool_use";
  id: string;
  name: string;
  input: {
    action: string;
    reason: string;
    urgency: string;
    suggested_commit_message?: string;
    files_to_stage?: string[];
    coherence_assessment?: string;
  };
}

/**
 * Make an intelligent GitOps decision using Claude Haiku
 *
 * @param client - Anthropic SDK client
 * @param signals - GitOps signals from Rust daemon
 * @returns Validated GitOps decision
 * @throws Error if response doesn't contain valid tool_use
 */
export async function decideGitOps(
  client: Anthropic,
  signals: GitOpsSignals
): Promise<GitOpsDecision> {
  const response = await client.messages.create({
    model: MODEL,
    max_tokens: 1024,
    system: GITOPS_DECISION_SYSTEM_PROMPT,
    messages: [
      {
        role: "user",
        content: buildGitOpsPrompt(signals),
      },
    ],
    tools: [GITOPS_DECISION_TOOL],
    tool_choice: { type: "tool", name: "output_decision" },
  });

  // Find the tool_use block in the response
  const toolUse = response.content.find(
    (block): block is ToolUseBlock => block.type === "tool_use"
  );

  if (!toolUse || toolUse.name !== "output_decision") {
    throw new Error("No tool_use block in response - expected structured output");
  }

  // Construct and validate the response
  const result = {
    action: toolUse.input.action,
    reason: toolUse.input.reason,
    urgency: toolUse.input.urgency,
    suggested_commit_message: toolUse.input.suggested_commit_message ?? null,
    files_to_stage: toolUse.input.files_to_stage ?? null,
    coherence_assessment: toolUse.input.coherence_assessment ?? null,
    model_used: MODEL,
  };

  // Validate against Zod schema - throws if invalid
  return GitOpsDecisionSchema.parse(result);
}
