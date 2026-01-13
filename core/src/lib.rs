//! context-os-core: Unified query engine for context-os
//!
//! Provides direct SQLite queries replacing the Python CLI bottleneck.
//!
//! # Example
//!
//! ```ignore
//! use context_os_core::{Database, QueryEngine, QueryFlexInput};
//!
//! let db = Database::open("path/to/db.sqlite").await?;
//! let engine = QueryEngine::new(db);
//! let result = engine.query_flex(QueryFlexInput::default()).await?;
//! ```

pub mod error;
pub mod http;
pub mod query;
pub mod storage;
pub mod types;

pub use error::{CommandError, CoreError};
pub use query::QueryEngine;
pub use storage::Database;
pub use types::*;
