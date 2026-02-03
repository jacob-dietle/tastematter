# GitOps Decision Agent Specification

**Status:** DRAFT
**Created:** 2026-01-30
**Foundation:**
  - [[05_INTELLIGENCE_LAYER_ARCHITECTURE]]
  - [[06_INTELLIGENT_GITOPS_SPEC]] (in _system/specs/)
  - [[context-git-ops skill]]

---

## Executive Summary

**Problem:** User forgets to commit/push, overhead grows daily. Simple threshold checks ("5 files uncommitted for 24 hours") are not valuable - they're just noise.

**Solution:** An intelligent GitOps Decision Agent that understands:
- Change coherence ("these 5 files are all part of the auth feature")
- Work session context ("Claude session just ended, good commit boundary")
- Content completeness ("this looks like incomplete work, don't commit yet")
- User patterns ("you've been grinding for 4 hours, push and take a break")

**Key Insight:** The intelligence lives in a TypeScript agent with Claude Agent SDK tools, not in Rust threshold checks. Rust collects signals, TypeScript makes decisions.

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         RUST DAEMON (Signal Collector)                   │
│                                                                          │
│   run_sync() phases:                                                     │
│   ├─ Phase 1: Git sync (existing)                                       │
│   ├─ Phase 2: Session parsing (existing)                                │
│   ├─ Phase 3: Chain building (existing)                                 │
│   ├─ Phase 4: Intel enrichment (existing)                               │
│   └─ Phase 5: GitOps signal collection (NEW)                            │
│        ├─ query_repo_status() → uncommitted, unpushed, branch           │
│        ├─ get_recent_session_activity() → files touched, duration       │
│        ├─ get_active_chain_context() → workstream, accomplishments      │
│        └─ load_user_rules() → promptable rules from YAML                │
│                              │                                           │
│                              ▼                                           │
│              POST /api/intel/gitops-decide                              │
│                    GitOpsSignals                                         │
└─────────────────────────────────────────────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                    TYPESCRIPT INTEL SERVICE                              │
│                                                                          │
│   POST /api/intel/gitops-decide                                         │
│        │                                                                 │
│        ▼                                                                 │
│   ┌─────────────────────────────────────────────────────────────────┐   │
│   │              GitOps Decision Agent (Claude Haiku)                │   │
│   │                                                                  │   │
│   │   SYSTEM PROMPT:                                                 │   │
│   │   "You are an intelligent GitOps assistant. Analyze signals      │   │
│   │    and decide: commit, push, notify, or wait. Consider change    │   │
│   │    coherence, session boundaries, and user rules."               │   │
│   │                                                                  │   │
│   │   TOOLS:                                                         │   │
│   │   ├─ analyze_change_coherence(files) → coherence score, theme    │   │
│   │   ├─ suggest_commit_message(files, context) → message            │   │
│   │   ├─ evaluate_completeness(files, diff_summary) → complete?      │   │
│   │   └─ output_decision(action, reason, details) → structured out   │   │
│   │                                                                  │   │
│   │   tool_choice: { type: "tool", name: "output_decision" }         │   │
│   └─────────────────────────────────────────────────────────────────┘   │
│                              │                                           │
│                              ▼                                           │
│                    GitOpsDecision                                        │
│                    { action, reason, suggested_message, urgency }        │
└─────────────────────────────────────────────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                         RUST DAEMON (Action Handler)                     │
│                                                                          │
│   match decision.action:                                                 │
│     "commit" → store suggestion, surface via CLI                        │
│     "push"   → store suggestion, surface via CLI                        │
│     "notify" → store warning with reason                                │
│     "wait"   → log decision, no action                                  │
│                                                                          │
│   CLI: tastematter gitops status                                        │
│        → Shows pending decisions with agent reasoning                   │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Type Contracts

### Rust → TypeScript (Request)

