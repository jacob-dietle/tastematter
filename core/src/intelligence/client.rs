//! Intelligence client — direct Anthropic API + sidecar fallback
//!
//! Primary path: ANTHROPIC_API_KEY → direct API calls via agents/*.rs
//! Fallback: localhost:3002 sidecar for methods not yet ported (name_chain, summarize_chain)
//!
//! Graceful degradation: returns `Ok(None)` on any failure, never errors on network.

use log::{info, warn};
use reqwest::Client;
use std::time::{Duration, Instant};
use uuid::Uuid;

use crate::error::CoreError;
use crate::intelligence::agents;
use crate::intelligence::types::{
    ChainNamingRequest, ChainNamingResponse, ChainSummaryRequest, ChainSummaryResponse,
    ContextSynthesisRequest, ContextSynthesisResponse,
};

/// Intelligence client with direct Anthropic API support.
///
/// Created via `from_env()` (requires ANTHROPIC_API_KEY) or `new()` (sidecar-only).
/// When `api_key` is set, `synthesize_context()` calls Anthropic directly.
/// Other methods (name_chain, summarize_chain) still use the sidecar until ported.
pub struct IntelClient {
    pub base_url: String,
    http_client: Client,
    api_key: Option<String>,
}

impl IntelClient {
    /// Create from ANTHROPIC_API_KEY (env var or config file). Returns None if not found.
    ///
    /// Checks in order:
    /// 1. `ANTHROPIC_API_KEY` environment variable (for CI and overrides)
    /// 2. `~/.context-os/config.yaml` `intelligence.api_key` field (set via `tastematter intel setup`)
    ///
    /// This is the primary constructor. Enables direct Anthropic API calls
    /// for ported agents, with sidecar fallback for unported methods.
    pub fn from_env() -> Option<Self> {
        // 1. Check env var first (CI, manual override)
        let api_key = std::env::var("ANTHROPIC_API_KEY").ok().or_else(|| {
            // 2. Fall back to config file
            crate::daemon::load_config(None)
                .ok()
                .and_then(|c| c.intelligence.api_key)
        })?;

        Some(Self {
            base_url: "http://localhost:3002".to_string(),
            http_client: Client::builder()
                .timeout(Duration::from_secs(15))
                .build()
                .expect("Failed to build HTTP client"),
            api_key: Some(api_key),
        })
    }

