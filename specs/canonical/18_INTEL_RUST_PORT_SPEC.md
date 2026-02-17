---
title: "Intel Service → Embedded Rust Port"
type: architecture-spec
created: 2026-02-16
last_updated: 2026-02-16
status: approved
foundation:
  - "[[canonical/05_INTELLIGENCE_LAYER_ARCHITECTURE]]"
  - "[[canonical/12_CONTEXT_RESTORATION_API_SPEC]]"
  - "[[canonical/15_CONTEXT_RESTORE_PHASE3_EPISTEMIC_COMPRESSION]]"
supersedes:
  - "intel/ TypeScript sidecar (to be archived after port)"
related:
  - "[[core/src/intelligence/client.rs]]"
  - "[[core/src/intelligence/types.rs]]"
  - "[[core/src/intelligence/cache.rs]]"
  - "[[intel/src/agents/*.ts]]"
tags:
  - tastematter
  - intelligence-layer
  - rust-port
  - canonical
---

# Intel Service → Embedded Rust Port

## Executive Summary

Port the TypeScript intel sidecar (`intel/`) to an embedded Rust module within the `tastematter` binary. Every agent follows an identical pattern (system prompt + tool_choice → parse structured JSON). No agent uses multi-turn conversation or tool dispatch. This is 7 formatted API calls, not an agent framework.

**Decision driver:** Single binary distribution. No Bun dependency. No sidecar process management. Users get synthesis by setting `ANTHROPIC_API_KEY`.

**What changes:** `IntelClient` calls Anthropic API directly instead of `localhost:3002`.
**What doesn't change:** `QueryEngine.intel_client`, `build_synthesis_request()`, `merge_synthesis()`, graceful degradation, all callers.

**Scope:** ~1,100 lines of new Rust. Eliminates 7,188 lines of TypeScript source + tests + runtime dependency.

---

## Design Decision Record

### Context

The intel service is a TypeScript + Bun + Elysia HTTP server running on localhost:3002. The Rust binary calls it via HTTP for LLM-powered features (chain naming, context synthesis, etc.). This creates a distribution problem: users who install the Rust binary via `curl | bash` get no synthesis because the sidecar isn't distributed.

### Options Considered

| Option | Verdict | Reason |
|--------|---------|--------|
| Bun compile → ship 2 binaries | Rejected | Complex distribution, process management |
| CF Worker cloud-hosted | Rejected | Pays per-user LLM costs, latency |
| Embed in Rust (direct Anthropic API) | **Accepted** | Single binary, zero deps, user provides key |
| Keep sidecar, distribute later | Rejected | Defers the problem, blocks Phase 3 value |

### Future-Proofing

The GitOps agent feature (Roadmap Phase 4) WILL need local tool dispatch (git commands, file reads). That is a fundamentally different pattern — multi-turn agent loops with tool execution. When that time comes:

1. The generic `call_anthropic()` built here becomes the LLM call primitive for the agent loop
2. Tool dispatch is added on top (not a replacement)
3. No throwaway work — this port is a foundation, not a detour

---

## Architecture

### Before (Sidecar)

```
tastematter binary                    intel sidecar (bun)
┌──────────────────┐                 ┌──────────────────┐
│ IntelClient      │──HTTP POST──→   │ Elysia router    │
│   .name_chain()  │                 │   /api/intel/*    │
│   .synthesize()  │                 │   ↓               │
│   .summarize()   │                 │ Anthropic SDK     │
│                  │←──JSON──────    │   ↓               │
│ merge_synthesis()│                 │ Zod validation    │
└──────────────────┘                 └──────────────────┘
```

### After (Embedded)

```
tastematter binary
┌──────────────────────────────────────┐
│ IntelClient                          │
│   .name_chain()  ──→ call_anthropic()│──→ api.anthropic.com
│   .synthesize()  ──→ call_anthropic()│
│   .summarize()   ──→ call_anthropic()│
│                                      │
│ merge_synthesis() (unchanged)        │
└──────────────────────────────────────┘
```

