//! Index layer: Chain graph, inverted index.
//!
//! This module ports the Python `index/` layer to Rust for single-binary distribution.

pub mod chain_graph;
pub mod inverted_index;

pub use chain_graph::*;
pub use inverted_index::*;