    /// Create with base URL for sidecar-only mode (backward compat / testing).
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            http_client: Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .expect("Failed to build HTTP client"),
            api_key: None,
        }
    }

    /// Create with explicit API key (for testing direct API with specific keys).
    pub fn new_with_api_key(base_url: &str, api_key: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            http_client: Client::builder()
                .timeout(Duration::from_secs(15))
                .build()
                .expect("Failed to build HTTP client"),
            api_key: Some(api_key.to_string()),
        }
    }

    /// Call chain naming endpoint with graceful degradation.
    ///
    /// Currently uses sidecar (Phase 2 will port to direct API).
    /// Returns `Ok(Some(response))` on success, `Ok(None)` on any failure.
    pub async fn name_chain(
        &self,
        request: &ChainNamingRequest,
    ) -> Result<Option<ChainNamingResponse>, CoreError> {
        let correlation_id = Uuid::new_v4().to_string();
        let start = Instant::now();
        let url = format!("{}/api/intel/name-chain", self.base_url);

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
                Ok(None)
            }
            Err(e) => {
                warn!(
                    target: "intelligence",
                    "Intelligence service unavailable - degrading gracefully: correlation_id={}, duration_ms={}, error={}",
                    correlation_id,
                    duration_ms,
                    e
                );
                Ok(None)
            }
        }
    }

    /// Call chain summary endpoint with graceful degradation.
    ///
    /// Currently uses sidecar (Phase 3 will port to direct API).
    /// Returns `Ok(Some(response))` on success, `Ok(None)` on any failure.
    pub async fn summarize_chain(
        &self,
        request: &ChainSummaryRequest,
    ) -> Result<Option<ChainSummaryResponse>, CoreError> {
        let correlation_id = Uuid::new_v4().to_string();
        let start = Instant::now();
        let url = format!("{}/api/intel/summarize-chain", self.base_url);

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
                Ok(None)
            }
            Err(e) => {
                warn!(
                    target: "intelligence",
                    "Intelligence service unavailable - degrading gracefully: correlation_id={}, duration_ms={}, error={}",
                    correlation_id,
                    duration_ms,
                    e
                );
                Ok(None)
            }
        }
    }

    /// Synthesize context for a project query.
    ///
    /// **Ported to direct API** — calls Anthropic directly when api_key is set.
    /// Falls back to sidecar HTTP call if no api_key (backward compat).
    /// Returns `Ok(Some(response))` on success, `Ok(None)` on any failure.
    pub async fn synthesize_context(
        &self,
        request: &ContextSynthesisRequest,
    ) -> Result<Option<ContextSynthesisResponse>, CoreError> {
        let correlation_id = Uuid::new_v4().to_string();
        let start = Instant::now();

        if let Some(api_key) = &self.api_key {
            // Direct Anthropic API call (ported path)
            info!(
                target: "intelligence",
                "Starting direct API synthesis: correlation_id={}, query={}, cluster_count={}, read_count={}",
                correlation_id,
                request.query,
                request.clusters.len(),
                request.suggested_reads.len()
            );

            match agents::context_synthesis::synthesize_context(&self.http_client, api_key, request)
                .await
            {
                Ok(data) => {
                    let duration_ms = start.elapsed().as_millis();
                    info!(
                        target: "intelligence",
                        "Direct API synthesis completed: correlation_id={}, duration_ms={}, one_liner_len={}",
                        correlation_id,
                        duration_ms,
                        data.one_liner.len()
                    );
                    Ok(Some(data))
                }
                Err(e) => {
                    let duration_ms = start.elapsed().as_millis();
                    warn!(
                        target: "intelligence",
                        "Direct API synthesis failed - degrading gracefully: correlation_id={}, duration_ms={}, error={}",
                        correlation_id,
                        duration_ms,
                        e
                    );
                    Ok(None)
                }
            }
        } else {
            // Sidecar fallback (pre-port path)
            self.synthesize_context_via_sidecar(request, &correlation_id, start)
                .await
        }
    }

    /// Sidecar path for synthesize_context (used when no API key)
    async fn synthesize_context_via_sidecar(
        &self,
        request: &ContextSynthesisRequest,
        correlation_id: &str,
        start: Instant,
    ) -> Result<Option<ContextSynthesisResponse>, CoreError> {
        let url = format!("{}/api/intel/synthesize-context", self.base_url);

        info!(
            target: "intelligence",
            "Starting sidecar synthesis: correlation_id={}, operation=synthesize_context, query={}",
            correlation_id,
            request.query
        );

        let result = self
            .http_client
            .post(&url)
            .header("X-Correlation-ID", correlation_id)
            .header("Content-Type", "application/json")
            .timeout(Duration::from_secs(15))
            .json(request)
            .send()
            .await;

        let duration_ms = start.elapsed().as_millis();

        match result {
            Ok(response) if response.status().is_success() => {
                match response.json::<ContextSynthesisResponse>().await {
                    Ok(data) => {
                        info!(
                            target: "intelligence",
                            "Sidecar synthesis completed: correlation_id={}, duration_ms={}, one_liner_len={}",
                            correlation_id,
                            duration_ms,
                            data.one_liner.len()
                        );
                        Ok(Some(data))
                    }
                    Err(e) => {
                        warn!(
                            target: "intelligence",
                            "Failed to parse sidecar response: correlation_id={}, duration_ms={}, error={}",
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
                    "Sidecar returned error: correlation_id={}, duration_ms={}, status={}",
                    correlation_id,
                    duration_ms,
                    response.status().as_u16()
                );
                Ok(None)
            }
            Err(e) => {
                warn!(
                    target: "intelligence",
                    "Sidecar unavailable - degrading gracefully: correlation_id={}, duration_ms={}, error={}",
                    correlation_id,
                    duration_ms,
                    e
                );
                Ok(None)
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

    /// Whether this client has a direct Anthropic API key
    pub fn has_api_key(&self) -> bool {
        self.api_key.is_some()
    }
}

impl Default for IntelClient {
    fn default() -> Self {
        Self::new("http://localhost:3002")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intelligence::types::{ClusterInput, SuggestedReadInput};

    #[test]
    fn intel_client_creates_with_base_url() {
        let client = IntelClient::new("http://localhost:3002");
        assert_eq!(client.base_url, "http://localhost:3002");
        assert!(client.api_key.is_none());
    }

    #[test]
    fn intel_client_default_uses_3002() {
        let client = IntelClient::default();
        assert_eq!(client.base_url, "http://localhost:3002");
        assert!(client.api_key.is_none());
    }

    #[test]
    fn from_env_returns_none_without_api_key() {
        // Remove env var if set (test isolation)
        std::env::remove_var("ANTHROPIC_API_KEY");
        let client = IntelClient::from_env();
        assert!(client.is_none());
    }

    #[test]
    fn from_env_returns_some_with_api_key() {
        std::env::set_var("ANTHROPIC_API_KEY", "test-key-123");
        let client = IntelClient::from_env();
        assert!(client.is_some());
        let client = client.unwrap();
        assert!(client.has_api_key());
        assert_eq!(client.api_key.as_deref(), Some("test-key-123"));
        // Clean up
        std::env::remove_var("ANTHROPIC_API_KEY");
    }

    #[test]
    fn has_api_key_reports_correctly() {
        let without = IntelClient::new("http://localhost:3002");
        assert!(!without.has_api_key());

        std::env::set_var("ANTHROPIC_API_KEY", "test-key");
        let with = IntelClient::from_env().unwrap();
        assert!(with.has_api_key());
        std::env::remove_var("ANTHROPIC_API_KEY");
    }

    #[tokio::test]
    async fn intel_client_returns_none_when_service_unavailable() {
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
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
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
        assert!(start.elapsed().as_secs() < 15);
    }

    #[tokio::test]
    async fn health_check_returns_false_when_unavailable() {
        let client = IntelClient::new("http://localhost:59999");
        let result = client.health_check().await;
        assert!(!result);
    }

    // ---- summarize_chain tests ----

    #[tokio::test]
    async fn summarize_chain_returns_none_when_service_unavailable() {
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
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn summarize_chain_handles_minimal_request() {
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
        assert!(start.elapsed().as_secs() < 15);
    }

    // ---- synthesize_context tests ----

    #[tokio::test]
    async fn synthesize_context_returns_none_when_service_unavailable() {
        let client = IntelClient::new("http://localhost:59999");
        let request = ContextSynthesisRequest {
            query: "test".to_string(),
            status: "unknown".to_string(),
            work_tempo: "dormant".to_string(),
            clusters: vec![],
            suggested_reads: vec![],
            context_package_content: None,
            key_metrics: None,
            evidence_sources: vec![],
        };
        let result = client.synthesize_context(&request).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn synthesize_context_handles_full_request() {
        let client = IntelClient::new("http://localhost:59999");
        let request = ContextSynthesisRequest {
            query: "nickel".to_string(),
            status: "healthy".to_string(),
            work_tempo: "active".to_string(),
            clusters: vec![ClusterInput {
                files: vec!["src/auth.rs".to_string()],
                access_pattern: "high_access_high_session".to_string(),
                pmi_score: 2.5,
            }],
            suggested_reads: vec![SuggestedReadInput {
                path: "specs/README.md".to_string(),
                priority: 1,
                surprise: false,
            }],
            context_package_content: Some("# Package 35".to_string()),
            key_metrics: Some(serde_json::json!({"files": 20})),
            evidence_sources: vec!["CLAUDE.md".to_string()],
        };
        let result = client.synthesize_context(&request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn synthesize_context_has_timeout() {
        let client = IntelClient::new("http://localhost:59999");
        let start = std::time::Instant::now();
        let request = ContextSynthesisRequest {
            query: "timeout-test".to_string(),
            status: "unknown".to_string(),
            work_tempo: "dormant".to_string(),
            clusters: vec![],
            suggested_reads: vec![],
            context_package_content: None,
            key_metrics: None,
            evidence_sources: vec![],
        };
        let _ = client.synthesize_context(&request).await;
        assert!(start.elapsed().as_secs() < 20);
    }
}
