//! Git repository status query for GitOps decisions.
//!
//! Queries git status, unpushed commits, and timestamps for intelligent GitOps.

use chrono::{DateTime, Utc};
use std::path::Path;
use std::process::Command;

use crate::intelligence::UncommittedFile;

/// Error querying git status.
#[derive(Debug, Clone)]
pub struct GitStatusError {
    pub message: String,
}

impl std::fmt::Display for GitStatusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GitStatusError: {}", self.message)
    }
}

impl std::error::Error for GitStatusError {}

/// Repository status for GitOps decisions.
#[derive(Debug, Clone)]
pub struct GitRepoStatus {
    /// Uncommitted files with status
    pub uncommitted_files: Vec<UncommittedFile>,
    /// Number of unpushed commits
    pub unpushed_commits: i32,
    /// Current branch name
    pub branch: String,
    /// Timestamp of last commit (None if no commits)
    pub last_commit_timestamp: Option<DateTime<Utc>>,
    /// Timestamp of last push (estimated from remote tracking)
    pub last_push_timestamp: Option<DateTime<Utc>>,
}

/// Query current git repository status.
///
/// Runs:
/// - `git status --porcelain` for uncommitted files
/// - `git rev-list @{u}..HEAD --count` for unpushed commits
/// - `git log -1 --format=%aI` for last commit timestamp
/// - `git branch --show-current` for current branch
pub fn query_repo_status(repo_path: &Path) -> Result<GitRepoStatus, GitStatusError> {
    // Get current branch
    let branch = get_current_branch(repo_path)?;

    // Get uncommitted files
    let uncommitted_files = get_uncommitted_files(repo_path)?;

    // Get unpushed commits count
    let unpushed_commits = get_unpushed_count(repo_path);

    // Get last commit timestamp
    let last_commit_timestamp = get_last_commit_timestamp(repo_path);

    // Get last push timestamp (from remote tracking branch)
    let last_push_timestamp = get_last_push_timestamp(repo_path);

    Ok(GitRepoStatus {
        uncommitted_files,
        unpushed_commits,
        branch,
        last_commit_timestamp,
        last_push_timestamp,
    })
}

/// Get current branch name.
fn get_current_branch(repo_path: &Path) -> Result<String, GitStatusError> {
    let output = Command::new("git")
        .args(["branch", "--show-current"])
        .current_dir(repo_path)
        .output()
        .map_err(|e| GitStatusError {
            message: format!("git branch failed: {}", e),
        })?;

    if !output.status.success() {
        return Err(GitStatusError {
            message: "git branch --show-current failed".to_string(),
        });
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Get uncommitted files with status.
fn get_uncommitted_files(repo_path: &Path) -> Result<Vec<UncommittedFile>, GitStatusError> {
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(repo_path)
        .output()
        .map_err(|e| GitStatusError {
            message: format!("git status failed: {}", e),
        })?;

    if !output.status.success() {
        return Err(GitStatusError {
            message: "git status --porcelain failed".to_string(),
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut files = Vec::new();

    for line in stdout.lines() {
        if line.len() < 3 {
            continue;
        }

        // Format: XY filename
        // X = index status, Y = worktree status
        let status_chars = &line[..2];
        let path = line[3..].trim().to_string();

        // Determine status from XY codes
        let status = match status_chars.chars().nth(0) {
            Some('M') | Some('m') => "modified",
            Some('A') | Some('a') => "added",
            Some('D') | Some('d') => "deleted",
            Some('R') | Some('r') => "renamed",
            Some('?') => "added", // Untracked files
            _ => match status_chars.chars().nth(1) {
                Some('M') | Some('m') => "modified",
                Some('D') | Some('d') => "deleted",
                _ => "modified", // Default
            },
        };

        files.push(UncommittedFile {
            path,
            status: status.to_string(),
            lines_changed: None, // Could add git diff --numstat if needed
        });
    }

    Ok(files)
}

/// Get count of unpushed commits.
fn get_unpushed_count(repo_path: &Path) -> i32 {
    // Try to get count of commits ahead of upstream
    let output = Command::new("git")
        .args(["rev-list", "@{u}..HEAD", "--count"])
        .current_dir(repo_path)
        .output();

    match output {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout)
            .trim()
            .parse()
            .unwrap_or(0),
        _ => {
            // No upstream tracking branch or error - try alternative
            // Count commits not on any remote
            let output = Command::new("git")
                .args(["log", "--oneline", "--not", "--remotes"])
                .current_dir(repo_path)
                .output();

            match output {
                Ok(out) if out.status.success() => {
                    String::from_utf8_lossy(&out.stdout).lines().count() as i32
                }
                _ => 0,
            }
        }
    }
}

/// Get timestamp of last commit.
fn get_last_commit_timestamp(repo_path: &Path) -> Option<DateTime<Utc>> {
    let output = Command::new("git")
        .args(["log", "-1", "--format=%aI"])
        .current_dir(repo_path)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let timestamp_str = String::from_utf8_lossy(&output.stdout);
    chrono::DateTime::parse_from_rfc3339(timestamp_str.trim())
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

/// Get timestamp of when we last pushed (estimated from remote tracking branch).
fn get_last_push_timestamp(repo_path: &Path) -> Option<DateTime<Utc>> {
    // Get the timestamp of the commit at origin/HEAD or origin/<current-branch>
    let output = Command::new("git")
        .args(["log", "-1", "--format=%aI", "@{u}"])
        .current_dir(repo_path)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let timestamp_str = String::from_utf8_lossy(&output.stdout);
    chrono::DateTime::parse_from_rfc3339(timestamp_str.trim())
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_repo_status_works_in_current_repo() {
        // Test against the current repository
        let result = query_repo_status(Path::new("."));

        assert!(
            result.is_ok(),
            "Should work in git repo: {:?}",
            result.err()
        );

        let status = result.unwrap();
        assert!(!status.branch.is_empty(), "Should have a branch name");

        println!("Branch: {}", status.branch);
        println!("Uncommitted files: {}", status.uncommitted_files.len());
        println!("Unpushed commits: {}", status.unpushed_commits);
    }

    #[test]
    fn test_get_current_branch() {
        let result = get_current_branch(Path::new("."));
        assert!(result.is_ok());
        let branch = result.unwrap();
        assert!(!branch.is_empty(), "Should return branch name");
    }

    #[test]
    fn test_get_uncommitted_files() {
        let result = get_uncommitted_files(Path::new("."));
        assert!(result.is_ok());
        // We may or may not have uncommitted files, that's ok
    }

    #[test]
    fn test_get_last_commit_timestamp() {
        let ts = get_last_commit_timestamp(Path::new("."));
        // There should be commits in this repo
        assert!(ts.is_some(), "Should have commit history");
    }
}
