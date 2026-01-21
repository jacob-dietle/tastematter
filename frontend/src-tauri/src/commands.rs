use serde::Serialize;
use std::process::Command;
use tauri::{command, State};
use log::info;

use crate::logging::LogEvent;
use crate::AppState;

// Import types from core library (Phase 2: direct integration)
use context_os_core::{
    QueryFlexInput, QueryTimelineInput, QuerySessionsInput, QueryChainsInput,
    QueryResult, TimelineData, SessionQueryResult as CoreSessionQueryResult,
    ChainQueryResult as CoreChainQueryResult, CoreError,
};

// Log event command - receives log events from frontend
#[command]
pub fn log_event(event: LogEvent, state: State<AppState>) -> Result<(), String> {
    state.log_service.log(event);
    Ok(())
}

// Error type for IPC
#[derive(Serialize)]
pub struct CommandError {
    pub code: String,
    pub message: String,
    pub details: Option<String>,
}

impl From<std::io::Error> for CommandError {
    fn from(err: std::io::Error) -> Self {
        CommandError {
            code: "IO_ERROR".to_string(),
            message: err.to_string(),
            details: None,
        }
    }
}

impl From<CoreError> for CommandError {
    fn from(err: CoreError) -> Self {
        CommandError {
            code: "CORE_ERROR".to_string(),
            message: format!("{:?}", err),
            details: None,
        }
    }
}

// =============================================================================
// Query Commands - Using context_os_core directly (Phase 2)
// =============================================================================

#[command]
pub async fn query_flex(
    state: State<'_, AppState>,
    files: Option<String>,
    time: Option<String>,
    chain: Option<String>,
    session: Option<String>,
    agg: Vec<String>,
    limit: Option<u32>,
    sort: Option<String>,
) -> Result<QueryResult, CommandError> {
    info!("[query_flex] time={:?}, chain={:?}, limit={:?}", time, chain, limit);

    let engine = state.get_query_engine().await?;

    let input = QueryFlexInput {
        files,
        time,
        chain,
        session,
        agg,
        limit,
        sort,
    };

    let result = engine.query_flex(input).await?;

    info!("[query_flex] success: {} results", result.result_count);
    Ok(result)
}

#[command]
pub async fn query_timeline(
    state: State<'_, AppState>,
    time: String,
    files: Option<String>,
    chain: Option<String>,
    limit: Option<u32>,
) -> Result<TimelineData, CommandError> {
    info!("[query_timeline] time={}, limit={:?}", time, limit);

    let engine = state.get_query_engine().await?;

    let input = QueryTimelineInput {
        time: time.clone(),
        files,
        chain,
        limit,
    };

    let result = engine.query_timeline(input).await?;

    info!("[query_timeline] success: {} files, {} buckets", result.files.len(), result.buckets.len());
    Ok(result)
}

#[command]
pub async fn query_sessions(
    state: State<'_, AppState>,
    time: String,
    chain: Option<String>,
    limit: Option<u32>,
) -> Result<CoreSessionQueryResult, CommandError> {
    info!("[query_sessions] time={}, chain={:?}, limit={:?}", time, chain, limit);

    let engine = state.get_query_engine().await?;

    let input = QuerySessionsInput {
        time: time.clone(),
        chain,
        limit,
    };

    let result = engine.query_sessions(input).await?;

    info!("[query_sessions] success: {} sessions", result.sessions.len());
    Ok(result)
}

#[command]
pub async fn query_chains(
    state: State<'_, AppState>,
    limit: Option<u32>,
) -> Result<CoreChainQueryResult, CommandError> {
    info!("[query_chains] limit={:?}", limit);

    let engine = state.get_query_engine().await?;

    let input = QueryChainsInput {
        limit,
    };

    let result = engine.query_chains(input).await?;

    info!("[query_chains] success: {} chains", result.chains.len());
    Ok(result)
}

// =============================================================================
// Git Commands - Using subprocess (legitimate use of Command::new)
// =============================================================================

#[derive(Serialize)]
pub struct GitStatus {
    pub branch: String,
    pub ahead: u32,
    pub behind: u32,
    pub staged: Vec<String>,
    pub modified: Vec<String>,
    pub untracked: Vec<String>,
    pub has_conflicts: bool,
}

#[derive(Serialize)]
pub struct GitOpResult {
    pub success: bool,
    pub message: String,
    pub error: Option<String>,
    pub files_affected: Option<u32>,
}

// Git commands

#[command]
pub async fn git_status() -> Result<GitStatus, CommandError> {
    let output = Command::new("git")
        .args(["status", "-sb", "--porcelain"])
        .output()
        .map_err(|e| CommandError {
            code: "GIT_NOT_FOUND".to_string(),
            message: "Git not found. Please ensure git is installed and in PATH.".to_string(),
            details: Some(e.to_string()),
        })?;

    let status_str = String::from_utf8_lossy(&output.stdout);

    let (branch, ahead, behind) = parse_status_sb_header(&status_str);
    let file_lines: String = status_str.lines().skip(1).collect::<Vec<_>>().join("\n");
    let (staged, modified, untracked, has_conflicts) = parse_porcelain_status(&file_lines);

    Ok(GitStatus {
        branch,
        ahead,
        behind,
        staged,
        modified,
        untracked,
        has_conflicts,
    })
}

