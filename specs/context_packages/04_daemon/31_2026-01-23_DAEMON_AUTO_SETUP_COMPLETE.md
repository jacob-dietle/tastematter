---
title: "Tastematter Context Package 31"
package_number: 31
date: 2026-01-23
status: current
previous_package: "[[30_2026-01-20_CLI_DISTRIBUTION_ARCHITECTURE]]"
related:
  - "[[core/src/daemon/platform/mod.rs]]"
  - "[[core/src/daemon/platform/windows.rs]]"
  - "[[core/src/main.rs]]"
  - "[[scripts/install/install.ps1]]"
tags:
  - context-package
  - tastematter
  - daemon-auto-setup
---

# Tastematter - Context Package 31

## Executive Summary

Implemented cross-platform daemon auto-setup so the one-liner install "just works" without user configuration. Daemon now auto-registers to run on login (Windows Startup folder, macOS launchd, Linux systemd). Release v0.1.0-alpha.9 deployed and tested end-to-end.

## Global Context

**Project:** Tastematter - Context intelligence CLI for Claude Code sessions
**Focus This Session:** Daemon auto-setup feature (no user configuration required)

### Architecture

```
core/src/daemon/
├── mod.rs           # Existing: exports config, state, sync
├── config.rs        # Existing: DaemonConfig
├── state.rs         # Existing: DaemonState
├── sync.rs          # Existing: run_sync orchestrator
└── platform/        # NEW: Cross-platform autostart
    ├── mod.rs       # DaemonPlatform trait + types
    ├── windows.rs   # Startup folder VBS script
    ├── macos.rs     # launchd plist + launchctl
    └── linux.rs     # systemd user service
```

### Key Design Decisions

1. **Windows: Startup folder VBS script** (not Task Scheduler)
   - Reason: Task Scheduler `onlogon` trigger requires admin
   - VBS script runs hidden (no console window)
   - No admin required [VERIFIED: tested on Windows]

2. **No third-party dependencies**
   - Uses native OS tools only (schtasks fallback, launchctl, systemctl)
   - No NSSM, no external binaries

3. **User-level install only**
   - No sudo/admin required on any platform
   - Files in user directories (~/.local/bin, ~/Library/LaunchAgents, ~/.config/systemd/user)

## Local Problem Set

### Completed This Session

- [X] Phase 1: Core types + DaemonPlatform trait [VERIFIED: [[core/src/daemon/platform/mod.rs]]:1-180]
- [X] Phase 2: Windows implementation (Startup folder) [VERIFIED: [[core/src/daemon/platform/windows.rs]]:1-280]
- [X] Phase 3: macOS implementation (launchd) [VERIFIED: [[core/src/daemon/platform/macos.rs]]:1-150]
- [X] Phase 4: Linux implementation (systemd) [VERIFIED: [[core/src/daemon/platform/linux.rs]]:1-150]
- [X] Phase 5: CLI commands (install/uninstall/status) [VERIFIED: [[core/src/main.rs]]:1117-1165]
- [X] Phase 6: Install script updates [VERIFIED: [[scripts/install/install.ps1]]:77-97]
- [X] All 10 platform tests passing [VERIFIED: cargo test daemon::platform 2026-01-23]
- [X] Release v0.1.0-alpha.9 deployed [VERIFIED: gh run view shows success]
- [X] End-to-end install tested [VERIFIED: irm install.tastematter.dev/install.ps1 | iex]

### In Progress

None - feature complete.

### Jobs To Be Done (Future)

1. [ ] Test macOS install on actual Mac - Success criteria: `daemon install` creates plist, `daemon status` shows registered
2. [ ] Test Linux install in WSL - Success criteria: systemd user service created
3. [ ] Add `tastematter daemon start-now` to start immediately after install
4. [ ] Add `tastematter update` self-update command

## File Locations

