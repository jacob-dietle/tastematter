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
use tastematter::{
    capture::file_watcher::{
        create_event_from_path, event_types, EventDebouncer, EventFilter, FileEvent,
        WatcherConfig, WatcherStats,
    },
    capture::git_sync::{sync_commits, SyncOptions, SyncResult},
    capture::jsonl_parser::{sync_sessions, ParseOptions, ParseResult, SessionSummary},
    daemon::{load_config, run_sync, DaemonConfig, DaemonState},
    index::chain_graph::{build_chain_graph, ChainBuildResult},
    index::inverted_index::{build_inverted_index, get_sessions_for_file, IndexBuildResult},
    Database, QueryChainsInput, QueryCoAccessInput, QueryEngine, QueryFileInput,
    QueryFlexInput, QueryReceiptsInput, QuerySearchInput, QuerySessionsInput,
    QueryTimelineInput, QueryVerifyInput,
};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "tastematter")]
#[command(version = "0.1.0")]
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
    /// Show daemon status
    Status,
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Open database (from explicit path or auto-discover)
    let db = if let Some(ref path) = cli.db {
        Database::open(path).await?
    } else {
        Database::open_default().await?
    };

    let engine = QueryEngine::new(db);

    match cli.command {
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
                let result = engine.query_flex(input).await?;
                output(&result, &format)?;
            }
            QueryCommands::Chains { limit, format } => {
                let input = QueryChainsInput { limit: Some(limit) };
                let result = engine.query_chains(input).await?;
                output(&result, &format)?;
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
                let result = engine.query_timeline(input).await?;
                output(&result, &format)?;
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
                let result = engine.query_sessions(input).await?;
                output(&result, &format)?;
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
                let result = engine.query_search(input).await?;
                output(&result, &format)?;
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
                let result = engine.query_file(input).await?;
                output(&result, &format)?;
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
                let result = engine.query_co_access(input).await?;
                output(&result, &format)?;
            }
            QueryCommands::Verify { receipt_id, format } => {
                let input = QueryVerifyInput { receipt_id };
                let result = engine.query_verify(input).await?;
                output(&result, &format)?;
            }
            QueryCommands::Receipts { limit, format } => {
                let input = QueryReceiptsInput {
                    limit: Some(limit),
                };
                let result = engine.query_receipts(input).await?;
                output(&result, &format)?;
            }
        },
        Commands::Serve { port, host, cors } => {
            use tastematter::http::{create_router, AppState};
            use std::sync::Arc;
            use std::time::Instant;

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
                    let sessions_linked: u32 = chains
                        .values()
                        .map(|c| c.sessions.len() as u32)
                        .sum();
                    let largest_chain = chains
                        .values()
                        .map(|c| c.sessions.len())
                        .max()
                        .unwrap_or(0);
                    let orphan_sessions = chains
                        .values()
                        .filter(|c| c.sessions.len() == 1)
                        .count() as u32;

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
                                chains: std::collections::HashMap<String, tastematter::index::chain_graph::Chain>,
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
                                chains: std::collections::HashMap<String, tastematter::index::chain_graph::Chain>,
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
                            access.session_id, access.access_type, access.timestamp, access.access_count
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
                        println!("Indexed {} accesses across {} files from {} sessions",
                            result.accesses_indexed, result.unique_files, result.unique_sessions);
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
            let watch_path = PathBuf::from(&path).canonicalize().unwrap_or_else(|_| PathBuf::from(&path));
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
            ).map_err(|e| format!("Failed to create watcher: {}", e))?;

            // Start watching
            let mode = if recursive {
                RecursiveMode::Recursive
            } else {
                RecursiveMode::NonRecursive
            };
            watcher.watch(&watch_path, mode)
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
                                    file_event.size_bytes.map_or("dir".to_string(), |s| format!("{}b", s))
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
        Commands::Daemon { daemon_cmd } => {
            match daemon_cmd {
                DaemonCommands::Once { project } => {
                    eprintln!("Running single sync cycle...");

                    // Load or create config
                    let mut config = load_config(None).map_err(|e| format!("Config error: {}", e))?;

                    // Apply project filter if provided
                    if let Some(p) = project {
                        config.project.path = Some(p);
                    }

                    // Run sync
                    let result = run_sync(&config).map_err(|e| format!("Sync error: {}", e))?;

                    // Output results
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
                }
                DaemonCommands::Start { interval, project } => {
                    eprintln!("Starting daemon (interval: {}min)...", interval);
                    eprintln!("Press Ctrl+C to stop");

                    // Load or create config
                    let mut config = load_config(None).map_err(|e| format!("Config error: {}", e))?;
                    config.sync.interval_minutes = interval;

                    // Apply project filter if provided
                    if let Some(p) = project {
                        config.project.path = Some(p);
                    }

                    // Load state
                    let state_path = dirs::home_dir()
                        .ok_or("Could not find home directory")?
                        .join(".context-os")
                        .join("daemon.state.json");
                    let mut state = DaemonState::load_or_default(&state_path);
                    state.started_at = Some(chrono::Utc::now());

                    // Run initial sync
                    eprintln!("Running initial sync...");
                    let result = run_sync(&config).map_err(|e| format!("Sync error: {}", e))?;
                    state.git_commits_synced += result.git_commits_synced as i64;
                    state.sessions_parsed += result.sessions_parsed as i64;
                    state.chains_built += result.chains_built as i64;
                    state.last_git_sync = Some(chrono::Utc::now());
                    state.last_session_parse = Some(chrono::Utc::now());
                    state.last_chain_build = Some(chrono::Utc::now());
                    let _ = state.save(&state_path);

                    eprintln!(
                        "Initial sync: {} commits, {} sessions, {} chains",
                        result.git_commits_synced, result.sessions_parsed, result.chains_built
                    );

                    // Daemon loop
                    let interval_duration = std::time::Duration::from_secs(interval as u64 * 60);
                    loop {
                        eprintln!("Next sync in {} minutes...", interval);
                        std::thread::sleep(interval_duration);

                        eprintln!("Running sync...");
                        match run_sync(&config) {
                            Ok(result) => {
                                state.git_commits_synced += result.git_commits_synced as i64;
                                state.sessions_parsed += result.sessions_parsed as i64;
                                state.chains_built += result.chains_built as i64;
                                state.last_git_sync = Some(chrono::Utc::now());
                                state.last_session_parse = Some(chrono::Utc::now());
                                state.last_chain_build = Some(chrono::Utc::now());
                                let _ = state.save(&state_path);

                                eprintln!(
                                    "Sync complete: {} commits, {} sessions, {} chains",
                                    result.git_commits_synced, result.sessions_parsed, result.chains_built
                                );
                            }
                            Err(e) => {
                                eprintln!("Sync error: {}", e);
                            }
                        }
                    }
                }
                DaemonCommands::Status => {
                    // Load state
                    let state_path = dirs::home_dir()
                        .ok_or("Could not find home directory")?
                        .join(".context-os")
                        .join("daemon.state.json");

                    if !state_path.exists() {
                        println!("Status: not running (no state file)");
                        println!("Run 'tastematter daemon once' or 'tastematter daemon start' to begin syncing.");
                    } else {
                        let state = DaemonState::load_or_default(&state_path);
                        println!("=== Daemon Status ===");
                        if let Some(started) = state.started_at {
                            println!("Started: {}", started.format("%Y-%m-%d %H:%M:%S UTC"));
                        }
                        if let Some(last_sync) = state.last_git_sync {
                            println!("Last git sync: {}", last_sync.format("%Y-%m-%d %H:%M:%S UTC"));
                        }
                        if let Some(last_parse) = state.last_session_parse {
                            println!("Last session parse: {}", last_parse.format("%Y-%m-%d %H:%M:%S UTC"));
                        }
                        println!("\n=== Cumulative Stats ===");
                        println!("Git commits synced: {}", state.git_commits_synced);
                        println!("Sessions parsed: {}", state.sessions_parsed);
                        println!("Chains built: {}", state.chains_built);
                        println!("File events captured: {}", state.file_events_captured);
                    }
                }
            }
        }
    }

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
