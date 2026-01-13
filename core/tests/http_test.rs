//! HTTP API tests - written FIRST per TDD methodology
//!
//! These tests validate the HTTP API endpoints expose the same
//! functionality as the CLI and Tauri IPC interfaces.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

mod common;

#[tokio::test]
async fn test_health_endpoint_returns_200() {
    // RED: This test will fail until http module is implemented
    let app = common::create_test_router().await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_query_flex_returns_data() {
    let app = common::create_test_router().await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/query/flex")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"time": "7d", "limit": 5}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_query_chains_returns_data() {
    let app = common::create_test_router().await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/query/chains")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"limit": 10}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_query_timeline_returns_data() {
    let app = common::create_test_router().await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/query/timeline")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"time": "7d"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_query_sessions_returns_data() {
    let app = common::create_test_router().await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/query/sessions")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"time": "7d", "limit": 20}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
