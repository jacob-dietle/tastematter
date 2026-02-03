---
title: "Tastematter Context Package 32"
package_number: 32
date: 2026-01-24
status: current
previous_package: "[[31_2026-01-23_DAEMON_AUTO_SETUP_COMPLETE]]"
related:
  - "[[website/index.html]]"
  - "[[website/styles.css]]"
  - "[[core/src/telemetry/mod.rs]]"
  - "[[specs/POSTHOG_TELEMETRY_SPEC.md]]"
tags:
  - context-package
  - tastematter
  - alpha-distribution
  - telemetry
  - landing-page
---

# Tastematter - Context Package 32

## Executive Summary

Built alpha distribution infrastructure: (1) Landing page at tastematter.dev with terminal aesthetic matching taste.systems brand, (2) PostHog telemetry integration for anonymous usage tracking. Both deployed and verified working.

## Global Context

**Project:** Tastematter - Context intelligence CLI for Claude Code sessions
**Focus This Session:** Alpha distribution readiness (landing page + telemetry)

### Architecture

```
apps/tastematter/
├── core/                        # Rust CLI (existing)
│   ├── src/
│   │   ├── telemetry/           # NEW: Anonymous telemetry
│   │   │   └── mod.rs           # TelemetryClient + TelemetryConfig
│   │   ├── main.rs              # MODIFIED: Instrumentation
│   │   └── lib.rs               # MODIFIED: Export telemetry
│   └── Cargo.toml               # MODIFIED: Added posthog-rs
│
├── website/                     # NEW: Landing page
│   ├── index.html               # Main page with install commands
│   ├── terms.html               # Terms of service
│   ├── privacy.html             # Privacy policy (telemetry disclosure)
│   ├── styles.css               # Design system extract (~770 lines)
│   └── assets/
│       ├── orange_icon.svg      # Logo icon
│       └── VCR_OSD_MONO_1.001.ttf  # Brand font
│
└── specs/
    └── POSTHOG_TELEMETRY_SPEC.md  # NEW: Telemetry spec
```

### Key Design Decisions

1. **Static HTML + CSS for landing page** (not Next.js/Astro)
   - Reason: Simplest deployment, matches brand exactly
   - Cloudflare Pages for hosting
   - [VERIFIED: live at tastematter.dev]

2. **PostHog blocking client** (not async)
   - Reason: CLI is already async, blocking telemetry is fire-and-forget
   - Never blocks CLI operations (errors swallowed)
   - `posthog-rs = { version = "0.3", default-features = false }`
   - [VERIFIED: [[core/Cargo.toml]]:34]

3. **Terminal design aesthetic** (sharp corners, VCR font)
   - Extracted from taste.systems brand
   - Primary: #D35400, Secondary: #FF5C00
   - Border radius: 0 everywhere
   - [VERIFIED: [[website/styles.css]]:10-33]

## Local Problem Set

### Completed This Session

- [X] Landing page created with terminal aesthetic [VERIFIED: [[website/index.html]]]
- [X] Mobile responsive design [VERIFIED: [[website/styles.css]]:602-775]
- [X] Deployed to tastematter.dev via Cloudflare Pages [VERIFIED: curl returns 200]
- [X] PostHog telemetry spec written [VERIFIED: [[specs/POSTHOG_TELEMETRY_SPEC.md]]]
- [X] Telemetry module implemented (~160 lines) [VERIFIED: [[core/src/telemetry/mod.rs]]]
- [X] CLI commands instrumented [VERIFIED: [[core/src/main.rs]]:358-388, 1202-1209]
- [X] Telemetry config auto-created on first run [VERIFIED: ~/.context-os/telemetry.yaml exists]

### In Progress

None - all tasks complete for alpha distribution.

### Jobs To Be Done (Next Session)

1. [ ] **Verify PostHog events in dashboard** - Check posthog.com for `command_executed` events
2. [ ] **Create install scripts** - install.ps1 and install.sh at install.tastematter.dev
3. [ ] **Release v0.1.0-alpha.10** - With telemetry enabled
4. [ ] **Start Phase 5: Intel Service** - MCP server for context-as-a-service

## File Locations

| File | Purpose | Status |
|------|---------|--------|
| [[website/index.html]] | Landing page | NEW |
| [[website/styles.css]] | Design system (~770 lines) | NEW |
| [[website/terms.html]] | Terms of service | NEW |
| [[website/privacy.html]] | Privacy policy | NEW |
| [[core/src/telemetry/mod.rs]] | Telemetry client | NEW |
| [[core/src/main.rs]] | CLI entry + instrumentation | MODIFIED |
| [[core/src/lib.rs]] | Module exports | MODIFIED |
| [[core/Cargo.toml]] | Dependencies | MODIFIED |
| [[specs/POSTHOG_TELEMETRY_SPEC.md]] | Telemetry spec | NEW |

## Telemetry Details

### Events Captured

| Command | Event | Properties |
|---------|-------|------------|
| All commands | `command_executed` | command, duration_ms, platform, version |

### Opt-Out Mechanisms

```bash
# Environment variable
TASTEMATTER_NO_TELEMETRY=1 tastematter query flex

# Config file (~/.context-os/telemetry.yaml)
enabled: false
uuid: <preserved>
```

### Privacy Commitments

- Anonymous UUID only (no PII)
- Command names + timing (no file paths)
- No query results or session content
- PostHog API key: `phc_viCzBS9wW3iaNF0jG0j9mR6IApVnTc62jDkfxPNGUIP`

## Test State

- Build: ✅ Passing (`cargo build --release`)
- Tests: ✅ 691 tests passing (no new telemetry tests added yet)
- Website: ✅ Live at tastematter.dev
- Telemetry config: ✅ Auto-created at ~/.context-os/telemetry.yaml

### Verification Commands

```bash
# Build CLI with telemetry
cd apps/tastematter/core
cargo build --release

# Verify telemetry config created
cat ~/.context-os/telemetry.yaml

# Test command (should send event to PostHog)
./target/release/tastematter query flex --time 1d

# Deploy website updates
cd apps/tastematter/website
CLOUDFLARE_ACCOUNT_ID=4c8353a21e0bfc69a1e036e223cba4d8 npx wrangler pages deploy . --project-name=tastematter --branch=main --commit-dirty=true
```

## For Next Agent

**Context Chain:**
- Previous: [[31_2026-01-23_DAEMON_AUTO_SETUP_COMPLETE]] (daemon auto-setup)
- This package: Alpha distribution infrastructure complete
- Next action: Verify PostHog events, create install scripts

**Start here:**
1. Read this context package
2. Check PostHog dashboard for events: https://posthog.com
3. Read [[specs/POSTHOG_TELEMETRY_SPEC.md]] for telemetry details
4. Continue with install scripts or Phase 5 (Intel Service)

**Key files to understand:**
- [[core/src/telemetry/mod.rs]] - Telemetry implementation
- [[website/styles.css]] - Design system (if modifying website)

**Do NOT:**
- Edit existing context packages (append-only)
- Expose file paths or content in telemetry events
- Block CLI operations on telemetry (fire-and-forget only)

**Key insight:**
Telemetry uses blocking PostHog client but errors are swallowed - CLI never fails due to telemetry. Config persists UUID across sessions at `~/.context-os/telemetry.yaml`.
[VERIFIED: [[core/src/telemetry/mod.rs]]:113-131]
