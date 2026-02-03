---
title: "Tastematter Context Package 33"
package_number: 33
date: 2026-01-24
status: current
previous_package: "[[32_2026-01-24_ALPHA_DISTRIBUTION_INFRA]]"
related:
  - "[[core/src/telemetry/mod.rs]]"
  - "[[core/src/telemetry/events.rs]]"
  - "[[specs/POSTHOG_TELEMETRY_SPEC.md]]"
tags:
  - context-package
  - tastematter
  - telemetry
  - posthog
---

# Tastematter - Context Package 33

## Executive Summary

Debugged PostHog telemetry (events were sending successfully), designed privacy-first telemetry schema following Claude Code/Vercel/HashiCorp patterns, and began implementing typed event structs. Created `events.rs` with 4 event types and updated `mod.rs` with helper methods.

## Global Context

**Project:** Tastematter - Context intelligence CLI for Claude Code sessions
**Focus This Session:** PostHog telemetry debugging and schema design

### Expert Framework Applied

Researched and applied telemetry patterns from:
- [Claude Code](https://code.claude.com/docs/en/data-usage) - "No code or file paths"
- [Vercel CLI](https://vercel.com/docs/cli/about-telemetry) - "No sensitive data"
- [HashiCorp Checkpoint](https://checkpoint.hashicorp.com/) - Machine UUID, minimal data

### Privacy Principles (Non-Negotiable)

| NEVER Collect | ALWAYS Collect | Collect With Care |
|---------------|----------------|-------------------|
| File paths | Machine UUID | Result counts |
| Query content | Platform (OS) | Time range buckets |
| Error messages | Version | Error codes only |
| User identity | Command name | |
| Env variables | Duration (ms) | |

## Local Problem Set

### Completed This Session

- [X] Debugged PostHog - added `TASTEMATTER_TELEMETRY_DEBUG=1` env var [VERIFIED: [[core/src/telemetry/mod.rs]]:115-154]
- [X] Verified events ARE sending to PostHog (4 events captured) [VERIFIED: PostHog MCP query]
- [X] Created PostHog dashboard "Tastematter Alpha Telemetry" [VERIFIED: PostHog insight IDs 6391198, 6391205]
- [X] Researched expert telemetry patterns (Claude Code, Vercel, HashiCorp, Apple)
- [X] Designed telemetry schema spec [VERIFIED: [[plans/synchronous-coalescing-harbor.md]]]
- [X] Created `events.rs` with typed event structs [VERIFIED: [[core/src/telemetry/events.rs]]:1-250]
- [X] Updated `mod.rs` with event exports and helper methods [VERIFIED: [[core/src/telemetry/mod.rs]]:13-18, 172-192]
- [X] Updated `lib.rs` to export new types [VERIFIED: [[core/src/lib.rs]]:28-31]

### In Progress

- [ ] Instrument commands in `main.rs` with typed helpers
  - Created helper methods: `capture_command()`, `capture_sync()`, `capture_error()`, `capture_feature()`
  - Need to replace generic `capture()` calls with typed helpers
  - Add `result_count` and `time_range_bucket` to query commands

### Jobs To Be Done (Next Session)

1. [ ] **Instrument main.rs** - Replace generic `capture()` with typed helpers
   - Success criteria: All commands use `capture_command()` with proper properties
2. [ ] **Build and test** - Verify events show enriched properties in PostHog
   - Success criteria: Debug output shows `time_range_bucket`, `result_count`
3. [ ] **Create PostHog insights** - Duration percentiles, error rates
   - Success criteria: Dashboard shows command performance over time

## File Locations

| File | Purpose | Status |
|------|---------|--------|
| [[core/src/telemetry/events.rs]] | Typed event structs | Created (~250 lines) |
| [[core/src/telemetry/mod.rs]] | Client + helper methods | Modified |
| [[core/src/lib.rs]] | Public exports | Modified |
| [[core/src/main.rs]] | CLI commands | Needs instrumentation |
| [[specs/POSTHOG_TELEMETRY_SPEC.md]] | Original spec | Reference |
| [[plans/synchronous-coalescing-harbor.md]] | Telemetry design spec | Created |

## Event Schema

### 1. CommandExecutedEvent
```rust
pub struct CommandExecutedEvent {
    pub command: String,           // "query_flex", "daemon_status"
    pub duration_ms: u64,
    pub success: bool,
    pub result_count: Option<u32>,
    pub time_range_bucket: Option<TimeRangeBucket>, // "1d", "7d", "30d"
}
```

### 2. SyncCompletedEvent
```rust
pub struct SyncCompletedEvent {
    pub sessions_parsed: u32,
    pub chains_built: u32,
    pub duration_ms: u64,
}
```

### 3. ErrorOccurredEvent
```rust
pub struct ErrorOccurredEvent {
    pub error_code: ErrorCode,    // DB_CONNECTION, PARSE_FAILED, etc.
    pub command: String,
    // NEVER: error message, stack trace
}
```

### 4. FeatureUsedEvent
```rust
pub struct FeatureUsedEvent {
    pub feature: String,          // "daemon_autostart", "chain_query"
    pub first_use: bool,
}
```

## Test State

- Events module: 8 unit tests defined in `events.rs`
- Need to run: `cargo test --lib telemetry` to verify
- Build status: Not verified (implementation interrupted)

### Test Commands for Next Agent

```bash
# Build and run tests
cd apps/tastematter/core
cargo build --release
cargo test --lib telemetry

# Test with debug mode
TASTEMATTER_TELEMETRY_DEBUG=1 ./target/release/tastematter query flex --time 7d

# Expected enriched output:
# [telemetry] command_executed: {command: "query_flex", duration_ms: 234, result_count: 47, time_range_bucket: "7d", success: true}
```

## PostHog Dashboard Created

**Dashboard:** [Tastematter Alpha Telemetry](https://us.posthog.com/project/297687/dashboard/1126201)

**Insights:**
1. CLI Usage (line graph) - ID: 6391198
2. Commands by Type (pie chart) - ID: 6391205

**Current Data:** 4 events on 2026-01-24

## For Next Agent

**Context Chain:**
- Previous: [[32_2026-01-24_ALPHA_DISTRIBUTION_INFRA]] (landing page + basic telemetry)
- This package: Telemetry schema design + typed events
- Next action: Instrument main.rs with typed helpers

**Start here:**
1. Read this context package
2. Read [[core/src/telemetry/events.rs]] for event types
3. Run `cargo build --release` to verify build
4. Update [[core/src/main.rs]] to use typed helpers

**Do NOT:**
- Collect file paths in ANY event
- Include error messages (use error codes only)
- Store query content or session data

**Key insight:**
Privacy-first telemetry follows Claude Code pattern: "This logging does not include any code or file paths." All events use machine UUID (not user identity), command names (not arguments), and aggregate counts (not actual data).
[VERIFIED: Claude Code docs at code.claude.com/docs/en/data-usage]
