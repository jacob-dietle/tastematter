---
title: "Tastematter Context Package 67"
package_number: 67
date: 2026-02-16
status: current
previous_package: "[[66_2026-02-16_DOWNLOAD_ALERTS_DEPLOYED_INTEL_PORT_APPROVED]]"
related:
  - "[[specs/canonical/18_INTEL_RUST_PORT_SPEC.md]]"
  - "[[core/src/intelligence/anthropic.rs]]"
  - "[[core/src/intelligence/agents/context_synthesis.rs]]"
  - "[[core/src/intelligence/client.rs]]"
tags:
  - context-package
  - tastematter
  - intel-rust-port
---

# Tastematter - Context Package 67

## Executive Summary

Intel Rust Port Phase 1 COMPLETE. Created `anthropic.rs` (generic `call_anthropic()` function) and `agents/context_synthesis.rs` (first ported agent). Modified `client.rs` to use direct Anthropic API when `ANTHROPIC_API_KEY` is set, with sidecar fallback. Modified `main.rs` to use `IntelClient::from_env()`. All 65 intelligence tests passing, clean compile. [VERIFIED: `cargo check` clean, `cargo test intelligence:: -- --test-threads=2` 65/65 passed]

## Global Context

### Architecture (Post-Port Phase 1)

```
ANTHROPIC_API_KEY set?
  ├── YES → IntelClient::from_env() → Some(client)
  │         ├── synthesize_context() → agents::context_synthesis → call_anthropic() → api.anthropic.com
  │         ├── name_chain() → sidecar localhost:3002 (NOT YET PORTED)
  │         └── summarize_chain() → sidecar localhost:3002 (NOT YET PORTED)
  └── NO → IntelClient::from_env() → None
            └── QueryEngine::new(db) — no intel, fields stay None
```

### Key Design Decisions

- **Hybrid approach**: `IntelClient` has both `api_key: Option<String>` and `base_url: String`. Direct API for ported agents, sidecar for unported. [VERIFIED: [[client.rs]]:25-29]
- **`from_env()` returns `Option<Self>`**: Gated on ANTHROPIC_API_KEY. No key = no client = no wasted connections. [VERIFIED: [[client.rs]]:36-46]
- **Backward compat preserved**: `IntelClient::new()` and `Default` still work for tests and sidecar-only mode. [VERIFIED: [[client.rs]]:48-58, 369-372]
- **Generic `call_anthropic()`**: One function for ALL agents — takes system prompt, user message, tool definition, returns `serde_json::Value`. [VERIFIED: [[anthropic.rs]]:107-155]
- **Prompts ported verbatim from TS**: `build_system_prompt()` and `build_user_message()` match `intel/src/agents/context-synthesis.ts` line-for-line. [VERIFIED: [[agents/context_synthesis.rs]]:22-39, 43-119]

## Local Problem Set

### Completed This Session

1. [X] Created `core/src/intelligence/anthropic.rs` — API types + `call_anthropic()` + 8 tests [VERIFIED: 169 lines]
   - `AnthropicRequest`, `Message`, `ToolDefinition`, `ToolChoice` (serialization)
   - `AnthropicResponse`, `ContentBlock` (tagged enum with `#[serde(other)]`), `Usage` (deserialization)
   - `AnthropicError` enum (Network, ApiError, NoToolUse, ParseError)
   - `call_anthropic()` — POST to api.anthropic.com, extracts tool_use input

2. [X] Created `core/src/intelligence/agents/mod.rs` — Module registry [VERIFIED: 7 lines]

3. [X] Created `core/src/intelligence/agents/context_synthesis.rs` — First ported agent + 8 tests [VERIFIED: 302 lines]
   - `build_system_prompt()` — cluster_count/read_count interpolation
   - `build_user_message()` — formats clusters, reads, context package, evidence sources
   - `tool_definition()` — JSON schema matching TS `CONTEXT_SYNTHESIS_TOOL`
   - `synthesize_context()` — calls `call_anthropic()`, parses tool input → ContextSynthesisResponse

4. [X] Modified `core/src/intelligence/client.rs` — Hybrid direct/sidecar pattern [VERIFIED: 582 lines]
   - Added `api_key: Option<String>` field
   - Added `from_env()` constructor (returns `Option<Self>`)
   - Added `has_api_key()` helper
   - `synthesize_context()` branches: api_key → direct API, else → sidecar
   - Added `synthesize_context_via_sidecar()` private method
   - Kept `new()`, `Default`, `name_chain()`, `summarize_chain()` unchanged
   - Added 3 new tests: `from_env_returns_none_without_api_key`, `from_env_returns_some_with_api_key`, `has_api_key_reports_correctly`

