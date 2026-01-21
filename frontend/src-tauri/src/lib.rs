mod commands;
mod logging;

use logging::LogService;
use std::sync::Arc;
use tokio::sync::OnceCell;
use context_os_core::{Database, QueryEngine, CoreError};

pub struct AppState {
    pub log_service: Arc<LogService>,
    pub query_engine: Arc<OnceCell<QueryEngine>>,
}

impl AppState {
    /// Create a new AppState for the Tauri app
    pub fn new(log_service: Arc<LogService>) -> Self {
        Self {
            log_service,
            query_engine: Arc::new(OnceCell::new()),
        }
    }

    /// Create AppState for testing (without log service dependency)
    pub fn new_for_test() -> Self {
        Self {
            log_service: Arc::new(LogService::new()),
            query_engine: Arc::new(OnceCell::new()),
        }
    }

    /// Get or initialize the QueryEngine lazily
    ///
    /// Uses the canonical database path (~/.context-os/context_os_events.db)
    /// Initialization happens once on first call, subsequent calls return cached engine
    pub async fn get_query_engine(&self) -> Result<&QueryEngine, CoreError> {
        self.query_engine.get_or_try_init(|| async {
            let db_path = Database::canonical_path()?;
            let db = Database::open(&db_path).await?;
            Ok(QueryEngine::new(db))
        }).await
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  let log_service = Arc::new(LogService::new());

  tauri::Builder::default()
    .manage(AppState::new(log_service.clone()))
    .setup(|app| {
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .targets([
              tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stdout),
              tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::LogDir {
                file_name: Some("rust".into())
              }),
            ])
            .build(),
        )?;
      }
      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
      commands::query_flex,
      commands::query_timeline,
      commands::query_sessions,
      commands::query_chains,
      commands::git_status,
      commands::git_pull,
      commands::git_push,
      commands::log_event
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
