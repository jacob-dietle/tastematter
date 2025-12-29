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
