//! Error types for context-os-core

use serde::Serialize;
use thiserror::Error;

/// Error type for context-os-core operations
/// Must convert to CommandError format for Tauri compatibility
#[derive(Error, Debug)]
pub enum CoreError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Query error: {message}")]
    Query { message: String },

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Tauri-compatible error format
/// This is what gets sent to the frontend
#[derive(Debug, Serialize)]
pub struct CommandError {
    pub code: String,
    pub message: String,
    pub details: Option<String>,
}

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
