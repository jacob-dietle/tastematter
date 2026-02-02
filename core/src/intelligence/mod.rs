//! Intelligence service integration module
//!
//! Provides integration with the TypeScript intelligence service at localhost:3002
//! for AI-powered chain naming, commit analysis, and session insights.
//!
//! # Architecture
//!
//! - `IntelClient`: HTTP client for calling intelligence service endpoints
//! - `MetadataStore`: SQLite cache for storing intelligence responses
//! - Types: `ChainNamingRequest`, `ChainNamingResponse`, `ChainMetadata`, etc.
//!
//! # Graceful Degradation
//!
//! All client methods return `Ok(None)` when the service is unavailable,
//! ensuring the application continues to function without AI features.
//!
//! # Example
//!
//! ```ignore
//! use tastematter::intelligence::{IntelClient, MetadataStore, ChainNamingRequest};
//!
//! // Create client
//! let client = IntelClient::default();
//!
//! // Call service (graceful degradation on failure)
//! let request = ChainNamingRequest {
//!     chain_id: "abc123".to_string(),
//!     files_touched: vec!["src/main.rs".to_string()],
//!     session_count: 5,
//!     recent_sessions: vec![],
//! };
//!
//! if let Some(response) = client.name_chain(&request).await? {
//!     // Cache the result
//!     let store = MetadataStore::new(Path::new("cache.db")).await?;
//!     store.cache_chain_name(&response).await?;
//! }
//! ```

mod cache;
mod client;
mod types;

pub use cache::MetadataStore;
pub use client::IntelClient;
pub use types::*;
