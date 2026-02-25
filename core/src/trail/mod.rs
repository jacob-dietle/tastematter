//! Global Trail: sync local trail data to a remote D1 database.
//!
//! Enables multi-machine context by pushing local SQLite trail data
//! to a Cloudflare D1 database. Any machine can pull and query the
//! full trail history.
//!
//! Trail push is opt-in: if `~/.context-os/trail.toml` doesn't exist,
//! push is silently skipped.

pub mod config;
pub mod paths;
pub mod pull;
pub mod push;