### Key Abstraction: `call_anthropic()`

One generic function handles all 7 agents:

```rust
/// Generic Anthropic Messages API call with tool_choice
async fn call_anthropic(
    http_client: &Client,
    api_key: &str,
    model: &str,
    max_tokens: u32,
    system_prompt: &str,
    user_message: &str,
    tool: &ToolDefinition,
) -> Result<serde_json::Value, AnthropicError>
```

Each agent is then:
1. Build system prompt (string)
2. Build user message (string from request data)
3. Define tool schema (JSON)
4. Call `call_anthropic()` → get tool_use input as `serde_json::Value`
5. Deserialize into typed response struct

---

## Type Contracts

### Anthropic API Types (new)

```rust
/// Anthropic Messages API request
#[derive(Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    system: String,
    messages: Vec<Message>,
    tools: Vec<ToolDefinition>,
    tool_choice: ToolChoice,
}

#[derive(Serialize)]
struct Message {
    role: String,        // "user"
    content: String,
}

#[derive(Serialize)]
struct ToolDefinition {
    name: String,
    description: String,
    input_schema: serde_json::Value,
}

#[derive(Serialize)]
struct ToolChoice {
    #[serde(rename = "type")]
    choice_type: String,  // "tool"
    name: String,
}

/// Anthropic Messages API response (partial — only what we need)
#[derive(Deserialize)]
struct AnthropicResponse {
    content: Vec<ContentBlock>,
    usage: Usage,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum ContentBlock {
    #[serde(rename = "tool_use")]
    ToolUse { id: String, name: String, input: serde_json::Value },
    #[serde(other)]
    Other,
}

#[derive(Deserialize)]
struct Usage {
    input_tokens: u32,
    output_tokens: u32,
}
```

### Existing Types (unchanged)

All request/response types in `intelligence/types.rs` remain as-is:
- `ChainNamingRequest` / `ChainNamingResponse`
- `ChainSummaryRequest` / `ChainSummaryResponse`
- `ContextSynthesisRequest` / `ContextSynthesisResponse`
- `ClusterInput`, `SuggestedReadInput`

### IntelClient (modified)

```rust
pub struct IntelClient {
    http_client: Client,
    api_key: String,           // NEW: Anthropic API key
    model: String,             // NEW: default "claude-haiku-4-5-20251001"
}

impl IntelClient {
    /// Create from environment variable
    pub fn from_env() -> Option<Self> {
        let api_key = std::env::var("ANTHROPIC_API_KEY").ok()?;
        Some(Self {
            http_client: Client::builder()
                .timeout(Duration::from_secs(15))
                .build()
                .expect("Failed to build HTTP client"),
            api_key,
            model: "claude-haiku-4-5-20251001".to_string(),
        })
    }
}
```

**Initialization in main.rs:**
```rust
// Before: IntelClient::default() (always created, fails silently)
// After: IntelClient::from_env() (None if no API key — no wasted connections)
let query_engine = match IntelClient::from_env() {
    Some(client) => QueryEngine::new(db).with_intel(client),
    None => QueryEngine::new(db),
};
```

---

## Agent Inventory

Each agent is a module with: system prompt, tool schema, prompt builder, caller function.

| Agent | TS File | Rust Module | Model | max_tokens | Priority |
|-------|---------|-------------|-------|------------|----------|
| context-synthesis | 194 lines | `agents/context_synthesis.rs` | haiku | 1024 | P0 (context restore) |
| chain-naming | 333 lines | `agents/chain_naming.rs` | haiku | 256 | P0 (daemon uses it) |
| chain-summary | 198 lines | `agents/chain_summary.rs` | haiku | 512 | P1 |
| commit-analysis | 175 lines | `agents/commit_analysis.rs` | sonnet | 512 | P2 (not called yet) |
| session-summary | 155 lines | `agents/session_summary.rs` | haiku | 512 | P2 (not called yet) |
| insights | 228 lines | `agents/insights.rs` | sonnet | 1024 | P2 (not called yet) |
| gitops-decision | 251 lines | `agents/gitops_decision.rs` | sonnet | 512 | P3 (future) |