```rust
// core/src/intelligence/types.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitOpsSignals {
    // Git state
    pub uncommitted_files: Vec<UncommittedFile>,
    pub unpushed_commits: i32,
    pub current_branch: String,
    pub last_commit_timestamp: Option<String>,  // ISO8601
    pub last_push_timestamp: Option<String>,    // ISO8601

    // Session context
    pub recent_session: Option<RecentSessionContext>,
    pub active_chain: Option<ActiveChainContext>,

    // User rules (promptable)
    pub user_rules: Vec<String>,

    // Time context
    pub hours_since_last_commit: Option<f64>,
    pub hours_since_last_push: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UncommittedFile {
    pub path: String,
    pub status: String,  // "modified" | "added" | "deleted" | "renamed"
    pub lines_changed: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentSessionContext {
    pub session_id: String,
    pub ended_at: Option<String>,  // ISO8601, None if still active
    pub files_touched: Vec<String>,
    pub duration_minutes: i32,
    pub conversation_summary: Option<String>,  // From chain summary if available
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveChainContext {
    pub chain_id: String,
    pub workstream_tags: Vec<String>,
    pub accomplishments: Vec<String>,
    pub status: String,  // "in_progress" | "complete" | "paused"
}
```

### TypeScript Schemas (Zod)

```typescript
// intel/src/types/shared.ts

export const UncommittedFileSchema = z.object({
  path: z.string(),
  status: z.enum(["modified", "added", "deleted", "renamed"]),
  lines_changed: z.number().int().nullable(),
});

export const RecentSessionContextSchema = z.object({
  session_id: z.string(),
  ended_at: z.string().nullable(),
  files_touched: z.array(z.string()),
  duration_minutes: z.number().int(),
  conversation_summary: z.string().nullable(),
});

export const ActiveChainContextSchema = z.object({
  chain_id: z.string(),
  workstream_tags: z.array(z.string()),
  accomplishments: z.array(z.string()),
  status: z.enum(["in_progress", "complete", "paused"]),
});

export const GitOpsSignalsSchema = z.object({
  uncommitted_files: z.array(UncommittedFileSchema),
  unpushed_commits: z.number().int(),
  current_branch: z.string(),
  last_commit_timestamp: z.string().nullable(),
  last_push_timestamp: z.string().nullable(),
  recent_session: RecentSessionContextSchema.nullable(),
  active_chain: ActiveChainContextSchema.nullable(),
  user_rules: z.array(z.string()),
  hours_since_last_commit: z.number().nullable(),
  hours_since_last_push: z.number().nullable(),
});
export type GitOpsSignals = z.infer<typeof GitOpsSignalsSchema>;
```

### TypeScript → Rust (Response)

```typescript
// intel/src/types/shared.ts

export const GitOpsActionSchema = z.enum([
  "commit",     // Ready to commit - suggest message
  "push",       // Committed but unpushed - suggest push
  "notify",     // Needs attention - explain why
  "wait",       // Not ready - explain why waiting
  "ask",        // Need user input before deciding
]);
export type GitOpsAction = z.infer<typeof GitOpsActionSchema>;

export const GitOpsUrgencySchema = z.enum(["low", "medium", "high"]);
export type GitOpsUrgency = z.infer<typeof GitOpsUrgencySchema>;

export const GitOpsDecisionSchema = z.object({
  action: GitOpsActionSchema,
  reason: z.string(),              // Human-readable explanation
  urgency: GitOpsUrgencySchema,

  // Optional details based on action
  suggested_commit_message: z.string().nullable(),
  files_to_stage: z.array(z.string()).nullable(),
  coherence_assessment: z.string().nullable(),  // "auth feature", "mixed changes"

  model_used: z.string(),
});
export type GitOpsDecision = z.infer<typeof GitOpsDecisionSchema>;
```

```rust
// core/src/intelligence/types.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GitOpsAction {
    Commit,
    Push,
    Notify,
    Wait,
    Ask,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GitOpsUrgency {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitOpsDecision {
    pub action: GitOpsAction,
    pub reason: String,
    pub urgency: GitOpsUrgency,
    pub suggested_commit_message: Option<String>,
    pub files_to_stage: Option<Vec<String>>,
    pub coherence_assessment: Option<String>,
    pub model_used: String,
}
```