5. [X] Modified `core/src/intelligence/mod.rs` — Registered new modules [VERIFIED: 39 lines]
   - Added `pub mod agents;` and `pub mod anthropic;`
   - Updated module docs

6. [X] Modified `core/src/main.rs` — Switched to `from_env()` pattern [VERIFIED]
   - Line ~628: `IntelClient::from_env()` with match arm → `QueryEngine::new(db).with_intel(client)` or plain `QueryEngine::new(db)`
   - Line ~1275: `IntelClient::from_env().unwrap_or_default()` for `intel` CLI commands

### In Progress

Nothing — Phase 1 is complete and all tests pass.

### Jobs To Be Done (Next Session)

1. [ ] **E2E test with real API key** — Run `ANTHROPIC_API_KEY=... tastematter context "nickel"` WITHOUT the TS sidecar running. Verify synthesis fields populate. Success criteria: `one_liner`, `narrative`, `cluster_names` all non-None.

2. [ ] **Phase 2: Port `chain-naming` agent** — Same pattern: create `agents/chain_naming.rs`, embed prompts from `intel/src/agents/chain-naming.ts`, update `client.rs` to route `name_chain()` through direct API when api_key set.

3. [ ] **Phase 3: Port `chain-summary` agent** — Same pattern for `agents/chain_summary.rs`.

4. [ ] **Phase 4: Port remaining agents** — `commit-analysis`, `session-summary`, `insights`, `gitops-decision`.

5. [ ] **Phase 5: Remove TS sidecar** — Once all agents ported, delete `intel/` directory, update docs.

## File Locations

| File | Purpose | Status |
|------|---------|--------|
| [[core/src/intelligence/anthropic.rs]] | Generic Anthropic API caller + types | NEW (169 lines) |
| [[core/src/intelligence/agents/mod.rs]] | Agent module registry | NEW (7 lines) |
| [[core/src/intelligence/agents/context_synthesis.rs]] | Context synthesis agent (ported from TS) | NEW (302 lines) |
| [[core/src/intelligence/client.rs]] | IntelClient with direct API + sidecar | MODIFIED (582 lines) |
| [[core/src/intelligence/mod.rs]] | Module registration | MODIFIED (39 lines) |
| [[core/src/main.rs]] | CLI entry point | MODIFIED (2 lines changed) |
| [[intel/src/agents/context-synthesis.ts]] | Original TS agent (reference) | UNCHANGED |
| [[specs/canonical/18_INTEL_RUST_PORT_SPEC.md]] | Port specification | REFERENCE |

## Test State

- **Intelligence module tests: 65 passed, 0 failed, 1 ignored** [VERIFIED: cargo test 2026-02-16]
- **New tests added: 11** (8 in anthropic.rs, 8 in context_synthesis.rs, 3 in client.rs — some overlap with existing)
- **All existing tests still pass** (backward compat confirmed)

### Test Commands for Next Agent
```bash
# Verify compilation
cd core && cargo check

# Run intelligence tests only (safe, no memory issues)
cargo test intelligence:: -- --test-threads=2

# E2E test with real API key
ANTHROPIC_API_KEY=sk-ant-... cargo run -- context "nickel" --db ~/.context-os/context_os_events.db

# Full test suite (USE THREAD LIMIT)
cargo test -- --test-threads=2
```

## For Next Agent

**Context Chain:**
- Previous: [[66_2026-02-16_DOWNLOAD_ALERTS_DEPLOYED_INTEL_PORT_APPROVED]] (architecture decision + spec written)
- This package: Phase 1 implementation complete — `anthropic.rs` + `context_synthesis` agent
- Next action: E2E test with real API key, then port `chain-naming` agent

**Start here:**
1. Read this context package
2. Read [[specs/canonical/18_INTEL_RUST_PORT_SPEC.md]] for full port plan
3. Run: `cargo test intelligence:: -- --test-threads=2` to confirm state
4. Run E2E test: `ANTHROPIC_API_KEY=... tastematter context "nickel"` (no sidecar)

**Do NOT:**
- Run `cargo test` without `--test-threads=2` (crashes machine)
- Start the TS intel service for E2E test (that defeats the purpose)
- Edit `types.rs` (existing types unchanged, new types are in `anthropic.rs`)
- Add external crates — everything needed is already in Cargo.toml

**Key insight:**
All 7 TS agents follow the SAME pattern: system prompt + tool_choice → structured JSON. The `call_anthropic()` function in `anthropic.rs` handles this generically. Porting an agent = writing `build_system_prompt()`, `build_user_message()`, `tool_definition()`, and a thin `pub async fn` wrapper. Each agent is ~80-100 lines of Rust. [VERIFIED: [[anthropic.rs]]:107-155, [[agents/context_synthesis.rs]]:161-197]
