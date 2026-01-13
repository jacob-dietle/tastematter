//! context-os CLI - Fast file access intelligence
//!
//! A standalone CLI for querying context-os data with <100ms latency.
//!
//! # Usage
//!
//! ```bash
//! # Query most accessed files in the last 7 days
//! context-os query flex --time 7d
//!
//! # Query chains
//! context-os query chains --limit 5
//!
//! # Query timeline data
//! context-os query timeline --time 14d
//!
//! # Query sessions
//! context-os query sessions --time 7d
//! ```

use clap::{Parser, Subcommand};
use context_os_core::{
    Database, QueryChainsInput, QueryEngine, QueryFlexInput, QuerySessionsInput,
    QueryTimelineInput,
};

#[derive(Parser)]
#[command(name = "context-os")]
#[command(version = "0.1.0")]
#[command(about = "Context OS - Fast file access intelligence", long_about = None)]
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
        },
        Commands::Serve { port, host, cors } => {
            use context_os_core::http::{create_router, AppState};
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
