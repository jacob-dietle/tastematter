//! Type Contracts for Phase 2: Tauri Integration
//!
//! These types define the integration contract between context-os-core and Tastematter.
//! CRITICAL: The output types MUST serialize to the EXACT same JSON as the current
//! commands.rs to avoid breaking the frontend.
//!
//! Phase 2 primarily integrates Phase 1 types, so most contracts are about:
//! 1. AppState structure for dependency injection
//! 2. Command signature changes (adding State parameter)
//! 3. Error type conversions

use context_os_core::{
    CoreError, QueryEngine, Database,
    QueryFlexInput, QueryTimelineInput, QuerySessionsInput, QueryChainsInput,
    QueryResult, TimelineData, SessionQueryResult, ChainQueryResult,
};
use std::sync::Arc;
use tokio::sync::OnceCell;

// =============================================================================
// APP STATE CONTRACT
// =============================================================================

/// Application state shared across all Tauri commands
///
/// This struct is managed by Tauri and injected into commands via State<'_, AppState>
///
/// CRITICAL: Must be Send + Sync for Tauri's async runtime
pub struct AppState {
    /// Logging service (existing from Phase 0)
    pub log_service: Arc<LogService>,

    /// Query engine - lazily initialized on first use
    /// Using OnceCell to avoid blocking app startup
    pub query_engine: Arc<OnceCell<QueryEngine>>,
}

impl AppState {
    /// Create new AppState with uninitialized query engine
    pub fn new(log_service: Arc<LogService>) -> Self {
        Self {
            log_service,
            query_engine: Arc::new(OnceCell::new()),
        }
    }

    /// Get or initialize the query engine
    ///
    /// Returns error if database cannot be found or opened
    ///
    /// # Errors
    /// - CoreError::Config if database path not found
    /// - CoreError::Database if connection fails
    pub async fn get_query_engine(&self) -> Result<&QueryEngine, CoreError> {
        self.query_engine.get_or_try_init(|| async {
            let db_path = find_database_path()?;
            let db = Database::open(&db_path).await?;
            Ok(QueryEngine::new(db))
        }).await
    }
}

/// Find database path from standard locations
///
/// Search order:
/// 1. ~/.context-os/context.db (primary)
/// 2. Environment variable CONTEXT_OS_DB (override)
fn find_database_path() -> Result<std::path::PathBuf, CoreError> {
    // Check environment override first
    if let Ok(path) = std::env::var("CONTEXT_OS_DB") {
        let path = std::path::PathBuf::from(path);
        if path.exists() {
            return Ok(path);
        }
    }

    // Standard location
    if let Some(home) = dirs::home_dir() {
        let path = home.join(".context-os/context.db");
        if path.exists() {
            return Ok(path);
        }
    }

    Err(CoreError::Config(
        "Database not found. Ensure context-os daemon is running and database exists at ~/.context-os/context.db".into()
    ))
}

// =============================================================================
// COMMAND SIGNATURE CONTRACTS
// =============================================================================

/// Command signature for query_flex
///
/// Note: `state` parameter is added - Tauri injects this automatically
/// All other parameters remain identical to preserve API compatibility
///
/// BEFORE:
/// ```rust,ignore
/// pub async fn query_flex(
///     files: Option<String>,
///     // ... other params
/// ) -> Result<QueryResult, CommandError>
/// ```
///
/// AFTER:
/// ```rust,ignore
/// pub async fn query_flex(
///     state: State<'_, AppState>,  // NEW: injected by Tauri
///     files: Option<String>,
///     // ... other params (unchanged)
/// ) -> Result<QueryResult, CommandError>
/// ```
pub trait QueryFlexCommand {
    async fn query_flex(
        state: tauri::State<'_, AppState>,
        files: Option<String>,
        time: Option<String>,
        chain: Option<String>,
        session: Option<String>,
        agg: Vec<String>,
        limit: Option<u32>,
        sort: Option<String>,
    ) -> Result<QueryResult, CommandError>;
}

/// Command signature for query_timeline
pub trait QueryTimelineCommand {
    async fn query_timeline(
        state: tauri::State<'_, AppState>,
        time: String,
        files: Option<String>,
        limit: Option<u32>,
    ) -> Result<TimelineData, CommandError>;
}

/// Command signature for query_sessions
pub trait QuerySessionsCommand {
    async fn query_sessions(
        state: tauri::State<'_, AppState>,
        time: String,
        chain: Option<String>,
        limit: Option<u32>,
    ) -> Result<SessionQueryResult, CommandError>;
}

/// Command signature for query_chains
pub trait QueryChainsCommand {
    async fn query_chains(
        state: tauri::State<'_, AppState>,
        limit: Option<u32>,
    ) -> Result<ChainQueryResult, CommandError>;
}

// =============================================================================
// ERROR CONVERSION CONTRACT
// =============================================================================

/// CommandError - the error type returned to frontend
///
/// MUST match existing commands.rs:58-67 exactly
#[derive(Debug, serde::Serialize)]
pub struct CommandError {
    pub code: String,
    pub message: String,
    pub details: Option<String>,
}