---

## Agent Implementation

### System Prompt

```typescript
// intel/src/agents/gitops-decision.ts

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
```

### Tool Definitions

```typescript
// intel/src/agents/gitops-decision.ts

export const GITOPS_DECISION_TOOLS: Anthropic.Tool[] = [
  {
    name: "analyze_change_coherence",
    description: "Analyze whether uncommitted files form a coherent unit or are mixed unrelated changes",
    input_schema: {
      type: "object",
      properties: {
        files: {
          type: "array",
          items: { type: "string" },
          description: "File paths to analyze"
        }
      },
      required: ["files"]
    }
  },
  {
    name: "suggest_commit_message",
    description: "Generate a conventional commit message based on changed files and context",
    input_schema: {
      type: "object",
      properties: {
        files: {
          type: "array",
          items: { type: "string" },
          description: "Files to be committed"
        },
        context: {
          type: "string",
          description: "Additional context (workstream, accomplishments)"
        }
      },
      required: ["files"]
    }
  },
  {
    name: "output_decision",
    description: "Output the final GitOps decision",
    input_schema: {
      type: "object",
      properties: {
        action: {
          type: "string",
          enum: ["commit", "push", "notify", "wait", "ask"],
          description: "Recommended action"
        },
        reason: {
          type: "string",
          description: "Human-readable explanation for the decision"
        },
        urgency: {
          type: "string",
          enum: ["low", "medium", "high"],
          description: "How urgent is this recommendation"
        },
        suggested_commit_message: {
          type: "string",
          description: "Suggested commit message if action is 'commit'"
        },
        files_to_stage: {
          type: "array",
          items: { type: "string" },
          description: "Which files to stage if action is 'commit'"
        },
        coherence_assessment: {
          type: "string",
          description: "Brief assessment of change coherence"
        }
      },
      required: ["action", "reason", "urgency"]
    }
  }
];
```

### Agent Function

```typescript
// intel/src/agents/gitops-decision.ts

const MODEL = "claude-3-5-haiku-latest";  // Fast, cheap, good for decisions

export async function decideGitOps(
  client: Anthropic,
  signals: GitOpsSignals
): Promise<GitOpsDecision> {
  // Build context-rich prompt
  const userPrompt = buildGitOpsPrompt(signals);

  const response = await client.messages.create({
    model: MODEL,
    max_tokens: 1024,
    system: GITOPS_DECISION_SYSTEM_PROMPT,
    messages: [{ role: "user", content: userPrompt }],
    tools: GITOPS_DECISION_TOOLS,
    tool_choice: { type: "tool", name: "output_decision" },
  });

  // Extract tool use
  const toolUse = response.content.find(
    (block): block is ToolUseBlock => block.type === "tool_use"
  );

  if (!toolUse || toolUse.name !== "output_decision") {
    throw new Error("Agent did not output decision");
  }

  return GitOpsDecisionSchema.parse({
    ...toolUse.input,
    model_used: MODEL,
  });
}

function buildGitOpsPrompt(signals: GitOpsSignals): string {
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
    prompt += `- Status: ${signals.recent_session.ended_at ? 'ended' : 'active'}\n`;
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
```

---

## Endpoint Implementation