**Implementation order:** P0 first (unblocks Phase 3 context restore), P1 next (daemon chain naming), P2/P3 deferred.

---

## Implementation Phases

### Phase 1: Core Abstraction + Context Synthesis (P0)

**Goal:** `tastematter context "nickel"` works with `ANTHROPIC_API_KEY` set, no sidecar needed.

**Files to create:**
```
core/src/intelligence/
├── mod.rs              # Add new modules
├── anthropic.rs        # NEW: call_anthropic() + API types (~150 lines)
└── agents/
    ├── mod.rs          # NEW: agent module registry
    └── context_synthesis.rs  # NEW: prompt + tool schema + caller (~80 lines)
```

**Files to modify:**
```
core/src/intelligence/client.rs   # IntelClient::from_env(), replace synthesize_context()
core/src/main.rs                  # IntelClient::from_env() instead of IntelClient::default()
```

**Tests (TDD):**
1. `anthropic.rs`: Serialize request → verify JSON matches Anthropic API spec
2. `anthropic.rs`: Deserialize mock response → extract tool_use input
3. `anthropic.rs`: Handle error responses (400, 401, 429, 500) → graceful degradation
4. `context_synthesis.rs`: Build prompt from request → verify format matches TS output
5. `context_synthesis.rs`: Parse mock tool_use → deserialize to ContextSynthesisResponse
6. `client.rs`: `from_env()` returns None when no API key

**Success criteria:** E2E test — `ANTHROPIC_API_KEY=... tastematter context "nickel"` returns populated synthesis fields.

**Estimated lines:** ~300 new, ~50 modified

### Phase 2: Chain Naming (P0)

**Goal:** `tastematter intel name-chain` works without sidecar. Daemon auto-names chains.

**Files to create:**
```
core/src/intelligence/agents/chain_naming.rs  # NEW: prompt + schema + caller (~100 lines)
```

**Files to modify:**
```
core/src/intelligence/client.rs   # Replace name_chain() to use direct API
```

**Tests (TDD):**
1. Build prompt from ChainNamingRequest → verify format
2. Parse mock tool_use → deserialize to ChainNamingResponse
3. Handle A/B test variant (if keeping)

**Estimated lines:** ~120 new, ~30 modified

### Phase 3: Chain Summary (P1)

**Files to create:**
```
core/src/intelligence/agents/chain_summary.rs  # NEW (~80 lines)
```

**Tests:** Same pattern as Phase 2.

**Estimated lines:** ~100 new, ~20 modified

### Phase 4: Remaining Agents (P2, deferred)

Port commit-analysis, session-summary, insights, gitops-decision when needed. Same mechanical pattern. ~80-100 lines each.

### Phase 5: Archive TypeScript Intel Service

