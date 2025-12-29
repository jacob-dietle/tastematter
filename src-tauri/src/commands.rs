use serde::{Deserialize, Serialize};
use std::process::Command;
use tauri::command;

// Result types matching TypeScript interfaces

#[derive(Serialize, Deserialize)]
pub struct QueryResult {
    pub receipt_id: String,
    pub timestamp: String,
    pub result_count: usize,
    pub results: Vec<FileResult>,
    pub aggregations: Aggregations,
}

#[derive(Serialize, Deserialize)]
pub struct FileResult {
    pub file_path: String,
    pub access_count: u32,
    pub last_access: Option<String>,
    pub session_count: Option<u32>,
    pub sessions: Option<Vec<String>>,
    pub chains: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize)]
pub struct Aggregations {
    pub count: Option<CountAgg>,
    pub recency: Option<RecencyAgg>,
}

#[derive(Serialize, Deserialize)]
pub struct CountAgg {
    pub total_files: u32,
    pub total_accesses: u32,
}

#[derive(Serialize, Deserialize)]
pub struct RecencyAgg {
    pub newest: String,
    pub oldest: String,
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

impl From<serde_json::Error> for CommandError {
    fn from(err: serde_json::Error) -> Self {
        CommandError {
            code: "JSON_PARSE_ERROR".to_string(),
            message: "Failed to parse CLI output".to_string(),
            details: Some(err.to_string()),
        }
    }
}

// Main query command

#[command]
pub async fn query_flex(
    files: Option<String>,
    time: Option<String>,
    chain: Option<String>,
    session: Option<String>,
    agg: Vec<String>,
    limit: Option<u32>,
    sort: Option<String>,
) -> Result<QueryResult, CommandError> {
    // Build command with context-os CLI path
    let cli_path = std::env::var("CONTEXT_OS_CLI")
        .unwrap_or_else(|_| "C:/Users/dietl/.context-os/bin/context-os.cmd".to_string());

    let mut cmd = Command::new(&cli_path);
    cmd.args(["query", "flex", "--format", "json"]);

    if let Some(f) = files {
        cmd.args(["--files", &f]);
    }
    if let Some(t) = time {
        cmd.args(["--time", &t]);
    }
    if let Some(c) = chain {
        cmd.args(["--chain", &c]);
    }
    if let Some(s) = session {
        cmd.args(["--session", &s]);
    }
    if let Some(l) = limit {
        cmd.args(["--limit", &l.to_string()]);
    }
    if let Some(s) = sort {
        cmd.args(["--sort", &s]);
    }

    cmd.args(["--agg", &agg.join(",")]);

    let output = cmd.output().map_err(|e| CommandError {
        code: "CLI_NOT_FOUND".to_string(),
        message: format!("context-os CLI not found at: {}", cli_path),
        details: Some(e.to_string()),
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(CommandError {
            code: "CLI_ERROR".to_string(),
            message: "CLI command failed".to_string(),
            details: Some(stderr.to_string()),
        });
    }

    let json_str = String::from_utf8(output.stdout).map_err(|e| CommandError {
        code: "UTF8_ERROR".to_string(),
        message: "Invalid UTF-8 in CLI output".to_string(),
        details: Some(e.to_string()),
    })?;

    serde_json::from_str(&json_str).map_err(CommandError::from)
}

// Git command types

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
    // Get current branch
    let branch_output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .map_err(|e| CommandError {
            code: "GIT_NOT_FOUND".to_string(),
            message: "Git not found. Please ensure git is installed and in PATH.".to_string(),
            details: Some(e.to_string()),
        })?;

    let branch = String::from_utf8_lossy(&branch_output.stdout)
        .trim()
        .to_string();

    // Get ahead/behind counts
    let (ahead, behind) = get_ahead_behind();

    // Get file status
    let status_output = Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .map_err(|e| CommandError {
            code: "GIT_ERROR".to_string(),
            message: "Failed to get git status".to_string(),
            details: Some(e.to_string()),
        })?;

    let status_str = String::from_utf8_lossy(&status_output.stdout);
    let (staged, modified, untracked, has_conflicts) = parse_porcelain_status(&status_str);

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

fn get_ahead_behind() -> (u32, u32) {
    // Get ahead count
    let ahead_output = Command::new("git")
        .args(["rev-list", "--count", "@{u}..HEAD"])
        .output();

    let ahead = match ahead_output {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout)
                .trim()
                .parse()
                .unwrap_or(0)
        }
        _ => 0, // No upstream or error
    };

    // Get behind count
    let behind_output = Command::new("git")
        .args(["rev-list", "--count", "HEAD..@{u}"])
        .output();

    let behind = match behind_output {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout)
                .trim()
                .parse()
                .unwrap_or(0)
        }
        _ => 0,
    };

    (ahead, behind)
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

        // Check for conflicts
        if index_status == 'U' || worktree_status == 'U' {
            has_conflicts = true;
        }

        // Staged changes (index has changes)
        if index_status != ' ' && index_status != '?' {
            staged.push(file_path.clone());
        }

        // Modified in worktree (not staged)
        if worktree_status == 'M' || worktree_status == 'D' {
            modified.push(file_path.clone());
        }

        // Untracked files
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
        // Parse files affected from output
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
    // Parse "X files changed" from git output
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

    // Git push outputs to stderr even on success
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