```typescript
// intel/src/index.ts (add to existing)

.post(
  "/api/intel/gitops-decide",
  async ({ body, correlationId }) => {
    const startTime = Date.now();
    const signals = GitOpsSignalsSchema.parse(body);

    logger.info({
      correlationId,
      event: "gitops_decide_request",
      uncommitted_count: signals.uncommitted_files.length,
      unpushed_count: signals.unpushed_commits,
      has_session_context: signals.recent_session !== null,
      has_chain_context: signals.active_chain !== null,
      rules_count: signals.user_rules.length,
    });

    const decision = await decideGitOps(anthropicClient, signals);

    logger.info({
      correlationId,
      event: "gitops_decide_response",
      action: decision.action,
      urgency: decision.urgency,
      has_commit_message: decision.suggested_commit_message !== null,
      duration_ms: Date.now() - startTime,
    });

    return decision;
  },
  {
    body: t.Object({
      uncommitted_files: t.Array(t.Object({
        path: t.String(),
        status: t.String(),
        lines_changed: t.Nullable(t.Number()),
      })),
      unpushed_commits: t.Number(),
      current_branch: t.String(),
      last_commit_timestamp: t.Nullable(t.String()),
      last_push_timestamp: t.Nullable(t.String()),
      recent_session: t.Nullable(t.Any()),
      active_chain: t.Nullable(t.Any()),
      user_rules: t.Array(t.String()),
      hours_since_last_commit: t.Nullable(t.Number()),
      hours_since_last_push: t.Nullable(t.Number()),
    }),
  }
)
```

---

## Rust Integration

### IntelClient Method

```rust
// core/src/intelligence/client.rs (add method)

pub async fn decide_gitops(
    &self,
    signals: &GitOpsSignals,
) -> Result<Option<GitOpsDecision>, CoreError> {
    if !self.health_check().await {
        return Ok(None);  // Graceful degradation
    }

    let correlation_id = Uuid::new_v4().to_string();
    let url = format!("{}/api/intel/gitops-decide", self.base_url);

    let response = self.client
        .post(&url)
        .header("X-Correlation-ID", &correlation_id)
        .json(signals)
        .timeout(Duration::from_secs(15))  // Longer timeout for agent
        .send()
        .await;

    match response {
        Ok(resp) if resp.status().is_success() => {
            let decision: GitOpsDecision = resp.json().await
                .map_err(|e| CoreError::IntelError(e.to_string()))?;
            Ok(Some(decision))
        }
        Ok(resp) => {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            tracing::warn!(
                correlation_id = %correlation_id,
                status = %status,
                body = %body,
                "GitOps decide failed"
            );
            Ok(None)
        }
        Err(e) => {
            tracing::warn!(
                correlation_id = %correlation_id,
                error = %e,
                "GitOps decide request failed"
            );
            Ok(None)
        }
    }
}
```

### Signal Collection

```rust
// core/src/daemon/gitops.rs (NEW file)

use crate::capture::git_status::{query_repo_status, GitRepoStatus};
use crate::intelligence::types::{GitOpsSignals, UncommittedFile, RecentSessionContext, ActiveChainContext};

/// Collect all signals needed for GitOps decision.
pub fn collect_gitops_signals(
    repo_path: &Path,
    recent_session: Option<RecentSessionContext>,
    active_chain: Option<ActiveChainContext>,
    user_rules: Vec<String>,
) -> Result<GitOpsSignals, String> {
    let status = query_repo_status(repo_path)?;

    let uncommitted_files: Vec<UncommittedFile> = status.uncommitted_files
        .into_iter()
        .map(|(path, file_status)| UncommittedFile {
            path,
            status: file_status,
            lines_changed: None,  // Could add git diff --numstat
        })
        .collect();

    let hours_since_last_commit = status.last_commit_timestamp
        .map(|ts| (Utc::now() - ts).num_minutes() as f64 / 60.0);

    let hours_since_last_push = status.last_push_timestamp
        .map(|ts| (Utc::now() - ts).num_minutes() as f64 / 60.0);

    Ok(GitOpsSignals {
        uncommitted_files,
        unpushed_commits: status.unpushed_commits,
        current_branch: status.branch,
        last_commit_timestamp: status.last_commit_timestamp.map(|t| t.to_rfc3339()),
        last_push_timestamp: status.last_push_timestamp.map(|t| t.to_rfc3339()),
        recent_session,
        active_chain,
        user_rules,
        hours_since_last_commit,
        hours_since_last_push,
    })
}
```

