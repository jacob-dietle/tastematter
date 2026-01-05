use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Command;
use tauri::command;
use chrono::{Local, Duration, Datelike, NaiveDate};

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
    // Set working directory to GTM OS root (3 levels up from src-tauri)
    cmd.current_dir("../../..");
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

// Phase 4: Timeline types

#[derive(Serialize)]
pub struct TimeBucket {
    pub date: String,
    pub day_of_week: String,
    pub access_count: u32,
    pub files_touched: u32,
    pub sessions: Vec<String>,
}

#[derive(Serialize)]
pub struct FileTimeline {
    pub file_path: String,
    pub total_accesses: u32,
    pub buckets: HashMap<String, u32>,
    pub first_access: String,
    pub last_access: String,
}

#[derive(Serialize)]
pub struct TimelineSummary {
    pub total_accesses: u32,
    pub total_files: u32,
    pub peak_day: String,
    pub peak_count: u32,
}

#[derive(Serialize)]
pub struct TimelineData {
    pub time_range: String,
    pub start_date: String,
    pub end_date: String,
    pub buckets: Vec<TimeBucket>,
    pub files: Vec<FileTimeline>,
    pub summary: TimelineSummary,
}

fn day_of_week_abbrev(date: &NaiveDate) -> String {
    match date.weekday() {
        chrono::Weekday::Mon => "Mon",
        chrono::Weekday::Tue => "Tue",
        chrono::Weekday::Wed => "Wed",
        chrono::Weekday::Thu => "Thu",
        chrono::Weekday::Fri => "Fri",
        chrono::Weekday::Sat => "Sat",
        chrono::Weekday::Sun => "Sun",
    }.to_string()
}

#[command]
pub async fn query_timeline(
    time: String,
    files: Option<String>,
    limit: Option<u32>,
) -> Result<TimelineData, CommandError> {
    // Parse time range to get number of days
    let days: i64 = match time.as_str() {
        "7d" => 7,
        "14d" => 14,
        "30d" => 30,
        _ => 7,
    };

    // Calculate date range
    let end_date = Local::now().date_naive();
    let start_date = end_date - Duration::days(days - 1);

    // Build CLI command
    let cli_path = std::env::var("CONTEXT_OS_CLI")
        .unwrap_or_else(|_| "C:/Users/dietl/.context-os/bin/context-os.cmd".to_string());

    let mut cmd = Command::new(&cli_path);
    // Set working directory to GTM OS root (3 levels up from src-tauri)
    cmd.current_dir("../../..");
    cmd.args(["query", "flex", "--format", "json"]);
    cmd.args(["--time", &time]);
    cmd.args(["--agg", "count,recency,sessions"]);
    cmd.args(["--limit", &limit.unwrap_or(30).to_string()]);

    if let Some(f) = files {
        cmd.args(["--files", &f]);
    }

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

    let query_result: QueryResult = serde_json::from_str(&json_str).map_err(CommandError::from)?;

    // Generate buckets for each day in range
    let mut buckets = Vec::new();
    let mut date = start_date;
    while date <= end_date {
        buckets.push(TimeBucket {
            date: date.format("%Y-%m-%d").to_string(),
            day_of_week: day_of_week_abbrev(&date),
            access_count: 0,
            files_touched: 0,
            sessions: Vec::new(),
        });
        date = date + Duration::days(1);
    }

    // Transform file results to timeline format
    // Note: We're using total access counts since per-day data
    // would require enhanced CLI support. For now, distribute
    // accesses across the timeline based on recency.
    let mut files: Vec<FileTimeline> = query_result.results.iter().map(|r| {
        let mut file_buckets = HashMap::new();

        // For now, put all accesses in the most recent day
        // This is a simplification - real per-day data would come from CLI
        if let Some(ref last) = r.last_access {
            if let Some(date_str) = last.get(0..10) {
                file_buckets.insert(date_str.to_string(), r.access_count);
            }
        }

        FileTimeline {
            file_path: r.file_path.clone(),
            total_accesses: r.access_count,
            buckets: file_buckets,
            first_access: r.last_access.clone().unwrap_or_default(),
            last_access: r.last_access.clone().unwrap_or_default(),
        }
    }).collect();

    // Sort by total accesses descending
    files.sort_by(|a, b| b.total_accesses.cmp(&a.total_accesses));

    // Calculate summary
    let total_accesses = query_result.aggregations.count
        .as_ref()
        .map(|c| c.total_accesses)
        .unwrap_or(0);
    let total_files = files.len() as u32;

    // Find peak day (using end_date as fallback since we don't have per-day data)
    let peak_day = end_date.format("%Y-%m-%d").to_string();
    let peak_count = total_accesses;

    Ok(TimelineData {
        time_range: time,
        start_date: start_date.format("%Y-%m-%d").to_string(),
        end_date: end_date.format("%Y-%m-%d").to_string(),
        buckets,
        files,
        summary: TimelineSummary {
            total_accesses,
            total_files,
            peak_day,
            peak_count,
        },
    })
}

