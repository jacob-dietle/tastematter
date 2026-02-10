//! Tastematter CLI - Context intelligence for Claude Code
//!
//! A standalone CLI for querying Claude Code session data with <100ms latency.
//!
//! # Usage
//!
//! ```bash
//! # Query most accessed files in the last 7 days
//! tastematter query flex --time 7d
//!
//! # Query chains
//! tastematter query chains --limit 5
//!
//! # Query timeline data
//! tastematter query timeline --time 14d
//!
//! # Query sessions
//! tastematter query sessions --time 7d
//! ```

use clap::{Parser, Subcommand};
use std::collections::HashMap;
use std::path::PathBuf;
use tastematter::{
    capture::file_watcher::{
        create_event_from_path, event_types, EventDebouncer, EventFilter, WatcherStats,
    },
    capture::git_sync::{sync_commits, SyncOptions, SyncResult},
    capture::jsonl_parser::{sync_sessions, ParseOptions, ParseResult, SessionSummary},
    daemon::{get_platform, load_config, run_sync, DaemonPlatform, InstallConfig},
    index::chain_graph::{build_chain_graph, ChainBuildResult},
    index::inverted_index::{build_inverted_index, get_sessions_for_file, IndexBuildResult},
    intelligence::{ChainNamingRequest, IntelClient},
    CommandExecutedEvent, ContextRestoreInput, Database, HeatSortBy, QueryChainsInput,
    QueryCoAccessInput, QueryEngine, QueryFileInput, QueryFlexInput, QueryHeatInput,
    QueryReceiptsInput, QuerySearchInput, QuerySessionsInput, QueryTimelineInput, QueryVerifyInput,
    TimeRangeBucket,
};

#[derive(Parser)]
#[command(name = "tastematter")]
#[command(version = env!("TASTEMATTER_VERSION"))]
#[command(about = "Tastematter - Context intelligence for Claude Code", long_about = None)]
struct Cli {
    /// Database path (optional, auto-discovers if not provided)
    #[arg(long, global = true)]
    db: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Query commands
    Query {
        #[command(subcommand)]
        query_type: QueryCommands,
    },
    /// Start HTTP API server for development
    Serve {
        /// Port to listen on
        #[arg(long, default_value = "3001")]
        port: u16,

        /// Host to bind to
        #[arg(long, default_value = "127.0.0.1")]
        host: String,

        /// Enable CORS for browser access
        #[arg(long)]
        cors: bool,
    },
    /// Sync git commits from repository
    SyncGit {
        /// Time range to sync (e.g., "90 days", "2025-01-01")
        #[arg(long)]
        since: Option<String>,

        /// Upper bound date
        #[arg(long)]
        until: Option<String>,

        /// Path to git repository
        #[arg(long, default_value = ".")]
        repo: String,

        /// Output format: json (default) or compact
        #[arg(long, default_value = "json")]
        format: String,
    },
    /// Parse JSONL session files and extract tool uses
    ParseSessions {
        /// Path to Claude directory (default: ~/.claude)
        #[arg(long)]
        claude_dir: Option<String>,

        /// Filter to specific project path
        #[arg(long)]
        project: Option<String>,

        /// Enable incremental mode (skip unchanged files)
        #[arg(long)]
        incremental: bool,

        /// Output format: json (default), compact, or summary
        #[arg(long, default_value = "summary")]
        format: String,
    },
    /// Build chain graph from session linking relationships
    BuildChains {
        /// Path to Claude directory (default: ~/.claude)
        #[arg(long)]
        claude_dir: Option<String>,

        /// Filter to specific project path
        #[arg(long)]
        project: Option<String>,

        /// Output format: json (default), compact, or summary
        #[arg(long, default_value = "summary")]
        format: String,
    },
    /// Build inverted file index (file -> sessions mapping)
    IndexFiles {
        /// Path to Claude directory (default: ~/.claude)
        #[arg(long)]
        claude_dir: Option<String>,

        /// Filter to specific project path
        #[arg(long)]
        project: Option<String>,

        /// Output format: summary, json, or compact
        #[arg(long, default_value = "summary")]
        format: String,

        /// Query which sessions touched a specific file
        #[arg(long)]
        query: Option<String>,
    },
    /// Watch a directory for file changes
    Watch {
        /// Directory to watch
        #[arg(long, default_value = ".")]
        path: String,

        /// Debounce window in milliseconds
        #[arg(long, default_value = "100")]
        debounce_ms: u64,

        /// Run for a maximum duration (seconds), then exit
        #[arg(long)]
        duration: Option<u64>,

        /// Watch recursively
        #[arg(long, default_value = "true")]
        recursive: bool,
    },
    /// Daemon commands for background sync
    Daemon {
        #[command(subcommand)]
        daemon_cmd: DaemonCommands,
    },
    /// Intelligence commands for AI-powered analysis
    Intel {
        #[command(subcommand)]
        intel_cmd: IntelCommands,
    },
    /// Restore context for a topic — composed query across flex, heat, chains, sessions, timeline, co-access
    Context {
        /// Search query (used as glob pattern *query*)
        query: String,

        /// Time window (default: 30d)
        #[arg(short, long, default_value = "30d")]
        time: String,

        /// Maximum results per sub-query
        #[arg(short, long, default_value = "20")]
        limit: u32,

        /// Output format: json (default), compact, table
        #[arg(long, default_value = "json")]
        format: String,
    },
}

