use super::{LogEvent, LogLevel, Component, ErrorInfo};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct LogService {
    log_dir: PathBuf,
    current_file: Mutex<Option<(String, std::fs::File)>>,
}

impl LogService {
    pub fn new() -> Self {
        // Use home directory or fallback to current dir
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."));

        let log_dir = home.join(".tastematter").join("logs");

        // Ensure log directory exists
        fs::create_dir_all(&log_dir).ok();

        Self {
            log_dir,
            current_file: Mutex::new(None),
        }
    }

    fn get_log_file(&self) -> std::io::Result<std::fs::File> {
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let filename = format!("dev-{}.jsonl", today);

        let mut guard = self.current_file.lock().unwrap();

        // Check if we need a new file (day changed)
        if let Some((date, _)) = guard.as_ref() {
            if date != &today {
                *guard = None;
            }
        }

        // Open or create file
        if guard.is_none() {
            let path = self.log_dir.join(&filename);
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)?;
            *guard = Some((today, file));
        }

        // Clone the file handle
        let (_, ref file) = guard.as_ref().unwrap();
        file.try_clone()
    }

    pub fn log(&self, event: LogEvent) {
        if let Ok(mut file) = self.get_log_file() {
            if let Ok(json) = serde_json::to_string(&event) {
                let _ = writeln!(file, "{}", json);
            }
        }
    }

    pub fn log_quick(
        &self,
        correlation_id: &str,
        component: Component,
        operation: &str,
        success: bool,
        duration_ms: Option<u64>,
        context: Option<serde_json::Value>,
        error: Option<ErrorInfo>,
    ) {
        self.log(LogEvent {
            correlation_id: correlation_id.to_string(),
            component,
            operation: operation.to_string(),
            success,
            duration_ms,
            context,
            error,
            level: if success { LogLevel::Info } else { LogLevel::Error },
            ..Default::default()
        });
    }
}

impl Default for LogService {
    fn default() -> Self {
        Self::new()
    }
}
