mod commands;
mod logging;

use logging::LogService;
use std::sync::Arc;

pub struct AppState {
    pub log_service: Arc<LogService>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  let log_service = Arc::new(LogService::new());

  tauri::Builder::default()
    .manage(AppState {
        log_service: log_service.clone(),
    })
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
