//! HTTP client for intelligence service integration
//!
//! Provides graceful degradation - returns `Ok(None)` when service is unavailable,
//! never errors on network failures.

use log::{info, warn};
use reqwest::Client;
use std::time::{Duration, Instant};
use uuid::Uuid;

use crate::error::CoreError;
use crate::intelligence::types::{
    ChainNamingRequest, ChainNamingResponse, ChainSummaryRequest, ChainSummaryResponse,
};

/// HTTP client for intelligence service at localhost:3002
pub struct IntelClient {
    pub base_url: String,
    http_client: Client,
}

impl Default for IntelClient {
    fn default() -> Self {
        Self::new("http://localhost:3002")
    }
}

impl IntelClient {
    /// Create new client with specified base URL
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            http_client: Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .expect("Failed to build HTTP client"),
        }
    }

    /// Call chain naming endpoint with graceful degradation
    ///
    /// Returns `Ok(Some(response))` on success, `Ok(None)` on any failure.
    /// Never returns `Err` for network or service failures - graceful degradation.
    pub async fn name_chain(
        &self,
        request: &ChainNamingRequest,
    ) -> Result<Option<ChainNamingResponse>, CoreError> {
        let correlation_id = Uuid::new_v4().to_string();
        let start = Instant::now();
        let url = format!("{}/api/intel/name-chain", self.base_url);

        // OBSERVABILITY: Log request start
        info!(
            target: "intelligence",
            "Starting intelligence request: correlation_id={}, operation=name_chain, chain_id={}, files_count={}",
            correlation_id,
            request.chain_id,
            request.files_touched.len()
        );

        let result = self
            .http_client
            .post(&url)
            .header("X-Correlation-ID", &correlation_id)
            .header("Content-Type", "application/json")
            .json(request)
            .send()
            .await;

        let duration_ms = start.elapsed().as_millis();

        match result {
            Ok(response) if response.status().is_success() => {
                match response.json::<ChainNamingResponse>().await {
                    Ok(data) => {
                        info!(
                            target: "intelligence",
                            "Intelligence request completed: correlation_id={}, duration_ms={}, success=true, generated_name={}",
                            correlation_id,
                            duration_ms,
                            data.generated_name
                        );
                        Ok(Some(data))
                    }
                    Err(e) => {
                        warn!(
                            target: "intelligence",
                            "Failed to parse intelligence response: correlation_id={}, duration_ms={}, error={}",
                            correlation_id,
                            duration_ms,
                            e
                        );
                        Ok(None)
                    }
                }
            }
            Ok(response) => {
                warn!(
                    target: "intelligence",
                    "Intelligence service returned error status: correlation_id={}, duration_ms={}, status={}",
                    correlation_id,
                    duration_ms,
                    response.status().as_u16()
                );
                Ok(None) // Graceful degradation
            }
            Err(e) => {
                warn!(
                    target: "intelligence",
                    "Intelligence service unavailable - degrading gracefully: correlation_id={}, duration_ms={}, error={}",
                    correlation_id,
                    duration_ms,
                    e
                );
                Ok(None) // Graceful degradation - don't error, just return None
            }
        }
    }

    /// Call chain summary endpoint with graceful degradation
    ///
    /// Returns `Ok(Some(response))` on success, `Ok(None)` on any failure.
    /// Never returns `Err` for network or service failures - graceful degradation.
    pub async fn summarize_chain(
        &self,
        request: &ChainSummaryRequest,
    ) -> Result<Option<ChainSummaryResponse>, CoreError> {
        let correlation_id = Uuid::new_v4().to_string();
        let start = Instant::now();
        let url = format!("{}/api/intel/summarize-chain", self.base_url);

        // OBSERVABILITY: Log request start
        info!(
            target: "intelligence",
            "Starting intelligence request: correlation_id={}, operation=summarize_chain, chain_id={}, session_count={}",
            correlation_id,
            request.chain_id,
            request.session_count
        );

        let result = self
            .http_client
            .post(&url)
            .header("X-Correlation-ID", &correlation_id)
            .header("Content-Type", "application/json")
            .json(request)
            .send()
            .await;

        let duration_ms = start.elapsed().as_millis();

        match result {
            Ok(response) if response.status().is_success() => {
                match response.json::<ChainSummaryResponse>().await {
                    Ok(data) => {
                        info!(
                            target: "intelligence",
                            "Intelligence request completed: correlation_id={}, duration_ms={}, success=true, status={:?}",
                            correlation_id,
                            duration_ms,
                            data.status
                        );
                        Ok(Some(data))
                    }
                    Err(e) => {
                        warn!(
                            target: "intelligence",
                            "Failed to parse intelligence response: correlation_id={}, duration_ms={}, error={}",
                            correlation_id,
                            duration_ms,
                            e
                        );
                        Ok(None)
                    }
                }
            }
            Ok(response) => {
                warn!(
                    target: "intelligence",
                    "Intelligence service returned error status: correlation_id={}, duration_ms={}, status={}",
                    correlation_id,
                    duration_ms,
                    response.status().as_u16()
                );
                Ok(None) // Graceful degradation
            }
            Err(e) => {
                warn!(
                    target: "intelligence",
                    "Intelligence service unavailable - degrading gracefully: correlation_id={}, duration_ms={}, error={}",
                    correlation_id,
                    duration_ms,
                    e
                );
                Ok(None) // Graceful degradation - don't error, just return None
            }
        }
    }

    /// Check if the intelligence service is available
    pub async fn health_check(&self) -> bool {
        let url = format!("{}/api/intel/health", self.base_url);
        match self.http_client.get(&url).send().await {
            Ok(response) => response.status().is_success(),
            Err(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intel_client_creates_with_base_url() {
        let client = IntelClient::new("http://localhost:3002");
        assert_eq!(client.base_url, "http://localhost:3002");
    }

    #[test]
    fn intel_client_default_uses_3002() {
        let client = IntelClient::default();
        assert_eq!(client.base_url, "http://localhost:3002");
    }

    #[tokio::test]
    async fn intel_client_returns_none_when_service_unavailable() {
        // Use port that's definitely not running
        let client = IntelClient::new("http://localhost:59999");
        let request = ChainNamingRequest {
            chain_id: "test".to_string(),
            files_touched: vec![],
            session_count: 1,
            recent_sessions: vec![],
            tools_used: None,
            first_user_intent: None,
            commit_messages: None,
            first_user_message: None,
            conversation_excerpt: None,
        };
        let result = client.name_chain(&request).await;
        assert!(result.is_ok()); // Doesn't error
        assert!(result.unwrap().is_none()); // Returns None
    }

    #[tokio::test]
    async fn intel_client_has_timeout() {
        let client = IntelClient::new("http://localhost:59999");
        let start = std::time::Instant::now();
        let request = ChainNamingRequest {
            chain_id: "test".to_string(),
            files_touched: vec![],
            session_count: 1,
            recent_sessions: vec![],
            tools_used: None,
            first_user_intent: None,
            commit_messages: None,
            first_user_message: None,
            conversation_excerpt: None,
        };
        let _ = client.name_chain(&request).await;
        // Should timeout within 15 seconds (default + buffer)
        assert!(start.elapsed().as_secs() < 15);
    }

    #[tokio::test]
    async fn health_check_returns_false_when_unavailable() {
        let client = IntelClient::new("http://localhost:59999");
        let result = client.health_check().await;
        assert!(!result);
    }

    // =========================================================================
    // summarize_chain() tests (Phase 2 - Chain Summary)
    // =========================================================================

    #[tokio::test]
    async fn summarize_chain_returns_none_when_service_unavailable() {
        // Use port that's definitely not running
        let client = IntelClient::new("http://localhost:59999");
        let request = ChainSummaryRequest {
            chain_id: "test-chain".to_string(),
            conversation_excerpt: Some("Help me fix auth...".to_string()),
            files_touched: vec!["src/auth.rs".to_string()],
            session_count: 3,
            duration_seconds: Some(1800),
            existing_workstreams: Some(vec!["pixee".to_string()]),
        };
        let result = client.summarize_chain(&request).await;
        assert!(result.is_ok()); // Doesn't error - graceful degradation
        assert!(result.unwrap().is_none()); // Returns None when unavailable
    }

    #[tokio::test]
    async fn summarize_chain_handles_minimal_request() {
        // Test with only required fields (optional fields None)
        let client = IntelClient::new("http://localhost:59999");
        let request = ChainSummaryRequest {
            chain_id: "minimal".to_string(),
            conversation_excerpt: None,
            files_touched: vec![],
            session_count: 1,
            duration_seconds: None,
            existing_workstreams: None,
        };
        let result = client.summarize_chain(&request).await;
        // Should complete without panicking
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn summarize_chain_has_timeout() {
        let client = IntelClient::new("http://localhost:59999");
        let start = std::time::Instant::now();
        let request = ChainSummaryRequest {
            chain_id: "timeout-test".to_string(),
            conversation_excerpt: None,
            files_touched: vec![],
            session_count: 1,
            duration_seconds: None,
            existing_workstreams: None,
        };
        let _ = client.summarize_chain(&request).await;
        // Should timeout within 15 seconds (default + buffer)
        assert!(start.elapsed().as_secs() < 15);
    }
}