fn parse_porcelain_status(status: &str) -> (Vec<String>, Vec<String>, Vec<String>, bool) {
    let mut staged = Vec::new();
    let mut modified = Vec::new();
    let mut untracked = Vec::new();
    let mut has_conflicts = false;

    for line in status.lines() {
        if line.len() < 3 {
            continue;
        }

        let index_status = line.chars().nth(0).unwrap_or(' ');
        let worktree_status = line.chars().nth(1).unwrap_or(' ');
        let file_path = line[3..].to_string();

        if index_status == 'U' || worktree_status == 'U' {
            has_conflicts = true;
        }

        if index_status != ' ' && index_status != '?' {
            staged.push(file_path.clone());
        }

        if worktree_status == 'M' || worktree_status == 'D' {
            modified.push(file_path.clone());
        }

        if index_status == '?' {
            untracked.push(file_path);
        }
    }

    (staged, modified, untracked, has_conflicts)
}

#[command]
pub async fn git_pull() -> Result<GitOpResult, CommandError> {
    let output = Command::new("git")
        .args(["pull", "--ff-only"])
        .output()
        .map_err(|e| CommandError {
            code: "GIT_ERROR".to_string(),
            message: "Failed to execute git pull".to_string(),
            details: Some(e.to_string()),
        })?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        let files_affected = parse_pull_files_affected(&stdout);

        Ok(GitOpResult {
            success: true,
            message: if stdout.contains("Already up to date") {
                "Already up to date".to_string()
            } else {
                "Pulled successfully".to_string()
            },
            error: None,
            files_affected,
        })
    } else {
        Ok(GitOpResult {
            success: false,
            message: "Pull failed".to_string(),
            error: Some(stderr),
            files_affected: None,
        })
    }
}

fn parse_pull_files_affected(output: &str) -> Option<u32> {
    for line in output.lines() {
        if line.contains("files changed") || line.contains("file changed") {
            if let Some(num) = line.split_whitespace().next() {
                return num.parse().ok();
            }
        }
    }
    None
}

#[command]
pub async fn git_push() -> Result<GitOpResult, CommandError> {
    let output = Command::new("git")
        .args(["push"])
        .output()
        .map_err(|e| CommandError {
            code: "GIT_ERROR".to_string(),
            message: "Failed to execute git push".to_string(),
            details: Some(e.to_string()),
        })?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    let combined = format!("{}{}", stdout, stderr);

    if output.status.success() {
        Ok(GitOpResult {
            success: true,
            message: if combined.contains("Everything up-to-date") {
                "Everything up-to-date".to_string()
            } else {
                "Pushed successfully".to_string()
            },
            error: None,
            files_affected: None,
        })
    } else {
        Ok(GitOpResult {
            success: false,
            message: "Push failed".to_string(),
            error: Some(stderr),
            files_affected: None,
        })
    }
}

// Git helper functions

fn parse_status_sb_header(output: &str) -> (String, u32, u32) {
    let first_line = output.lines().next().unwrap_or("");

    let branch = first_line
        .strip_prefix("## ")
        .and_then(|s| s.split("...").next())
        .map(|s| s.split_whitespace().next().unwrap_or(s))
        .unwrap_or("main")
        .to_string();

    let (ahead, behind) = if let Some(bracket_start) = first_line.find('[') {
        let bracket_content = &first_line[bracket_start..];
        let ahead = extract_count(bracket_content, "ahead ");
        let behind = extract_count(bracket_content, "behind ");
        (ahead, behind)
    } else {
        (0, 0)
    };

    (branch, ahead, behind)
}

fn extract_count(s: &str, prefix: &str) -> u32 {
    s.find(prefix)
        .and_then(|i| {
            let after = &s[i + prefix.len()..];
            after.split(|c: char| !c.is_ascii_digit())
                .next()
                .and_then(|n| n.parse().ok())
        })
        .unwrap_or(0)
}

// Tests for git status parsing
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_status_sb_with_upstream() {
        let output = "## main...origin/main [ahead 2, behind 3]\n M file1.txt\n?? file2.txt";
        let (branch, ahead, behind) = parse_status_sb_header(output);
        assert_eq!(branch, "main");
        assert_eq!(ahead, 2);
        assert_eq!(behind, 3);
    }

    #[test]
    fn test_parse_status_sb_no_upstream() {
        let output = "## main\n M file.txt";
        let (branch, ahead, behind) = parse_status_sb_header(output);
        assert_eq!(branch, "main");
        assert_eq!(ahead, 0);
        assert_eq!(behind, 0);
    }

    #[test]
    fn test_parse_status_sb_ahead_only() {
        let output = "## feature...origin/feature [ahead 5]";
        let (branch, ahead, behind) = parse_status_sb_header(output);
        assert_eq!(branch, "feature");
        assert_eq!(ahead, 5);
        assert_eq!(behind, 0);
    }

    #[test]
    fn test_parse_status_sb_behind_only() {
        let output = "## main...origin/main [behind 1]";
        let (branch, ahead, behind) = parse_status_sb_header(output);
        assert_eq!(branch, "main");
        assert_eq!(ahead, 0);
        assert_eq!(behind, 1);
    }

    #[test]
    fn test_extract_count_found() {
        assert_eq!(extract_count("[ahead 5, behind 3]", "ahead "), 5);
        assert_eq!(extract_count("[ahead 5, behind 3]", "behind "), 3);
    }

    #[test]
    fn test_extract_count_not_found() {
        assert_eq!(extract_count("[ahead 5]", "behind "), 0);
        assert_eq!(extract_count("no brackets", "ahead "), 0);
    }
}