### CLI Command

```rust
// core/src/main.rs (add subcommand)

/// GitOps intelligent decision-making
#[derive(Subcommand)]
enum GitOpsCommands {
    /// Get intelligent recommendation for current repo state
    Decide {
        /// Repository path (default: current directory)
        #[arg(long, default_value = ".")]
        repo: String,

        /// Output format: json or human
        #[arg(long, default_value = "human")]
        format: String,
    },

    /// Show current git state signals (without calling agent)
    Signals {
        /// Repository path
        #[arg(long, default_value = ".")]
        repo: String,
    },
}

// Handler:
GitOpsCommands::Decide { repo, format } => {
    let signals = collect_gitops_signals(
        Path::new(&repo),
        None,  // TODO: Get from recent session
        None,  // TODO: Get from active chain
        load_user_rules()?,  // From ~/.context-os/gitops-rules.yaml
    )?;

    let client = IntelClient::default();

    match client.decide_gitops(&signals).await? {
        Some(decision) => {
            if format == "json" {
                println!("{}", serde_json::to_string_pretty(&decision)?);
            } else {
                print_decision_human(&decision);
            }
        }
        None => {
            eprintln!("Intel service unavailable - cannot make intelligent decision");
            eprintln!("Run `tastematter gitops signals` to see raw state");
            std::process::exit(1);
        }
    }
}

fn print_decision_human(decision: &GitOpsDecision) {
    let icon = match decision.action {
        GitOpsAction::Commit => "📝",
        GitOpsAction::Push => "🚀",
        GitOpsAction::Notify => "⚠️",
        GitOpsAction::Wait => "⏳",
        GitOpsAction::Ask => "❓",
    };

    let urgency_color = match decision.urgency {
        GitOpsUrgency::High => "\x1b[31m",    // Red
        GitOpsUrgency::Medium => "\x1b[33m",  // Yellow
        GitOpsUrgency::Low => "\x1b[32m",     // Green
    };

    println!("\n{} Recommendation: {:?}", icon, decision.action);
    println!("{}Urgency: {:?}\x1b[0m", urgency_color, decision.urgency);
    println!("\nReason: {}", decision.reason);

    if let Some(msg) = &decision.suggested_commit_message {
        println!("\nSuggested commit message:");
        println!("  {}", msg);
    }

    if let Some(assessment) = &decision.coherence_assessment {
        println!("\nChange coherence: {}", assessment);
    }
}
```

---

## User Rules Format

```yaml
# ~/.context-os/gitops-rules.yaml

rules:
  # Time-based rules
  - "Remind me to commit if I have uncommitted changes for more than 2 hours"
  - "Push before end of day (after 5 PM)"

  # Path-based rules
  - "Commit knowledge_base/ changes within 1 hour of modification"
  - "Never auto-commit _system/state/ files - always ask first"
  - "Group transcript commits by engagement folder"

  # Content-based rules
  - "Don't suggest commit if files contain TODO or FIXME markers"
  - "Treat .env and credentials files as sensitive - always notify, never auto-commit"

  # Session-based rules
  - "Good time to commit when Claude session ends"
  - "Don't interrupt if I'm in an active session"

# Default thresholds (agent interprets these)
defaults:
  uncommitted_warn_hours: 2
  unpushed_warn_hours: 4
  max_coherent_files: 10  # More than this = likely should split
```

---

## Implementation Phases

### Phase 1: TypeScript Agent (~150 lines)
**File:** `intel/src/agents/gitops-decision.ts`
- System prompt with decision framework
- Tool definitions (analyze_coherence, suggest_message, output_decision)
- `decideGitOps()` function with prompt builder
- Tests: 8 unit tests

### Phase 2: TypeScript Types + Endpoint (~80 lines)
**Files:** `intel/src/types/shared.ts`, `intel/src/index.ts`
- Zod schemas for signals and decision
- HTTP endpoint with logging
- Tests: 5 integration tests

