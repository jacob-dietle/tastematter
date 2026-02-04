//! Common test utilities for HTTP integration tests

use std::sync::Arc;
use std::time::Instant;
use tastematter::{
    http::{create_router, AppState},
    Database, QueryEngine,
};

/// Create a test router with real database connection
pub async fn create_test_router() -> axum::Router {
    let db = Database::open_default().await.unwrap();
    let engine = QueryEngine::new(db);
    let state = Arc::new(AppState {
        engine,
        start_time: Instant::now(),
    });
    create_router(state, true)
}