// Chain types for query_chains command

#[derive(Serialize, Deserialize)]
pub struct ChainTimeRange {
    pub start: String,
    pub end: String,
}

#[derive(Serialize, Deserialize)]
pub struct ChainData {
    pub chain_id: String,
    pub session_count: u32,
    pub file_count: u32,
    pub time_range: Option<ChainTimeRange>,
}

#[derive(Deserialize)]
pub struct ChainCliResult {
    pub command: String,
    pub timestamp: String,
    pub result_count: usize,
    pub results: Vec<ChainData>,
    pub total_chains: u32,
}

#[derive(Serialize)]
pub struct ChainQueryResult {
    pub chains: Vec<ChainData>,
    pub total_chains: u32,
}

// Phase 5: Session View types

#[derive(Serialize, Clone)]
pub struct SessionFile {
    pub file_path: String,
    pub access_count: u32,
    pub access_types: Vec<String>,
    pub last_access: String,
}

#[derive(Serialize)]
pub struct SessionData {
    pub session_id: String,
    pub chain_id: Option<String>,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub duration_seconds: Option<u32>,
    pub file_count: u32,
    pub total_accesses: u32,
    pub files: Vec<SessionFile>,
    pub top_files: Vec<SessionFile>,
}

#[derive(Serialize)]
pub struct ChainSummary {
    pub chain_id: String,
    pub session_count: u32,
    pub file_count: u32,
    pub last_active: String,
}

#[derive(Serialize)]
pub struct SessionSummary {
    pub total_sessions: u32,
    pub total_files: u32,
    pub total_accesses: u32,
    pub active_chains: u32,
}

#[derive(Serialize)]
pub struct SessionQueryResult {
    pub time_range: String,
    pub sessions: Vec<SessionData>,
    pub chains: Vec<ChainSummary>,
    pub summary: SessionSummary,
}