#[derive(Subcommand)]
enum DaemonCommands {
    /// Run a single sync cycle and exit
    #[command(name = "once")]
    Once {
        /// Project path filter
        #[arg(long)]
        project: Option<String>,
    },
    /// Start the daemon (foreground)
    Start {
        /// Sync interval in minutes
        #[arg(long, default_value = "30")]
        interval: u32,

        /// Project path filter
        #[arg(long)]
        project: Option<String>,
    },
    /// Show daemon status (sync state + platform registration)
    Status,
    /// Install daemon to run on login
    Install {
        /// Sync interval in minutes
        #[arg(long, default_value = "30")]
        interval: u32,
    },
    /// Uninstall daemon from login
    Uninstall,
}

#[derive(Subcommand)]
enum IntelCommands {
    /// Check intel service health
    Health,
    /// Name a chain using AI
    #[command(name = "name-chain")]
    NameChain {
        /// Chain ID to name
        chain_id: String,
        /// Comma-separated list of files touched
        #[arg(long)]
        files: Option<String>,
        /// Number of sessions in chain
        #[arg(long, default_value = "1")]
        session_count: i32,
    },
}

#[derive(Subcommand)]
enum QueryCommands {
    /// Flexible file query with filters
    Flex {
        /// Time range filter: 7d, 14d, 30d, or custom Nd
        #[arg(short, long, default_value = "7d")]
        time: String,

        /// Filter by chain ID
        #[arg(short, long)]
        chain: Option<String>,

        /// File path pattern filter (glob-style: *.rs, src/*)
        #[arg(short, long)]
        files: Option<String>,

        /// Filter by session ID
        #[arg(short, long)]
        session: Option<String>,

        /// Aggregations to compute: count, recency
        #[arg(short, long, value_delimiter = ',')]
        agg: Vec<String>,

        /// Maximum results to return
        #[arg(short, long, default_value = "20")]
        limit: u32,

        /// Sort order: count (default) or recency
        #[arg(long)]
        sort: Option<String>,

        /// Output format: json (default) or compact
        #[arg(long, default_value = "json")]
        format: String,
    },

    /// Query chain metadata
    Chains {
        /// Maximum chains to return
        #[arg(short, long, default_value = "20")]
        limit: u32,

        /// Output format: json (default) or compact
        #[arg(long, default_value = "json")]
        format: String,
    },

    /// Query timeline data for visualization
    Timeline {
        /// Time range: 7d, 14d, 30d, or custom Nd
        #[arg(short, long, default_value = "7d")]
        time: String,

        /// File path pattern filter
        #[arg(short = 'p', long)]
        files: Option<String>,

        /// Filter by chain ID
        #[arg(short, long)]
        chain: Option<String>,

        /// Maximum files to include
        #[arg(short, long, default_value = "30")]
        limit: u32,

        /// Output format: json (default) or compact
        #[arg(long, default_value = "json")]
        format: String,
    },

    /// Query session-grouped data
    Sessions {
        /// Time range: 7d, 14d, 30d, or custom Nd
        #[arg(short, long, default_value = "7d")]
        time: String,

        /// Filter by chain ID
        #[arg(short, long)]
        chain: Option<String>,

        /// Maximum sessions to return
        #[arg(short, long, default_value = "50")]
        limit: u32,

        /// Output format: json (default) or compact
        #[arg(long, default_value = "json")]
        format: String,
    },

    /// Search files by pattern (substring match)
    Search {
        /// Pattern to search for (case-insensitive substring)
        pattern: String,

        /// Maximum results to return
        #[arg(short, long, default_value = "20")]
        limit: u32,

        /// Output format: json (default) or compact
        #[arg(long, default_value = "json")]
        format: String,
    },

    /// Show sessions that touched a file
    File {
        /// File path to query (exact, suffix, or substring match)
        file_path: String,

        /// Maximum sessions to return
        #[arg(short, long, default_value = "20")]
        limit: u32,

        /// Output format: json (default) or compact
        #[arg(long, default_value = "json")]
        format: String,
    },

    /// Show files frequently co-accessed with a file
    CoAccess {
        /// Anchor file to find co-accessed files for
        file_path: String,

        /// Maximum results to return
        #[arg(short, long, default_value = "10")]
        limit: u32,

        /// Output format: json (default) or compact
        #[arg(long, default_value = "json")]
        format: String,
    },

    /// Show file heat metrics (RCR, velocity, composite score)
    Heat {
        /// Long window time range: 30d (default), 14d, 60d, 90d
        #[arg(short, long, default_value = "30d")]
        time: String,

        /// File path pattern filter (glob-style)
        #[arg(short, long)]
        files: Option<String>,

        /// Maximum results to return
        #[arg(short, long, default_value = "50")]
        limit: u32,

        /// Sort by: heat (default), rcr, velocity, name
        #[arg(short, long, default_value = "heat")]
        sort: String,

        /// Output format: table (default), json, compact, csv
        #[arg(long, default_value = "table")]
        format: String,
    },

    /// Verify a query receipt against current data
    Verify {
        /// Receipt ID to verify (e.g., q_abc123)
        receipt_id: String,

        /// Output format: json (default) or compact
        #[arg(long, default_value = "json")]
        format: String,
    },

    /// List recent query receipts from the ledger
    Receipts {
        /// Maximum receipts to return
        #[arg(short, long, default_value = "20")]
        limit: u32,

        /// Output format: json (default) or compact
        #[arg(long, default_value = "json")]
        format: String,
    },
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Initialize telemetry (fire-and-forget, never blocks)
    let telemetry = tastematter::TelemetryClient::init();
    let start_time = std::time::Instant::now();