After all active agents are ported and verified:
1. Remove `IntelClient` localhost fallback
2. Archive `intel/` directory (don't delete — reference material)
3. Remove bun dependency from CLAUDE.md docs
4. Update install/setup documentation

---

## Prompt Management

### Development: Load from files

During development, prompts load from disk for fast iteration:

```rust
// In debug builds, load from file
#[cfg(debug_assertions)]
fn system_prompt() -> String {
    std::fs::read_to_string("src/intelligence/prompts/context_synthesis.txt")
        .unwrap_or_else(|_| CONTEXT_SYNTHESIS_PROMPT.to_string())
}

// In release builds, embed at compile time
#[cfg(not(debug_assertions))]
fn system_prompt() -> &'static str {
    CONTEXT_SYNTHESIS_PROMPT
}
```

### Production: Embedded strings

```rust
const CONTEXT_SYNTHESIS_PROMPT: &str = r#"You are a context analyst for a developer's project...
..."#;
```

This gives hot-reload during dev (edit txt, re-run) and zero-dep in production (compiled in).

---

## Error Handling

### Graceful Degradation (preserved)

The existing pattern — `Ok(None)` on any failure — is preserved exactly:

```rust
// API key not set → None (no client created)
// API returns 401 → Ok(None)
// API returns 429 → Ok(None), log rate limit
// API returns 500 → Ok(None)
// Network timeout → Ok(None)
// Malformed response → Ok(None), log parse error
// Valid response → Ok(Some(parsed_result))
```

### Rate Limiting

Anthropic returns `429` with `retry-after` header. For now: log and degrade. Future: respect retry-after in daemon background calls.

### Cost Guard

The existing `cost-guard.ts` middleware enforced daily budget. Port as:

```rust
// In IntelClient
daily_token_count: AtomicU32,  // Approximate tracking
daily_limit: u32,               // Default: 100K tokens/day

fn check_budget(&self, estimated_tokens: u32) -> bool {
    self.daily_token_count.load(Ordering::Relaxed) + estimated_tokens < self.daily_limit
}
```

---

## Test Strategy

### TDD Order (per phase)

1. **Serialization tests first** — Verify Rust structs serialize to exact JSON the Anthropic API expects
2. **Deserialization tests** — Verify mock Anthropic responses parse correctly
3. **Prompt construction** — Verify prompt output matches TS agent output for same input
4. **Error handling** — Verify graceful degradation for each failure mode
5. **Integration** — E2E with real API key (manual, not CI)

### Test Data

Use captured responses from the TS intel service as golden fixtures:

```rust
#[test]
fn parse_real_anthropic_response() {
    let raw = include_str!("../../tests/fixtures/anthropic_context_synthesis_response.json");
    let response: AnthropicResponse = serde_json::from_str(raw).unwrap();
    // ... verify extraction
}
```

### What NOT to Test

- Don't test the Anthropic API itself (that's their problem)
- Don't test prompt quality (that's evaluation, not testing)
- Don't mock HTTP — test serialization/deserialization with raw JSON

---

## Migration Checklist

- [ ] Phase 1: `anthropic.rs` + `agents/context_synthesis.rs` + tests
- [ ] Phase 1: `IntelClient::from_env()` + `synthesize_context()` direct call
- [ ] Phase 1: E2E verify `tastematter context "nickel"` works without sidecar
- [ ] Phase 2: `agents/chain_naming.rs` + tests
- [ ] Phase 2: `name_chain()` direct call + daemon verify
- [ ] Phase 3: `agents/chain_summary.rs` + tests
- [ ] Phase 5: Archive `intel/` directory
- [ ] Phase 5: Update CLAUDE.md, remove bun references
- [ ] Release: Version bump, release notes

---

## Risk Assessment

| Risk | Likelihood | Mitigation |
|------|-----------|------------|
| Anthropic API format changes | Low | Versioned API (2023-06-01), types are stable |
| Rate limiting blocks daemon | Medium | Cost guard + exponential backoff |
| Prompt drift from TS version | Low | Copy prompts verbatim, test with fixtures |
| Missing edge case in TS | Low | TS tests document edge cases → port as fixtures |

---

## References

- Anthropic Messages API: `https://docs.anthropic.com/en/api/messages`
- Existing Rust client: `core/src/intelligence/client.rs` (477 lines)
- Existing Rust types: `core/src/intelligence/types.rs` (714 lines)
- TS agents to port: `intel/src/agents/*.ts` (1,534 lines total)
- Context restore integration: `core/src/context_restore.rs`

---

**Specification Status:** APPROVED
**Created:** 2026-02-16
**Evidence:** E2E verification of Phase 2 synthesis (2026-02-16), full intel service inventory (3,068 lines TS), architectural analysis showing all 7 agents are single-call pattern
**Next Action:** Phase 1 implementation — `anthropic.rs` + context synthesis agent