#[command]
pub async fn query_sessions(
    time: String,
    chain: Option<String>,
    limit: Option<u32>,
) -> Result<SessionQueryResult, CommandError> {
    let cli_path = std::env::var("CONTEXT_OS_CLI")
        .unwrap_or_else(|_| "C:/Users/dietl/.context-os/bin/context-os.cmd".to_string());

    // Build CLI command
    let mut cmd = Command::new(&cli_path);
    cmd.current_dir("../../..");
    cmd.args(["query", "flex", "--format", "json"]);
    cmd.args(["--time", &time]);
    cmd.args(["--agg", "count,recency,sessions"]);
    cmd.args(["--sort", "recency"]);

    if let Some(c) = &chain {
        cmd.args(["--chain", c]);
    }

    let lim = limit.unwrap_or(50);
    cmd.args(["--limit", &lim.to_string()]);

    let output = cmd.output().map_err(|e| CommandError {
        code: "CLI_NOT_FOUND".to_string(),
        message: format!("context-os CLI not found at: {}", cli_path),
        details: Some(e.to_string()),
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(CommandError {
            code: "CLI_ERROR".to_string(),
            message: "Session query failed".to_string(),
            details: Some(stderr.to_string()),
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let query_result: QueryResult = serde_json::from_str(&stdout)
        .map_err(|e| CommandError {
            code: "PARSE_ERROR".to_string(),
            message: "Failed to parse query result".to_string(),
            details: Some(e.to_string()),
        })?;

    transform_to_sessions(query_result, &time)
}

fn transform_to_sessions(
    query_result: QueryResult,
    time_range: &str,
) -> Result<SessionQueryResult, CommandError> {
    // Group files by session
    let mut session_map: HashMap<String, Vec<SessionFile>> = HashMap::new();
    let mut chain_map: HashMap<String, (u32, u32, String)> = HashMap::new(); // (session_count, file_count, last_active)

    for result in &query_result.results {
        let file = SessionFile {
            file_path: result.file_path.clone(),
            access_count: result.access_count,
            access_types: vec!["read".to_string()], // Simplified - CLI doesn't provide per-file access types
            last_access: result.last_access.clone().unwrap_or_default(),
        };

        // Get sessions for this file
        if let Some(sessions) = &result.sessions {
            for session_id in sessions {
                if session_id.is_empty() { continue; }
                session_map.entry(session_id.clone())
                    .or_insert_with(Vec::new)
                    .push(file.clone());
            }
        }

        // Track chains
        if let Some(chains) = &result.chains {
            for chain_id in chains {
                if chain_id.is_empty() { continue; }
                let entry = chain_map.entry(chain_id.clone()).or_insert((0, 0, String::new()));
                entry.1 += 1; // file_count
                if result.last_access.as_ref().map(|a| a.as_str()).unwrap_or("") > entry.2.as_str() {
                    entry.2 = result.last_access.clone().unwrap_or_default();
                }
            }
        }
    }

    // Build session data
    let mut sessions: Vec<SessionData> = session_map.into_iter().map(|(session_id, files)| {
        let total_accesses: u32 = files.iter().map(|f| f.access_count).sum();
        let file_count = files.len() as u32;

        // Sort by access count for top files
        let mut sorted_files = files.clone();
        sorted_files.sort_by(|a, b| b.access_count.cmp(&a.access_count));
        let top_files: Vec<SessionFile> = sorted_files.iter().take(3).cloned().collect();

        // Get the most recent access as started_at
        let started_at = sorted_files.first()
            .map(|f| f.last_access.clone())
            .unwrap_or_default();

        SessionData {
            session_id,
            chain_id: None, // CLI doesn't provide chain per session yet
            started_at,
            ended_at: None,
            duration_seconds: None,
            file_count,
            total_accesses,
            files: sorted_files,
            top_files,
        }
    }).collect();

    // Sort sessions by recency (started_at descending)
    sessions.sort_by(|a, b| b.started_at.cmp(&a.started_at));

    // Build chain summaries
    let chains: Vec<ChainSummary> = chain_map.into_iter().map(|(chain_id, (session_count, file_count, last_active))| {
        ChainSummary {
            chain_id,
            session_count,
            file_count,
            last_active,
        }
    }).collect();

    // Build summary
    let total_sessions = sessions.len() as u32;
    let total_files: u32 = sessions.iter().map(|s| s.file_count).sum();
    let total_accesses: u32 = sessions.iter().map(|s| s.total_accesses).sum();
    let active_chains = chains.len() as u32;

    Ok(SessionQueryResult {
        time_range: time_range.to_string(),
        sessions,
        chains,
        summary: SessionSummary {
            total_sessions,
            total_files,
            total_accesses,
            active_chains,
        },
    })
}

// Chain query command

#[command]
pub async fn query_chains(
    limit: Option<u32>,
) -> Result<ChainQueryResult, CommandError> {
    let cli_path = std::env::var("CONTEXT_OS_CLI")
        .unwrap_or_else(|_| "C:/Users/dietl/.context-os/bin/context-os.cmd".to_string());

    let mut cmd = Command::new(&cli_path);
    cmd.current_dir("../../..");
    cmd.args(["query", "chains", "--format", "json"]);
    cmd.args(["--limit", &limit.unwrap_or(20).to_string()]);

    let output = cmd.output().map_err(|e| CommandError {
        code: "CLI_NOT_FOUND".to_string(),
        message: format!("context-os CLI not found at: {}", cli_path),
        details: Some(e.to_string()),
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(CommandError {
            code: "CLI_ERROR".to_string(),
            message: "Chain query failed".to_string(),
            details: Some(stderr.to_string()),
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let cli_result: ChainCliResult = serde_json::from_str(&stdout)
        .map_err(|e| CommandError {
            code: "PARSE_ERROR".to_string(),
            message: "Failed to parse chain query result".to_string(),
            details: Some(e.to_string()),
        })?;

    Ok(ChainQueryResult {
        chains: cli_result.results,
        total_chains: cli_result.total_chains,
    })
}