    // Track result count and time range for telemetry enrichment
    let mut result_count: Option<u32> = None;
    let mut time_range_bucket: Option<TimeRangeBucket> = None;

    // Extract command name and time range for telemetry
    let command_name = match &cli.command {
        Commands::Query { query_type } => match query_type {
            QueryCommands::Flex { time, .. } => {
                time_range_bucket = Some(TimeRangeBucket::from_time_arg(time));
                "query_flex"
            }
            QueryCommands::Chains { .. } => "query_chains",
            QueryCommands::Timeline { time, .. } => {
                time_range_bucket = Some(TimeRangeBucket::from_time_arg(time));
                "query_timeline"
            }
            QueryCommands::Sessions { time, .. } => {
                time_range_bucket = Some(TimeRangeBucket::from_time_arg(time));
                "query_sessions"
            }
            QueryCommands::Search { .. } => "query_search",
            QueryCommands::File { .. } => "query_file",
            QueryCommands::CoAccess { .. } => "query_coaccess",
            QueryCommands::Heat { time, .. } => {
                time_range_bucket = Some(TimeRangeBucket::from_time_arg(time));
                "query_heat"
            }
            QueryCommands::Verify { .. } => "query_verify",
            QueryCommands::Receipts { .. } => "query_receipts",
        },
        Commands::Serve { .. } => "serve",
        Commands::SyncGit { .. } => "sync_git",
        Commands::ParseSessions { .. } => "parse_sessions",
        Commands::BuildChains { .. } => "build_chains",
        Commands::IndexFiles { .. } => "index_files",
        Commands::Watch { .. } => "watch",
        Commands::Daemon { daemon_cmd } => match daemon_cmd {
            DaemonCommands::Once { .. } => "daemon_once",
            DaemonCommands::Start { .. } => "daemon_start",
            DaemonCommands::Status => "daemon_status",
            DaemonCommands::Install { .. } => "daemon_install",
            DaemonCommands::Uninstall => "daemon_uninstall",
        },
        Commands::Intel { intel_cmd } => match intel_cmd {
            IntelCommands::Health => "intel_health",
            IntelCommands::NameChain { .. } => "intel_name_chain",
        },
        Commands::Context { ref time, .. } => {
            time_range_bucket = Some(TimeRangeBucket::from_time_arg(time));
            "context"
        }
    };

    // Handle daemon commands FIRST - they manage their own database lifecycle
    // (create directory, create schema on fresh install)
    if let Commands::Daemon { ref daemon_cmd } = cli.command {
        // Daemon commands are handled separately because they:
        // 1. Create the database directory if it doesn't exist
        // 2. Create the schema on fresh install
        // 3. Don't need a pre-existing database
        let daemon_result: Result<(), Box<dyn std::error::Error>> = match daemon_cmd {
            DaemonCommands::Once { project } => {
                eprintln!("Running single sync cycle...");
                let mut config = load_config(None).map_err(|e| format!("Config error: {}", e))?;
                if let Some(p) = project {
                    config.project.path = Some(p.clone());
                }
                let result = run_sync(&config)
                    .await
                    .map_err(|e| format!("Sync error: {}", e))?;
                println!("{}", serde_json::to_string_pretty(&result)?);
                if !result.errors.is_empty() {
                    eprintln!("\nWarnings/Errors:");
                    for err in &result.errors {
                        eprintln!("  - {}", err);
                    }
                }
                eprintln!(
                    "\nSync complete: {} commits, {} sessions, {} chains, {} files indexed in {}ms",
                    result.git_commits_synced,
                    result.sessions_parsed,
                    result.chains_built,
                    result.files_indexed,
                    result.duration_ms
                );
                Ok(())
            }
            DaemonCommands::Start { interval, project } => {
                let interval_mins = *interval;
                eprintln!("Starting daemon (sync every {} min)...", interval_mins);
                let mut config = load_config(None).map_err(|e| format!("Config error: {}", e))?;
                config.sync.interval_minutes = interval_mins;
                if let Some(p) = project {
                    config.project.path = Some(p.clone());
                }
                let interval_duration = std::time::Duration::from_secs(interval_mins as u64 * 60);
                loop {
                    let start = std::time::Instant::now();
                    match run_sync(&config).await {
                        Ok(result) => {
                            eprintln!(
                                "[{}] Sync: {} commits, {} sessions, {} chains in {}ms",
                                chrono::Local::now().format("%H:%M:%S"),
                                result.git_commits_synced,
                                result.sessions_parsed,
                                result.chains_built,
                                result.duration_ms
                            );
                        }
                        Err(e) => eprintln!(
                            "[{}] Sync error: {}",
                            chrono::Local::now().format("%H:%M:%S"),
                            e
                        ),
                    }
                    let elapsed = start.elapsed();
                    if elapsed < interval_duration {
                        tokio::time::sleep(interval_duration - elapsed).await;
                    }
                }
            }
            DaemonCommands::Status => {
                let platform = get_platform();
                match platform.status() {
                    Ok(status) => {
                        println!("Platform: {}", status.platform_name);
                        println!("Installed: {}", if status.installed { "Yes" } else { "No" });
                        println!("Running: {}", if status.running { "Yes" } else { "No" });
                        if let Some(last_run) = status.last_run {
                            println!("Last run: {}", last_run.format("%Y-%m-%d %H:%M:%S"));
                        }
                        if let Some(next_run) = status.next_run {
                            println!("Next run: {}", next_run.format("%Y-%m-%d %H:%M:%S"));
                        }
                        if !status.message.is_empty() {
                            println!("Details: {}", status.message);
                        }
                        if !status.installed {
                            eprintln!("\nTo install as a background service:");
                            eprintln!("  tastematter daemon install");
                        }
                    }
                    Err(e) => {
                        eprintln!("Could not get daemon status: {}", e);
                    }
                }
                Ok(())
            }
            DaemonCommands::Install { interval } => {
                let interval_mins = *interval;
                let platform = get_platform();
                let binary_path = std::env::current_exe()
                    .map_err(|e| format!("Could not determine binary path: {}", e))?;
                let config = InstallConfig {
                    binary_path,
                    interval_minutes: interval_mins,
                    ..InstallConfig::default()
                };
                match platform.install(&config) {
                    Ok(result) => {
                        println!("{}", result.message);
                        if result.success {
                            println!("The daemon will sync every {} minutes.", interval_mins);
                            println!("\nTo check status: tastematter daemon status");
                            println!("To uninstall: tastematter daemon uninstall");
                        }
                        if let Some(details) = result.details {
                            println!("Details: {}", details);
                        }
                    }
                    Err(e) => eprintln!("Failed to install daemon: {}", e),
                }
                Ok(())
            }
            DaemonCommands::Uninstall => {
                let platform = get_platform();
                match platform.uninstall() {
                    Ok(()) => println!("Daemon uninstalled successfully!"),
                    Err(e) => eprintln!("Failed to uninstall daemon: {}", e),
                }
                Ok(())
            }
        };

        // Send telemetry for daemon commands (fire-and-forget)
        let mut event = CommandExecutedEvent::new(
            command_name,
            start_time.elapsed().as_millis() as u64,
            daemon_result.is_ok(),
        );
        if let Some(bucket) = time_range_bucket {
            event = event.with_time_range(bucket);
        }
        telemetry.capture_command(event);

        return daemon_result;
    }

