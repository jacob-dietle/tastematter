//! E2E tests for the direct Anthropic API path (intel Rust port).
//!
//! All tests are `#[ignore]` — they require a real ANTHROPIC_API_KEY.
//! Run locally: ANTHROPIC_API_KEY=sk-ant-... cargo test --test intel_e2e -- --ignored --test-threads=2
//! Cost: ~3 Haiku calls ≈ $0.001

use tastematter::intelligence::{
    ClusterInput, ContextSynthesisRequest, IntelClient, SuggestedReadInput,
};

fn minimal_request() -> ContextSynthesisRequest {
    ContextSynthesisRequest {
        query: "test-project".to_string(),
        status: "healthy".to_string(),
        work_tempo: "active".to_string(),
        clusters: vec![ClusterInput {
            files: vec!["src/main.rs".to_string(), "src/lib.rs".to_string()],
            access_pattern: "high_access_high_session".to_string(),
            pmi_score: 2.5,
        }],
        suggested_reads: vec![SuggestedReadInput {
            path: "README.md".to_string(),
            priority: 1,
            surprise: false,
        }],
        context_package_content: None,
        key_metrics: None,
        evidence_sources: vec!["CLAUDE.md".to_string()],
    }
}

#[tokio::test]
#[ignore] // Requires ANTHROPIC_API_KEY env var
async fn e2e_direct_api_synthesize_context() {
    let client = IntelClient::from_env().expect("ANTHROPIC_API_KEY must be set to run this test");

    let request = minimal_request();
    let result = client.synthesize_context(&request).await;

    assert!(
        result.is_ok(),
        "synthesize_context returned Err: {:?}",
        result.err()
    );

    let response = result
        .unwrap()
        .expect("Expected Some(response), got None — API call may have failed silently");

    assert!(
        !response.one_liner.is_empty(),
        "one_liner should not be empty"
    );
    assert!(
        !response.narrative.is_empty(),
        "narrative should not be empty"
    );
    assert_eq!(
        response.cluster_names.len(),
        1,
        "Expected 1 cluster_name for 1 input cluster, got {}",
        response.cluster_names.len()
    );
    assert_eq!(
        response.suggested_read_reasons.len(),
        1,
        "Expected 1 suggested_read_reason for 1 input read, got {}",
        response.suggested_read_reasons.len()
    );
}

#[tokio::test]
#[ignore] // Tests graceful degradation with invalid key
async fn e2e_bad_api_key_returns_none() {
    // Construct client directly with a bad key (don't pollute env)
    let client = IntelClient::new_with_api_key("http://localhost:3002", "invalid-key-12345");

    let request = minimal_request();
    let result = client.synthesize_context(&request).await;

    assert!(
        result.is_ok(),
        "Bad API key should not return Err — graceful degradation"
    );
    assert!(
        result.unwrap().is_none(),
        "Bad API key should return Ok(None), not Ok(Some(...))"
    );
}

#[tokio::test]
#[ignore] // Requires ANTHROPIC_API_KEY env var
async fn e2e_empty_clusters_and_reads() {
    let client = IntelClient::from_env().expect("ANTHROPIC_API_KEY must be set to run this test");

    let request = ContextSynthesisRequest {
        query: "empty-test".to_string(),
        status: "healthy".to_string(),
        work_tempo: "quiet".to_string(),
        clusters: vec![],
        suggested_reads: vec![],
        context_package_content: None,
        key_metrics: None,
        evidence_sources: vec![],
    };

    let result = client.synthesize_context(&request).await;

    assert!(
        result.is_ok(),
        "Empty input should not return Err: {:?}",
        result.err()
    );

    let response = result
        .unwrap()
        .expect("Expected Some(response) even with empty input");

    // LLM should still produce a one_liner and narrative
    assert!(
        !response.one_liner.is_empty(),
        "one_liner should not be empty even with no clusters"
    );
    assert!(
        !response.narrative.is_empty(),
        "narrative should not be empty even with no clusters"
    );

    // With 0 input clusters/reads, arrays should be empty
    assert!(
        response.cluster_names.is_empty(),
        "Expected 0 cluster_names for 0 input clusters, got {}",
        response.cluster_names.len()
    );
    assert!(
        response.suggested_read_reasons.is_empty(),
        "Expected 0 suggested_read_reasons for 0 input reads, got {}",
        response.suggested_read_reasons.len()
    );
}