| File | Purpose | Status |
|------|---------|--------|
| [[core/src/daemon/platform/mod.rs]] | Core types, DaemonPlatform trait | Added |
| [[core/src/daemon/platform/windows.rs]] | Windows Startup folder implementation | Added |
| [[core/src/daemon/platform/macos.rs]] | macOS launchd implementation | Added |
| [[core/src/daemon/platform/linux.rs]] | Linux systemd implementation | Added |
| [[core/src/daemon/mod.rs]] | Added `pub mod platform` export | Modified |
| [[core/src/main.rs]] | Added Install/Uninstall CLI commands | Modified |
| [[scripts/install/install.ps1]] | Added daemon registration after download | Modified |
| [[scripts/install/install.sh]] | Added daemon registration after download | Modified |

## Test State

- Platform tests: 10 passing, 2 ignored (integration tests)
- Command: `cargo test --lib daemon::platform --release`
- Last run: 2026-01-23
- Evidence: [VERIFIED: test output shows ok. 10 passed; 0 failed; 2 ignored]

### Test Commands for Next Agent

```bash
# Verify platform tests
cd core && cargo test --lib daemon::platform --release

# Test CLI commands
tastematter daemon status
tastematter daemon install
tastematter daemon uninstall

# Full build
cd core && cargo build --release
```

## CLI Interface

New commands added:

```bash
tastematter daemon install [--interval N]  # Register to run on login (default: 30 min)
tastematter daemon uninstall               # Remove registration
tastematter daemon status                  # Shows platform + sync state
```

Example status output:
```
=== Platform Status ===
Platform: Windows (Startup folder)
Registered: Yes
Running: No (will start on next login)

=== Sync State ===
Last git sync: 2026-01-23 10:30:15 UTC
Sessions parsed: 1,079
```

## Release History

| Version | Date | Changes |
|---------|------|---------|
| v0.1.0-alpha.8 | 2026-01-23 | Added Linux build target |
| v0.1.0-alpha.9 | 2026-01-23 | Daemon auto-setup (this release) |

## For Next Agent

**Context Chain:**
- Previous: [[30_2026-01-20_CLI_DISTRIBUTION_ARCHITECTURE]] (CLI distribution planning)
- This package: Daemon auto-setup complete, v0.1.0-alpha.9
- Next action: Test on macOS/Linux, or continue with other features

**Start here:**
1. Read this context package
2. Run `tastematter daemon status` to verify current state
3. If needed, test on macOS: `curl -fsSL https://install.tastematter.dev/install.sh | bash`

**Do NOT:**
- Use Task Scheduler on Windows (requires admin) - use Startup folder instead
- Expect Python CLI to exist - it was removed, Rust binary is canonical

**Key insight:**
Windows Startup folder VBS script is the reliable non-admin approach. Task Scheduler's `onlogon` trigger requires elevated privileges even with `/ru` and `/rl` flags.
[VERIFIED: tested 2026-01-23, schtasks /create returns "Access denied" without admin]

## Implementation Summary

### Platform Trait

```rust
pub trait DaemonPlatform {
    fn install(&self, config: &InstallConfig) -> Result<InstallResult, PlatformError>;
    fn uninstall(&self) -> Result<(), PlatformError>;
    fn is_installed(&self) -> Result<bool, PlatformError>;
    fn status(&self) -> Result<PlatformStatus, PlatformError>;
}
```

### Windows: Startup Folder

- Writes VBS script to `%APPDATA%\Microsoft\Windows\Start Menu\Programs\Startup\tastematter-daemon.vbs`
- Script runs `tastematter daemon start --interval 30` hidden (no console window)
- Checks `tasklist` for running process in status

### macOS: launchd

- Writes plist to `~/Library/LaunchAgents/dev.tastematter.daemon.plist`
- Uses `launchctl load/unload` for install/uninstall
- `RunAtLoad: true` starts on login

### Linux: systemd

- Writes unit file to `~/.config/systemd/user/tastematter.service`
- Uses `systemctl --user enable/start/stop` commands
- `Restart=on-failure` for resilience