    // For non-daemon commands, open database (auto-creates on fresh machines)
    let db = if let Some(ref path) = cli.db {
        Database::open(path).await?
    } else {
        Database::open_or_create_default().await?
    };

    let engine = QueryEngine::new(db).with_intel(IntelClient::default());

    match cli.command {
        // Daemon commands already handled above
        Commands::Daemon { .. } => unreachable!("Daemon commands handled above"),
        Commands::Query { query_type } => match query_type {
            QueryCommands::Flex {
                time,
                chain,
                files,
                session,
                agg,
                limit,
                sort,
                format,
            } => {
                let input = QueryFlexInput {
                    files,
                    time: Some(time),
                    chain,
                    session,
                    agg,
                    limit: Some(limit),
                    sort,
                };
                let query_result = engine.query_flex(input).await?;
                result_count = Some(query_result.result_count as u32);
                output(&query_result, &format)?;
            }
            QueryCommands::Chains { limit, format } => {
                let input = QueryChainsInput { limit: Some(limit) };
                let query_result = engine.query_chains(input).await?;
                result_count = Some(query_result.chains.len() as u32);
                match format.as_str() {
                    "table" => output_chains_table(&query_result),
                    _ => output(&query_result, &format)?,
                }
            }
            QueryCommands::Timeline {
                time,
                files,
                chain,
                limit,
                format,
            } => {
                let input = QueryTimelineInput {
                    time,
                    files,
                    chain,
                    limit: Some(limit),
                };
                let query_result = engine.query_timeline(input).await?;
                result_count = Some(query_result.files.len() as u32);
                output(&query_result, &format)?;
            }
            QueryCommands::Sessions {
                time,
                chain,
                limit,
                format,
            } => {
                let input = QuerySessionsInput {
                    time,
                    chain,
                    limit: Some(limit),
                };
                let query_result = engine.query_sessions(input).await?;
                result_count = Some(query_result.sessions.len() as u32);
                output(&query_result, &format)?;
            }
            QueryCommands::Search {
                pattern,
                limit,
                format,
            } => {
                let input = QuerySearchInput {
                    pattern,
                    limit: Some(limit),
                };
                let query_result = engine.query_search(input).await?;
                result_count = Some(query_result.total_matches as u32);
                output(&query_result, &format)?;
            }
            QueryCommands::File {
                file_path,
                limit,
                format,
            } => {
                let input = QueryFileInput {
                    file_path,
                    limit: Some(limit),
                };
                let query_result = engine.query_file(input).await?;
                result_count = Some(query_result.sessions.len() as u32);
                output(&query_result, &format)?;
            }
            QueryCommands::CoAccess {
                file_path,
                limit,
                format,
            } => {
                let input = QueryCoAccessInput {
                    file_path,
                    limit: Some(limit),
                };
                let query_result = engine.query_co_access(input).await?;
                result_count = Some(query_result.results.len() as u32);
                output(&query_result, &format)?;
            }
            QueryCommands::Heat {
                time,
                files,
                limit,
                sort,
                format,
            } => {
                let sort_by = match sort.as_str() {
                    "rcr" => HeatSortBy::Rcr,
                    "velocity" => HeatSortBy::Velocity,
                    "name" => HeatSortBy::Name,
                    _ => HeatSortBy::Heat,
                };
                let input = QueryHeatInput {
                    time: Some(time),
                    files,
                    limit: Some(limit),
                    sort: Some(sort_by),
                };
                let query_result = engine.query_heat(input).await?;
                result_count = Some(query_result.results.len() as u32);
                match format.as_str() {
                    "table" => output_heat_table(&query_result),
                    "csv" => output_heat_csv(&query_result),
                    _ => output(&query_result, &format)?,
                }
            }
            QueryCommands::Verify { receipt_id, format } => {
                let input = QueryVerifyInput { receipt_id };
                let query_result = engine.query_verify(input).await?;
                output(&query_result, &format)?;
            }
            QueryCommands::Receipts { limit, format } => {
                let input = QueryReceiptsInput { limit: Some(limit) };
                let query_result = engine.query_receipts(input).await?;
                result_count = Some(query_result.receipts.len() as u32);
                output(&query_result, &format)?;
            }
        },
        Commands::Serve { port, host, cors } => {
            use std::sync::Arc;
            use std::time::Instant;
            use tastematter::http::{create_router, AppState};

            let state = Arc::new(AppState {
                engine,
                start_time: Instant::now(),
            });

            let router = create_router(state, cors);
            let addr = format!("{}:{}", host, port);

            println!("Starting HTTP API server on http://{}", addr);
            println!("Press Ctrl+C to stop");

            let listener = tokio::net::TcpListener::bind(&addr).await?;
            axum::serve(listener, router).await?;
        }
        Commands::SyncGit {
            since,
            until,
            repo,
            format,
        } => {
            let options = SyncOptions {
                since,
                until,
                repo_path: Some(repo),
                incremental: true,
            };

            eprintln!("Syncing git commits...");

            match sync_commits(&options) {
                Ok((commits, result)) => {
                    // Output summary
                    eprintln!(
                        "Synced {} commits ({} agent commits detected)",
                        result.commits_synced,
                        commits.iter().filter(|c| c.is_agent_commit).count()
                    );

                    if !result.errors.is_empty() {
                        eprintln!("Warnings: {} parse errors", result.errors.len());
                    }

                    // Output structured result
                    #[derive(serde::Serialize)]
                    struct SyncOutput {
                        result: SyncResult,
                        commits: Vec<tastematter::capture::git_sync::GitCommit>,
                    }

                    let output_data = SyncOutput { result, commits };
                    output(&output_data, &format)?;
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::ParseSessions {
            claude_dir,
            project,
            incremental,
            format,
        } => {
            // Determine Claude directory
            let claude_path = if let Some(dir) = claude_dir {
                PathBuf::from(dir)
            } else {
                // Default to ~/.claude
                dirs::home_dir()
                    .ok_or("Could not determine home directory")?
                    .join(".claude")
            };

            if !claude_path.exists() {
                eprintln!("Claude directory not found: {}", claude_path.display());
                std::process::exit(1);
            }

            let options = ParseOptions {
                incremental,
                project_filter: project,
            };

            // For incremental mode, we'd load existing session sizes from DB
            // For now, use empty map (full sync)
            let existing: HashMap<String, i64> = HashMap::new();

            eprintln!("Parsing sessions from: {}", claude_path.display());
            eprintln!("Incremental: {}", incremental);

            match sync_sessions(&claude_path, &options, &existing) {
                Ok((summaries, result)) => {
                    eprintln!(
                        "Parsed {} sessions ({} skipped), {} total tool uses",
                        result.sessions_parsed, result.sessions_skipped, result.total_tool_uses
                    );

                    if !result.errors.is_empty() {
                        eprintln!("Errors: {}", result.errors.len());
                        for err in result.errors.iter().take(5) {
                            eprintln!("  - {}", err);
                        }
                    }

                    // Output based on format
                    match format.as_str() {
                        "summary" => {
                            println!("Sessions parsed: {}", result.sessions_parsed);
                            println!("Sessions skipped: {}", result.sessions_skipped);
                            println!("Total tool uses: {}", result.total_tool_uses);
                            println!("Errors: {}", result.errors.len());
                        }
                        "json" => {
                            #[derive(serde::Serialize)]
                            struct ParseOutput {
                                result: ParseResult,
                                sessions: Vec<SessionSummary>,
                            }
                            let output_data = ParseOutput {
                                result,
                                sessions: summaries,
                            };
                            println!("{}", serde_json::to_string_pretty(&output_data)?);
                        }
                        "compact" => {
                            #[derive(serde::Serialize)]
                            struct ParseOutput {
                                result: ParseResult,
                                sessions: Vec<SessionSummary>,
                            }
                            let output_data = ParseOutput {
                                result,
                                sessions: summaries,
                            };
                            println!("{}", serde_json::to_string(&output_data)?);
                        }
                        _ => {
                            println!("Sessions parsed: {}", result.sessions_parsed);
                            println!("Total tool uses: {}", result.total_tool_uses);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::BuildChains {
            claude_dir,
            project,
            format,
        } => {
            use tastematter::capture::jsonl_parser::encode_project_path;

            // Determine Claude directory
            let claude_path = if let Some(dir) = claude_dir {
                PathBuf::from(dir)
            } else {
                dirs::home_dir()
                    .ok_or("Could not determine home directory")?
                    .join(".claude")
            };

            if !claude_path.exists() {
                eprintln!("Claude directory not found: {}", claude_path.display());
                std::process::exit(1);
            }

            // Determine the project directory to scan
            let projects_dir = if let Some(ref project_path) = project {
                // Encode the project path and look in specific directory
                let encoded = encode_project_path(std::path::Path::new(project_path));
                claude_path.join("projects").join(&encoded)
            } else {
                // Scan all projects
                claude_path.join("projects")
            };

            if !projects_dir.exists() {
                eprintln!("Projects directory not found: {}", projects_dir.display());
                std::process::exit(1);
            }

            eprintln!("Building chain graph from: {}", projects_dir.display());

            match build_chain_graph(&projects_dir) {
                Ok(chains) => {
                    // Calculate statistics
                    let chains_built = chains.len() as u32;
                    let sessions_linked: u32 =
                        chains.values().map(|c| c.sessions.len() as u32).sum();
                    let largest_chain =
                        chains.values().map(|c| c.sessions.len()).max().unwrap_or(0);
                    let orphan_sessions =
                        chains.values().filter(|c| c.sessions.len() == 1).count() as u32;

                    eprintln!(
                        "Built {} chains with {} sessions (largest: {} sessions)",
                        chains_built, sessions_linked, largest_chain
                    );

                    let result = ChainBuildResult {
                        chains_built,
                        sessions_linked,
                        orphan_sessions,
                    };

                    match format.as_str() {
                        "summary" => {
                            println!("Chains built: {}", result.chains_built);
                            println!("Sessions linked: {}", result.sessions_linked);
                            println!("Largest chain: {} sessions", largest_chain);
                            println!("Orphan sessions: {}", result.orphan_sessions);
                        }
                        "json" => {
                            #[derive(serde::Serialize)]
                            struct ChainOutput {
                                result: ChainBuildResult,
                                largest_chain: usize,
                                chains: std::collections::HashMap<
                                    String,
                                    tastematter::index::chain_graph::Chain,
                                >,
                            }
                            let output_data = ChainOutput {
                                result,
                                largest_chain,
                                chains,
                            };
                            println!("{}", serde_json::to_string_pretty(&output_data)?);
                        }
                        "compact" => {
                            #[derive(serde::Serialize)]
                            struct ChainOutput {
                                result: ChainBuildResult,
                                largest_chain: usize,
                                chains: std::collections::HashMap<
                                    String,
                                    tastematter::index::chain_graph::Chain,
                                >,
                            }
                            let output_data = ChainOutput {
                                result,
                                largest_chain,
                                chains,
                            };
                            println!("{}", serde_json::to_string(&output_data)?);
                        }
                        _ => {
                            println!("Chains built: {}", result.chains_built);
                            println!("Largest chain: {} sessions", largest_chain);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::IndexFiles {
            claude_dir,
            project,
            format,
            query,
        } => {
            use tastematter::capture::jsonl_parser::encode_project_path;

            // Determine Claude directory
            let claude_path = if let Some(dir) = claude_dir {
                PathBuf::from(dir)
            } else {
                dirs::home_dir()
                    .ok_or("Could not determine home directory")?
                    .join(".claude")
            };

            if !claude_path.exists() {
                eprintln!("Claude directory not found: {}", claude_path.display());
                std::process::exit(1);
            }

            // Determine the project directory to scan
            let projects_dir = if let Some(ref project_path) = project {
                let encoded = encode_project_path(std::path::Path::new(project_path));
                claude_path.join("projects").join(&encoded)
            } else {
                claude_path.join("projects")
            };

            if !projects_dir.exists() {
                eprintln!("Projects directory not found: {}", projects_dir.display());
                std::process::exit(1);
            }

            eprintln!("Building inverted index from: {}", projects_dir.display());

            let index = build_inverted_index(&projects_dir, None);

            let result = IndexBuildResult {
                accesses_indexed: index
                    .file_to_accesses
                    .values()
                    .map(|v| v.len() as i32)
                    .sum(),
                unique_files: index.file_to_accesses.len() as i32,
                unique_sessions: index.session_to_files.len() as i32,
            };

            // If query is provided, look up specific file
            if let Some(file_query) = query {
                let accesses = get_sessions_for_file(&index, &file_query);
                if accesses.is_empty() {
                    println!("No sessions found that touched '{}'", file_query);
                } else {
                    println!("Sessions that touched '{}':", file_query);
                    for access in accesses {
                        println!(
                            "  {} ({}) - {} [count: {}]",
                            access.session_id,
                            access.access_type,
                            access.timestamp,
                            access.access_count
                        );
                    }
                }
            } else {
                // Output summary or full index
                match format.as_str() {
                    "summary" => {
                        println!("Unique files: {}", result.unique_files);
                        println!("Unique sessions: {}", result.unique_sessions);
                        println!("Total accesses: {}", result.accesses_indexed);
                    }
                    "json" => {
                        println!("{}", serde_json::to_string_pretty(&result)?);
                    }
                    "compact" => {
                        println!("{}", serde_json::to_string(&result)?);
                    }
                    _ => {
                        println!(
                            "Indexed {} accesses across {} files from {} sessions",
                            result.accesses_indexed, result.unique_files, result.unique_sessions
                        );
                    }
                }
            }
        }
        Commands::Watch {
            path,
            debounce_ms,
            duration,
            recursive,
        } => {
            use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
            use std::sync::mpsc::channel;
            use std::sync::Arc;
            use std::time::{Duration, Instant};

            // Resolve path
            let watch_path = PathBuf::from(&path)
                .canonicalize()
                .unwrap_or_else(|_| PathBuf::from(&path));
            eprintln!("Watching {} for file changes...", watch_path.display());
            eprintln!("Debounce: {}ms, Recursive: {}", debounce_ms, recursive);
            if let Some(dur) = duration {
                eprintln!("Duration: {}s", dur);
            }
            eprintln!("Press Ctrl+C to stop");

            // Create filter and debouncer
            let filter = EventFilter::new(&watch_path.to_string_lossy());
            let debouncer = Arc::new(EventDebouncer::with_debounce(debounce_ms));
            let stats = Arc::new(std::sync::Mutex::new(WatcherStats::default()));

            // Set up notify watcher
            let (tx, rx) = channel();
            let mut watcher = RecommendedWatcher::new(
                move |res: Result<notify::Event, notify::Error>| {
                    if let Ok(event) = res {
                        let _ = tx.send(event);
                    }
                },
                Config::default(),
            )
            .map_err(|e| format!("Failed to create watcher: {}", e))?;

            // Start watching
            let mode = if recursive {
                RecursiveMode::Recursive
            } else {
                RecursiveMode::NonRecursive
            };
            watcher
                .watch(&watch_path, mode)
                .map_err(|e| format!("Failed to watch path: {}", e))?;

            // Main event loop
            let start_time = Instant::now();
            let timeout_duration = duration.map(Duration::from_secs);

            loop {
                // Check timeout
                if let Some(max_dur) = timeout_duration {
                    if start_time.elapsed() >= max_dur {
                        eprintln!("\nDuration limit reached. Exiting...");
                        break;
                    }
                }

                // Process events (with timeout for responsiveness)
                match rx.recv_timeout(Duration::from_millis(100)) {
                    Ok(event) => {
                        for path in event.paths {
                            let path_str = path.to_string_lossy().to_string();

                            // Check if should be ignored
                            if filter.should_ignore(&path_str) {
                                let mut s = stats.lock().unwrap();
                                s.events_filtered += 1;
                                continue;
                            }

                            // Determine event type
                            let event_type = match event.kind {
                                notify::EventKind::Create(_) => event_types::CREATE,
                                notify::EventKind::Modify(_) => event_types::WRITE,
                                notify::EventKind::Remove(_) => event_types::DELETE,
                                _ => continue,
                            };

                            // Create file event
                            if let Some(file_event) = create_event_from_path(
                                &path_str,
                                event_type,
                                &watch_path.to_string_lossy(),
                                None,
                            ) {
                                // Add to debouncer
                                debouncer.add(file_event.clone());

                                let mut s = stats.lock().unwrap();
                                s.events_captured += 1;

                                // Print event
                                println!(
                                    "[{}] {} {} ({})",
                                    file_event.timestamp.format("%H:%M:%S"),
                                    event_type.to_uppercase(),
                                    file_event.path,
                                    file_event
                                        .size_bytes
                                        .map_or("dir".to_string(), |s| format!("{}b", s))
                                );
                            }
                        }
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        // Flush debounced events periodically
                        let flushed = debouncer.flush();
                        if !flushed.is_empty() {
                            let mut s = stats.lock().unwrap();
                            s.events_debounced += flushed.len() as i64;
                        }
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                        eprintln!("Watcher disconnected");
                        break;
                    }
                }
            }

            // Print final stats
            let final_stats = stats.lock().unwrap();
            eprintln!("\n--- Watch Statistics ---");
            eprintln!("Events captured: {}", final_stats.events_captured);
            eprintln!("Events filtered: {}", final_stats.events_filtered);
            eprintln!("Events debounced: {}", final_stats.events_debounced);
        }
        Commands::Intel { intel_cmd } => {
            let client = IntelClient::default();

            match intel_cmd {
                IntelCommands::Health => {
                    // Check the health endpoint
                    let url = format!("{}/api/intel/health", client.base_url);
                    match client.health_check().await {
                        true => {
                            println!("Intel service: OK");
                            println!("URL: {}", url);
                        }
                        false => {
                            println!("Intel service: UNAVAILABLE");
                            println!("URL: {}", url);
                            println!("\nStart the service with: cd apps/tastematter/intel && bun run dev");
                            std::process::exit(1);
                        }
                    }
                }
                IntelCommands::NameChain {
                    chain_id,
                    files,
                    session_count,
                } => {
                    // Parse files from comma-separated string
                    let files_touched: Vec<String> = files
                        .map(|f| f.split(',').map(|s| s.trim().to_string()).collect())
                        .unwrap_or_default();

                    let request = ChainNamingRequest {
                        chain_id: chain_id.clone(),
                        files_touched,
                        session_count,
                        recent_sessions: vec![],
                        tools_used: None,
                        first_user_intent: None,
                        commit_messages: None,
                        first_user_message: None,
                        conversation_excerpt: None,
                    };

                    match client.name_chain(&request).await {
                        Ok(Some(response)) => {
                            result_count = Some(1);
                            println!("{}", serde_json::to_string_pretty(&response)?);
                        }
                        Ok(None) => {
                            eprintln!("Intel service unavailable or returned error.");
                            eprintln!(
                                "Start the service with: cd apps/tastematter/intel && bun run dev"
                            );
                            std::process::exit(1);
                        }
                        Err(e) => {
                            eprintln!("Error calling intel service: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
            }
        }
        Commands::Context {
            query,
            time,
            limit,
            format,
        } => {
            let input = ContextRestoreInput {
                query,
                time: Some(time),
                limit: Some(limit),
            };
            let ctx_result = engine.query_context(input).await?;
            result_count = Some(ctx_result.work_clusters.len() as u32);
            match format.as_str() {
                "table" => output_context_table(&ctx_result),
                _ => output(&ctx_result, &format)?,
            }
        }
    }

    // Capture telemetry event using typed helper (fire-and-forget)
    let mut event = CommandExecutedEvent::new(
        command_name,
        start_time.elapsed().as_millis() as u64,
        true, // success (errors exit earlier via process::exit)
    );

    // Enrich with result count if available
    if let Some(count) = result_count {
        event = event.with_result_count(count);
    }

    // Enrich with time range bucket if available
    if let Some(bucket) = time_range_bucket {
        event = event.with_time_range(bucket);
    }

    telemetry.capture_command(event);

    Ok(())
}

/// Output data in the specified format
fn output<T: serde::Serialize>(data: &T, format: &str) -> Result<(), Box<dyn std::error::Error>> {
    match format {
        "json" => println!("{}", serde_json::to_string_pretty(data)?),
        "compact" => println!("{}", serde_json::to_string(data)?),
        _ => println!("{}", serde_json::to_string_pretty(data)?),
    }
    Ok(())
}

/// Output heat results as a formatted table
fn output_heat_table(result: &tastematter::HeatResult) {
    // Header
    println!(
        "{:<60} {:>6} {:>6} {:>5} {:>7} {:>6} {:>4}",
        "FILE", "7D", "TOTAL", "RCR", "VEL", "SCORE", "HEAT"
    );
    println!("{}", "-".repeat(100));

    // Rows
    for item in &result.results {
        // Truncate long file paths
        let display_path = if item.file_path.len() > 58 {
            format!("..{}", &item.file_path[item.file_path.len() - 56..])
        } else {
            item.file_path.clone()
        };

        println!(
            "{:<60} {:>6} {:>6} {:>5.2} {:>7.2} {:>6.3} {:>4}",
            display_path,
            item.count_7d,
            item.count_long,
            item.rcr,
            item.velocity,
            item.heat_score,
            item.heat_level,
        );
    }

    // Summary
    println!("{}", "-".repeat(100));
    println!(
        "Total: {} files | HOT: {} | WARM: {} | COOL: {} | COLD: {} | Window: {}",
        result.summary.total_files,
        result.summary.hot_count,
        result.summary.warm_count,
        result.summary.cool_count,
        result.summary.cold_count,
        result.time_range,
    );
    println!("Receipt: {}", result.receipt_id);
}

/// Output chain results as a formatted table
fn output_chains_table(result: &tastematter::ChainQueryResult) {
    // Header
    println!("{:<40} {:>8} {:>8}  ID", "CHAIN", "SESSIONS", "FILES");
    println!("{}", "-".repeat(90));

    // Rows
    for chain in &result.chains {
        // Truncate display_name if needed
        let name = if chain.display_name.len() > 38 {
            format!("{}...", &chain.display_name[..35])
        } else {
            chain.display_name.clone()
        };

        // Show truncated chain_id for reference
        let short_id = if chain.chain_id.len() > 12 {
            &chain.chain_id[..12]
        } else {
            &chain.chain_id
        };

        println!(
            "{:<40} {:>8} {:>8}  {}",
            name, chain.session_count, chain.file_count, short_id,
        );
    }

    // Summary
    println!("{}", "-".repeat(90));
    println!("Total: {} chains", result.total_chains);
}

/// Output context restore results as a summary table
fn output_context_table(result: &tastematter::ContextRestoreResult) {
    // Executive summary
    println!("=== Context: {} ===", result.query);
    println!(
        "Status: {} | Tempo: {} | Generated: {}",
        result.executive_summary.status,
        result.executive_summary.work_tempo,
        &result.generated_at[..10],
    );
    if let Some(ref ts) = result.executive_summary.last_meaningful_session {
        println!("Last meaningful session: {}", &ts[..10.min(ts.len())]);
    }
    println!();

    // Work clusters
    if !result.work_clusters.is_empty() {
        println!("--- Work Clusters ({}) ---", result.work_clusters.len());
        for (i, cluster) in result.work_clusters.iter().enumerate() {
            println!(
                "  [{}] PMI={:.2} ({}) {} files",
                i + 1,
                cluster.pmi_score,
                cluster.access_pattern,
                cluster.files.len(),
            );
            for f in cluster.files.iter().take(3) {
                println!("      {}", f);
            }
        }
        println!();
    }

    // Suggested reads
    if !result.suggested_reads.is_empty() {
        println!("--- Suggested Reads ---");
        for read in result.suggested_reads.iter().take(10) {
            let surprise_marker = if read.surprise { " *SURPRISE*" } else { "" };
            println!("  [P{}] {}{}", read.priority, read.path, surprise_marker);
        }
        println!();
    }

    // Insights
    if !result.insights.is_empty() {
        println!("--- Insights ---");
        for insight in &result.insights {
            println!(
                "  [{}] {}: {}",
                insight.insight_type, insight.title, insight.description
            );
        }
        println!();
    }

    // Verification
    println!(
        "Receipt: {} | Files: {} | Sessions: {} | Co-access pairs: {}",
        result.verification.receipt_id,
        result.verification.files_analyzed,
        result.verification.sessions_analyzed,
        result.verification.co_access_pairs,
    );
}

/// Output heat results as CSV
fn output_heat_csv(result: &tastematter::HeatResult) {
    println!(
        "file_path,count_7d,count_long,rcr,velocity,heat_score,heat_level,first_access,last_access"
    );
    for item in &result.results {
        println!(
            "{},{},{},{:.4},{:.4},{:.4},{},{},{}",
            item.file_path,
            item.count_7d,
            item.count_long,
            item.rcr,
            item.velocity,
            item.heat_score,
            item.heat_level,
            item.first_access,
            item.last_access,
        );
    }
}
