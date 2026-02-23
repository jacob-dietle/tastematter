---
title: "Tastematter Context Package 69"
package_number: 69
date: 2026-02-17
status: current
previous_package: "[[68_2026-02-16_INTEL_PORT_E2E_SETUP_CI_FIXES_NEEDED]]"
related:
  - "[[specs/POSTHOG_TELEMETRY_SPEC.md]]"
  - "[[core/src/telemetry/mod.rs]]"
  - "[[core/src/intelligence/client.rs]]"
tags:
  - context-package
  - tastematter
  - telemetry
---

# Tastematter - Context Package 69

## Executive Summary

Fixed PostHog telemetry that was **broken since day 1** — the `posthog-rs` crate panics inside tokio runtime, so the guard made the client always `None`. Replaced with raw async `reqwest::Client` POST matching the existing `IntelClient` pattern. Also added CI environment filtering to prevent smoke test noise. Released as v0.1.0-alpha.25 + alpha.26.

## What Was Done

### 1. Epistemic Grounding (before coding)

Ran epistemic-context-grounding skill to verify assumptions before implementing:

- [VERIFIED: Cargo.toml:35] `reqwest` already in deps (async, v0.12)
- [VERIFIED: Grep] Only `telemetry/mod.rs` + `Cargo.toml` reference `posthog_rs`
- [VERIFIED: client.rs:9-29] `IntelClient` is the exact async reqwest pattern to follow
- [VERIFIED: main.rs:623,1426] Exactly 2 call sites need `.await`
- [VERIFIED: specs/POSTHOG_TELEMETRY_SPEC.md] Original spec documented posthog-rs approach

Key decision: Evaluated "fix posthog-rs with spawn_blocking" vs "replace with raw reqwest POST". Both require identical main.rs changes. Raw reqwest is simpler (no Arc wrapper, no spawn_blocking per event), removes a dependency, and matches existing IntelClient pattern.

### 2. Implementation (4 files changed)

**`core/Cargo.toml`** — Removed `posthog-rs` dependency (1 line)
[VERIFIED: git diff shows -1 line]

**`core/src/telemetry/mod.rs`** — Core replacement:
- `client: Option<posthog_rs::Client>` → `client: reqwest::Client` (always created, gated by `config.enabled`)
- Removed tokio runtime guard (lines 82-93 of old code)
- `capture()` now async — POSTs JSON to `https://us.i.posthog.com/capture/`
- `is_enabled()` checks `config.enabled` (was `config.enabled && client.is_some()`)
- All typed helpers (`capture_command`, etc.) now async
[VERIFIED: 105 insertions, 411 deletions (mostly Cargo.lock transitive deps)]

**`core/src/main.rs`** — Added `.await` to both call sites:
- Line 623: daemon commands telemetry
- Line 1426: end-of-main telemetry

### 3. CI Noise Filter (alpha.26)

Added `|| std::env::var("CI").is_ok()` to init check. GitHub Actions sets `CI=true` automatically on all runners. Prevents smoke test / E2E runs from flooding PostHog.

### 4. Verification

- **Build:** `cargo build --release` clean
- **Tests:** 9/9 telemetry tests passing
- **Live test:** `TASTEMATTER_TELEMETRY_DEBUG=1 tastematter query flex --time 1d` → `[telemetry] ✓ Event sent successfully`
- **PostHog MCP query:** 95 events visible (1 local + 94 CI, confirming CI filter was needed)
- **CI/CD:** Both alpha.25 and alpha.26 passed full pipeline (CI + staging builds + smoke tests on 3 platforms + E2E + release)

## Releases

| Tag | Commit | Change |
|-----|--------|--------|
| v0.1.0-alpha.25 | `09770ae` + `2594397` | Telemetry fix + cargo fmt |
| v0.1.0-alpha.26 | `7783161` | CI environment filter |

## PostHog API Pattern (for reference)

```
POST https://us.i.posthog.com/capture/
Content-Type: application/json

{
    "api_key": "phc_viCzBS9...",
    "event": "command_executed",
    "distinct_id": "uuid-from-telemetry.yaml",
    "properties": {
        "$lib": "tastematter-cli",
        "platform": "windows",
        "version": "0.1.0",
        "command": "query_flex",
        "duration_ms": 53,
        ...
    }
}
```

## File Locations

| File | Purpose | Status |
|------|---------|--------|
| [[core/Cargo.toml]] | Dependencies | Modified (removed posthog-rs) |
| [[core/src/telemetry/mod.rs]] | Telemetry client | Rewritten (async reqwest) |
| [[core/src/telemetry/events.rs]] | Event types | Unchanged |
| [[core/src/main.rs]] | CLI entry point | 2 lines changed (.await) |
| [[core/src/intelligence/client.rs]] | IntelClient | Reference pattern (unchanged) |
| [[specs/POSTHOG_TELEMETRY_SPEC.md]] | Original spec | Reference (outdated re: posthog-rs) |

## Test State

- Telemetry tests: 9 passing (2 in mod.rs, 7 in events.rs)
- Full suite: 413+ tests, `cargo test -- --test-threads=2`
- CI: All green on alpha.25 and alpha.26

## For Next Agent

**Context Chain:**
- Previous: [[68_2026-02-16_INTEL_PORT_E2E_SETUP_CI_FIXES_NEEDED]]
- This package: Telemetry fixed and released
- Next action: Monitor PostHog for real user events as alpha users update

**Note:** The `specs/POSTHOG_TELEMETRY_SPEC.md` still references `posthog-rs` — it's now outdated but serves as historical context for the original design intent. The actual implementation in `telemetry/mod.rs` is the source of truth.

**Do NOT:**
- Re-add posthog-rs — it fundamentally conflicts with tokio runtime
- Make telemetry blocking — always fire-and-forget async
- Remove the `CI=true` check — prevents 90+ noise events per release
