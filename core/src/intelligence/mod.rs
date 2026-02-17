//! Intelligence layer — embedded LLM agents + metadata cache
//!
//! # Architecture
//!
//! - `IntelClient`: Calls Anthropic API directly (if ANTHROPIC_API_KEY set)
//!   or falls back to TypeScript sidecar at localhost:3002 for unported methods.
//! - `anthropic`: Generic `call_anthropic()` function for all agent calls.
//! - `agents`: Per-agent modules (prompt + tool schema + caller).
//! - `MetadataStore`: SQLite cache for storing intelligence responses.
//! - Types: `ChainNamingRequest`, `ChainNamingResponse`, `ChainMetadata`, etc.
//!
//! # Graceful Degradation
//!
//! All client methods return `Ok(None)` when unavailable:
//! - No API key → `IntelClient::from_env()` returns `None`
//! - API error (401, 429, 500) → `Ok(None)`
//! - Network timeout → `Ok(None)`
//! - Malformed response → `Ok(None)`
//!
//! # Example
//!
//! ```ignore
//! use tastematter::intelligence::IntelClient;
//!
//! // Create from env (None if ANTHROPIC_API_KEY not set)
//! if let Some(client) = IntelClient::from_env() {
//!     let engine = QueryEngine::new(db).with_intel(client);
//! }
//! ```

pub mod agents;
pub mod anthropic;
mod cache;
mod client;
mod types;

pub use cache::MetadataStore;
pub use client::IntelClient;
pub use types::*;