/// Convert CoreError to CommandError
///
/// Error code mapping:
/// - CoreError::Database -> "DATABASE_ERROR"
/// - CoreError::Query -> "QUERY_ERROR"
/// - CoreError::Config -> "CONFIG_ERROR"
/// - CoreError::Serialization -> "SERIALIZATION_ERROR"
///
/// NEW error code for Phase 2:
/// - Engine initialization failure -> "ENGINE_ERROR"
impl From<CoreError> for CommandError {
    fn from(err: CoreError) -> Self {
        match err {
            CoreError::Database(e) => CommandError {
                code: "DATABASE_ERROR".to_string(),
                message: "Database operation failed".to_string(),
                details: Some(e.to_string()),
            },
            CoreError::Query { message } => CommandError {
                code: "QUERY_ERROR".to_string(),
                message,
                details: None,
            },
            CoreError::Config(msg) => CommandError {
                code: "CONFIG_ERROR".to_string(),
                message: msg,
                details: None,
            },
            CoreError::Serialization(e) => CommandError {
                code: "SERIALIZATION_ERROR".to_string(),
                message: "Failed to serialize result".to_string(),
                details: Some(e.to_string()),
            },
        }
    }
}

/// Helper to create ENGINE_ERROR
/// Used when query engine initialization fails
pub fn engine_error(err: CoreError) -> CommandError {
    CommandError {
        code: "ENGINE_ERROR".to_string(),
        message: "Failed to initialize query engine".to_string(),
        details: Some(err.to_string()),
    }
}

// =============================================================================
// REMOVED CONTRACTS (Code to Delete)
// =============================================================================

/// The following patterns should be REMOVED from commands.rs:
///
/// 1. CLI Path Resolution (DELETE)
/// ```rust,ignore
/// // DELETE THIS:
/// let cli_path = std::env::var("CONTEXT_OS_CLI")
///     .unwrap_or_else(|_| "C:/Users/dietl/.context-os/bin/context-os.cmd".to_string());
/// ```
///
/// 2. Command Building (DELETE)
/// ```rust,ignore
/// // DELETE THIS:
/// let mut cmd = Command::new(&cli_path);
/// cmd.current_dir("../../..");
/// cmd.args(["query", "flex", "--format", "json"]);
/// // ... all argument building
/// ```
///
/// 3. CLI Error Codes (DELETE)
/// ```rust,ignore
/// // DELETE THESE ERROR CODES:
/// code: "CLI_NOT_FOUND"
/// code: "CLI_ERROR"
/// code: "UTF8_ERROR"
/// ```
///
/// 4. JSON Parsing of CLI Output (DELETE)
/// ```rust,ignore
/// // DELETE THIS:
/// let json_str = String::from_utf8(output.stdout)?;
/// let result: QueryResult = serde_json::from_str(&json_str)?;
/// ```

// =============================================================================
// TAURI SETUP CONTRACT
// =============================================================================

/// Tauri Builder setup pattern
///
/// This shows how to register AppState in lib.rs
///
/// ```rust,ignore
/// pub fn run() {
///     let log_service = Arc::new(LogService::new());
///
///     tauri::Builder::default()
///         .manage(AppState::new(log_service.clone()))
///         .invoke_handler(tauri::generate_handler![
///             commands::query_flex,
///             commands::query_timeline,
///             commands::query_sessions,
///             commands::query_chains,
///             // ... other commands unchanged
///         ])
///         .run(tauri::generate_context!())
///         .expect("error while running tauri application");
/// }
/// ```

// =============================================================================
// CARGO.TOML CONTRACT
// =============================================================================

/// Required dependencies to add to Cargo.toml
///
/// ```toml
/// [dependencies]
/// # Link to context-os-core library
/// context-os-core = { path = "../../context-os-core" }
///
/// # For OnceCell (lazy initialization)
/// tokio = { version = "1.40", features = ["sync"] }
///
/// # For home directory detection
/// dirs = "5.0"
/// ```

// =============================================================================
// VERIFICATION
// =============================================================================

/// Placeholder for LogService (imported from existing code)
pub struct LogService;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_error_serialization() {
        let err = CommandError {
            code: "DATABASE_ERROR".to_string(),
            message: "Connection failed".to_string(),
            details: Some("timeout".to_string()),
        };

        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("DATABASE_ERROR"));
        assert!(json.contains("Connection failed"));
        assert!(json.contains("timeout"));
    }

    #[test]
    fn test_engine_error_helper() {
        let core_err = CoreError::Config("Test error".to_string());
        let cmd_err = engine_error(core_err);

        assert_eq!(cmd_err.code, "ENGINE_ERROR");
        assert!(cmd_err.message.contains("query engine"));
    }

    #[test]
    fn test_error_code_mapping() {
        // Database error
        let db_err = CoreError::Database(sqlx::Error::RowNotFound);
        let cmd_err: CommandError = db_err.into();
        assert_eq!(cmd_err.code, "DATABASE_ERROR");

        // Query error
        let query_err = CoreError::Query { message: "Invalid filter".to_string() };
        let cmd_err: CommandError = query_err.into();
        assert_eq!(cmd_err.code, "QUERY_ERROR");

        // Config error
        let config_err = CoreError::Config("Not found".to_string());
        let cmd_err: CommandError = config_err.into();
        assert_eq!(cmd_err.code, "CONFIG_ERROR");
    }
}