### Phase 3: Rust Types (~60 lines)
**File:** `core/src/intelligence/types.rs`
- GitOpsSignals, GitOpsDecision structs
- Enum types for actions and urgency
- Tests: 3 serialization tests

### Phase 4: Rust Signal Collection (~100 lines)
**Files:** `core/src/capture/git_status.rs`, `core/src/daemon/gitops.rs`
- `query_repo_status()` - git status + log queries
- `collect_gitops_signals()` - aggregate all signals
- Tests: 5 unit tests

### Phase 5: Rust IntelClient Method (~40 lines)
**File:** `core/src/intelligence/client.rs`
- `decide_gitops()` method following existing pattern
- Graceful degradation
- Tests: 3 tests

### Phase 6: CLI Commands (~80 lines)
**File:** `core/src/main.rs`
- `tastematter gitops decide` - call agent
- `tastematter gitops signals` - show raw signals
- Human-friendly output formatting
- Tests: 4 tests

### Phase 7: Daemon Integration (~40 lines)
**File:** `core/src/daemon/sync.rs`
- Add Phase 5: GitOps signal collection
- Call agent if conditions met
- Store decision in SyncResult
- Tests: 3 integration tests

---

## Test Strategy

### Unit Tests (TypeScript)
```typescript
describe("GitOps Decision Agent", () => {
  test("recommends commit for coherent file set", async () => {
    const signals = mockSignals({
      uncommitted_files: [
        { path: "src/auth.rs", status: "modified" },
        { path: "src/auth_test.rs", status: "modified" },
      ],
      hours_since_last_commit: 3,
    });
    const decision = await decideGitOps(mockClient, signals);
    expect(decision.action).toBe("commit");
    expect(decision.coherence_assessment).toContain("auth");
  });

  test("recommends wait for active session", async () => {
    const signals = mockSignals({
      recent_session: { ended_at: null },  // Still active
    });
    const decision = await decideGitOps(mockClient, signals);
    expect(decision.action).toBe("wait");
  });

  test("respects user rules", async () => {
    const signals = mockSignals({
      uncommitted_files: [{ path: "_system/state/pipeline.yaml", status: "modified" }],
      user_rules: ["Never auto-commit _system/state/ - always ask"],
    });
    const decision = await decideGitOps(mockClient, signals);
    expect(decision.action).toBe("ask");
  });
});
```

### E2E Test
```bash
# 1. Start intel service
cd apps/tastematter/intel && bun run src/index.ts &

# 2. Create test repo with uncommitted changes
mkdir /tmp/test-gitops && cd /tmp/test-gitops
git init && echo "test" > file.txt && git add . && git commit -m "init"
echo "change" >> file.txt

# 3. Run gitops decide
tastematter gitops decide --repo /tmp/test-gitops

# Expected: Intelligent recommendation based on signals
```

---

## Success Criteria

- [ ] Agent makes contextually appropriate decisions (not just threshold checks)
- [ ] Suggested commit messages are relevant to actual changes
- [ ] User rules are respected and interpreted intelligently
- [ ] Session context influences decisions (don't interrupt active work)
- [ ] Coherence assessment groups related files correctly
- [ ] CLI output is human-friendly with clear reasoning
- [ ] Graceful degradation when intel service unavailable

---

## Cost Analysis

**Model:** claude-3-5-haiku-latest
**Estimated cost per decision:** ~$0.0005 (500 input tokens, 200 output)
**Daemon frequency:** Every 5 minutes when changes detected
**Daily cost (heavy usage):** ~$0.15/day

---

## Future Extensions

After MVP proves value:
1. **Desktop notifications** - Toast when urgency is high
2. **Auto-execute** - Option to auto-commit/push for low-risk decisions
3. **Learning** - Track which decisions user accepts/rejects
4. **Multi-repo** - Coordinate across related repositories
5. **Team mode** - Share rules, coordinate pushes
