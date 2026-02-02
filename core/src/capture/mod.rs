//! Capture layer: Git sync, JSONL parsing, file watching.
//!
//! This module ports the Python `capture/` layer to Rust for single-binary distribution.

pub mod file_watcher;
pub mod git_status;
pub mod git_sync;
pub mod jsonl_parser;

pub use file_watcher::*;
pub use git_status::*;
pub use git_sync::*;
pub use jsonl_parser::*;
