//! Build script to embed git version information at compile time.
//!
//! This enables distinguishing between:
//! - Local dev builds: `0.1.0-dev+abc1234`
//! - Tagged releases: `v0.1.0-alpha.11`

use std::process::Command;

fn main() {
    // Re-run if git HEAD changes (new commits, checkouts, etc.)
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/");

    // Get git describe output (tag if on tag, otherwise tag-commits-hash)
    let git_describe = Command::new("git")
        .args(["describe", "--tags", "--always", "--dirty"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout).ok()
            } else {
                None
            }
        })
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // Get short commit hash
    let git_hash = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout).ok()
            } else {
                None
            }
        })
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // Determine if we're on an exact tag
    let on_tag = Command::new("git")
        .args(["describe", "--tags", "--exact-match"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    // Build version string
    let version = if on_tag {
        // On a tag: use the tag name directly (e.g., "v0.1.0-alpha.11")
        git_describe
    } else {
        // Not on tag: show dev version with commit (e.g., "0.1.0-dev+abc1234")
        let cargo_version =
            std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.1.0".to_string());
        format!("{}-dev+{}", cargo_version, git_hash)
    };

    println!("cargo:rustc-env=TASTEMATTER_VERSION={}", version);
    println!("cargo:rustc-env=TASTEMATTER_GIT_HASH={}", git_hash);
}
