//! HTTP API server for context-os-core
//!
//! Exposes QueryEngine over HTTP for browser-based development.
//! NOT for production use - binds to localhost only.
//!
//! # Usage
//!
//! ```bash
//! context-os serve --port 3001 --cors
//! ```
//!
//! # Endpoints
//!
//! - `GET /api/health` - Health check
//! - `POST /api/query/flex` - Flexible file query
//! - `POST /api/query/timeline` - Timeline data query
//! - `POST /api/query/sessions` - Session-grouped query
//! - `POST /api/query/chains` - Chain metadata query

use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use tower_http::cors::{Any, CorsLayer};

use crate::{
    ChainQueryResult, ContextRestoreInput, ContextRestoreResult, CoreError, QueryChainsInput,
    QueryEngine, QueryFlexInput, QueryResult, QuerySessionsInput, QueryTimelineInput,
    SessionQueryResult, TimelineData,
};

/// Application state shared across handlers
pub struct AppState {
    pub engine: QueryEngine,
    pub start_time: Instant,
}

/// Health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: String,
    pub version: String,
    pub database: String,
    pub uptime_seconds: u64,
}

/// Error response format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub error: String,
    pub message: String,
}

impl From<CoreError> for (StatusCode, Json<ApiError>) {
    fn from(err: CoreError) -> Self {
        let api_error = ApiError {
            error: "QueryError".to_string(),
            message: err.to_string(),
        };
        (StatusCode::BAD_REQUEST, Json(api_error))
    }
}

/// Create the HTTP router with all query endpoints
pub fn create_router(state: Arc<AppState>, enable_cors: bool) -> Router {
    let mut router = Router::new()
        .route("/api/health", get(health_handler))
        .route("/api/query/flex", post(query_flex_handler))
        .route("/api/query/timeline", post(query_timeline_handler))
        .route("/api/query/sessions", post(query_sessions_handler))
        .route("/api/query/chains", post(query_chains_handler))
        .route("/api/query/context", post(query_context_handler))
        .with_state(state);

    if enable_cors {
        router = router.layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        );
    }

    router
}

/// Health check endpoint
async fn health_handler(State(state): State<Arc<AppState>>) -> Json<HealthStatus> {
    Json(HealthStatus {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        database: "connected".to_string(),
        uptime_seconds: state.start_time.elapsed().as_secs(),
    })
}

/// Flexible query endpoint - POST /api/query/flex
async fn query_flex_handler(
    State(state): State<Arc<AppState>>,
    Json(input): Json<QueryFlexInput>,
) -> Result<Json<QueryResult>, (StatusCode, Json<ApiError>)> {
    state
        .engine
        .query_flex(input)
        .await
        .map(Json)
        .map_err(Into::into)
}

/// Timeline query endpoint - POST /api/query/timeline
async fn query_timeline_handler(
    State(state): State<Arc<AppState>>,
    Json(input): Json<QueryTimelineInput>,
) -> Result<Json<TimelineData>, (StatusCode, Json<ApiError>)> {
    state
        .engine
        .query_timeline(input)
        .await
        .map(Json)
        .map_err(Into::into)
}

/// Sessions query endpoint - POST /api/query/sessions
async fn query_sessions_handler(
    State(state): State<Arc<AppState>>,
    Json(input): Json<QuerySessionsInput>,
) -> Result<Json<SessionQueryResult>, (StatusCode, Json<ApiError>)> {
    state
        .engine
        .query_sessions(input)
        .await
        .map(Json)
        .map_err(Into::into)
}

/// Chains query endpoint - POST /api/query/chains
async fn query_chains_handler(
    State(state): State<Arc<AppState>>,
    Json(input): Json<QueryChainsInput>,
) -> Result<Json<ChainQueryResult>, (StatusCode, Json<ApiError>)> {
    state
        .engine
        .query_chains(input)
        .await
        .map(Json)
        .map_err(Into::into)
}

/// Context restore endpoint - POST /api/query/context
async fn query_context_handler(
    State(state): State<Arc<AppState>>,
    Json(input): Json<ContextRestoreInput>,
) -> Result<Json<ContextRestoreResult>, (StatusCode, Json<ApiError>)> {
    state
        .engine
        .query_context(input)
        .await
        .map(Json)
        .map_err(Into::into)
}
