//! Context synthesis agent — fills 5 Option<String> fields in context restore output
//!
//! Ported from: intel/src/agents/context-synthesis.ts (194 lines)
//! Pattern: system prompt + tool_choice → structured JSON output
//!
//! Fields filled:
//! - one_liner: <120 char project summary
//! - narrative: 2-4 sentence state description
//! - cluster_names: 2-4 word labels per cluster
//! - cluster_interpretations: what each cluster means
//! - suggested_read_reasons: why to read each file

use crate::intelligence::anthropic::{call_anthropic, AnthropicError, ToolDefinition};
use crate::intelligence::types::{ContextSynthesisRequest, ContextSynthesisResponse};
use reqwest::Client;

const MODEL: &str = "claude-haiku-4-5-20251001";
const MAX_TOKENS: u32 = 1024;

/// Build system prompt with expected array lengths.
/// Ported verbatim from TS: intel/src/agents/context-synthesis.ts:24-39
fn build_system_prompt(cluster_count: usize, read_count: usize) -> String {
    format!(
        r#"You are a context analyst for a developer's project. Given deterministic data about their recent work, synthesize human-readable summaries.

OUTPUT RULES:
- one_liner: Under 120 characters. Factual summary of project state. Example: "Nickel transcript worker is production-ready with 4 providers"
- narrative: 2-4 sentences. Ground every claim in the evidence provided. Start with what was built, then current state, then what's next.
- cluster_names: Exactly {} names, each 2-4 words. Describe what the file group does. Example: "Core Pipeline", "Type Contracts"
- cluster_interpretations: Exactly {} interpretations. One sentence each explaining why these files move together.
- suggested_read_reasons: Exactly {} reasons. One sentence each explaining why the developer should read this file to resume work.

GROUNDING RULES:
- Only reference files, metrics, and evidence provided in the input
- If evidence is thin, keep summaries brief rather than speculating
- Use developer-facing language, not marketing language

You MUST use the output_context_synthesis tool to provide your response."#,
        cluster_count, cluster_count, read_count
    )
}

/// Build user message from synthesis request.
/// Ported verbatim from TS: intel/src/agents/context-synthesis.ts:88-137
pub fn build_user_message(request: &ContextSynthesisRequest) -> String {
    // Numbered clusters
    let clusters_section = if request.clusters.is_empty() {
        "(no clusters)".to_string()
    } else {
        request
            .clusters
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let files_display: Vec<&str> = c.files.iter().take(8).map(|s| s.as_str()).collect();
                let overflow = if c.files.len() > 8 {
                    format!(" (+{} more)", c.files.len() - 8)
                } else {
                    String::new()
                };
                format!(
                    "Cluster {} ({}, PMI={:.2}):\n  Files: {}{}",
                    i + 1,
                    c.access_pattern,
                    c.pmi_score,
                    files_display.join(", "),
                    overflow
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    // Numbered reads
    let reads_section = if request.suggested_reads.is_empty() {
        "(no suggested reads)".to_string()
    } else {
        request
            .suggested_reads
            .iter()
            .enumerate()
            .map(|(i, r)| {
                let surprise = if r.surprise { ", surprise" } else { "" };
                format!(
                    "Read {}: {} (priority={}{})",
                    i + 1,
                    r.path,
                    r.priority,
                    surprise
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    // Context package content (truncated to 3000 chars)
    let context_section = match &request.context_package_content {
        Some(content) => {
            let truncated = if content.len() > 3000 {
                &content[..3000]
            } else {
                content
            };
            format!(
                "Context Package Content (most recent):\n\"\"\"\n{}\n\"\"\"\n\n",
                truncated
            )
        }
        None => String::new(),
    };

    // Evidence sources
    let evidence_section = if request.evidence_sources.is_empty() {
        "(none)".to_string()
    } else {
        request.evidence_sources.join(", ")
    };

    format!(
        r#"Synthesize context for the following project data. Use the output_context_synthesis tool.

QUERY: "{}"
STATUS: {}
WORK TEMPO: {}

WORK CLUSTERS ({} total):
{}

SUGGESTED READS ({} total):
{}

{}EVIDENCE SOURCES: {}

Analyze this data and use the output_context_synthesis tool."#,
        request.query,
        request.status,
        request.work_tempo,
        request.clusters.len(),
        clusters_section,
        request.suggested_reads.len(),
        reads_section,
        context_section,
        evidence_section
    )
}

/// Tool definition for structured output.
/// Matches TS: intel/src/agents/context-synthesis.ts:45-83
fn tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "output_context_synthesis".to_string(),
        description: "Output synthesized context for a developer's project".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "one_liner": {
                    "type": "string",
                    "description": "Under 120 character project state summary"
                },
                "narrative": {
                    "type": "string",
                    "description": "2-4 sentence description of current project state"
                },
                "cluster_names": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "2-4 word name for each work cluster (index-matched)"
                },
                "cluster_interpretations": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "One sentence interpretation per cluster (index-matched)"
                },
                "suggested_read_reasons": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "One sentence reason per suggested read (index-matched)"
                }
            },
            "required": [
                "one_liner",
                "narrative",
                "cluster_names",
                "cluster_interpretations",
                "suggested_read_reasons"
            ]
        }),
    }
}

/// Synthesize context by calling Anthropic API directly.
///
/// Replaces the HTTP call to localhost:3002/api/intel/synthesize-context.
/// Returns typed ContextSynthesisResponse or AnthropicError for graceful degradation.
pub async fn synthesize_context(
    http_client: &Client,
    api_key: &str,
    request: &ContextSynthesisRequest,
) -> Result<ContextSynthesisResponse, AnthropicError> {
    let system = build_system_prompt(request.clusters.len(), request.suggested_reads.len());
    let user_msg = build_user_message(request);
    let tool = tool_definition();

    let input = call_anthropic(http_client, api_key, MODEL, MAX_TOKENS, &system, &user_msg, &tool)
        .await?;

    // Parse tool input into response type
    let one_liner = input["one_liner"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let narrative = input["narrative"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let cluster_names = input["cluster_names"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let cluster_interpretations = input["cluster_interpretations"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let suggested_read_reasons = input["suggested_read_reasons"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    Ok(ContextSynthesisResponse {
        one_liner,
        narrative,
        cluster_names,
        cluster_interpretations,
        suggested_read_reasons,
        model_used: MODEL.to_string(),
    })
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intelligence::types::{ClusterInput, SuggestedReadInput};

    // ---- Prompt construction tests (TDD #4) ----

    #[test]
    fn system_prompt_includes_cluster_and_read_counts() {
        let prompt = build_system_prompt(3, 5);
        assert!(prompt.contains("Exactly 3 names"));
        assert!(prompt.contains("Exactly 3 interpretations"));
        assert!(prompt.contains("Exactly 5 reasons"));
        assert!(prompt.contains("output_context_synthesis"));
    }

    #[test]
    fn user_message_formats_clusters() {
        let request = ContextSynthesisRequest {
            query: "nickel".to_string(),
            status: "healthy".to_string(),
            work_tempo: "active".to_string(),
            clusters: vec![ClusterInput {
                files: vec!["src/auth.rs".to_string(), "src/main.rs".to_string()],
                access_pattern: "high_access_high_session".to_string(),
                pmi_score: 2.5,
            }],
            suggested_reads: vec![SuggestedReadInput {
                path: "specs/README.md".to_string(),
                priority: 1,
                surprise: false,
            }],
            context_package_content: None,
            key_metrics: None,
            evidence_sources: vec!["CLAUDE.md".to_string()],
        };

        let msg = build_user_message(&request);

        assert!(msg.contains("QUERY: \"nickel\""));
        assert!(msg.contains("STATUS: healthy"));
        assert!(msg.contains("WORK TEMPO: active"));
        assert!(msg.contains("Cluster 1 (high_access_high_session, PMI=2.50)"));
        assert!(msg.contains("src/auth.rs, src/main.rs"));
        assert!(msg.contains("Read 1: specs/README.md (priority=1)"));
        assert!(msg.contains("EVIDENCE SOURCES: CLAUDE.md"));
    }

    #[test]
    fn user_message_handles_empty_clusters_and_reads() {
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

        let msg = build_user_message(&request);

        assert!(msg.contains("(no clusters)"));
        assert!(msg.contains("(no suggested reads)"));
        assert!(msg.contains("EVIDENCE SOURCES: (none)"));
    }

    #[test]
    fn user_message_truncates_context_package() {
        let long_content = "x".repeat(5000);
        let request = ContextSynthesisRequest {
            query: "test".to_string(),
            status: "ok".to_string(),
            work_tempo: "active".to_string(),
            clusters: vec![],
            suggested_reads: vec![],
            context_package_content: Some(long_content.clone()),
            key_metrics: None,
            evidence_sources: vec![],
        };

        let msg = build_user_message(&request);

        // Should contain truncated content (3000 chars, not 5000)
        assert!(msg.contains("Context Package Content"));
        assert!(!msg.contains(&long_content)); // Not the full 5000 chars
    }

    #[test]
    fn user_message_marks_surprise_reads() {
        let request = ContextSynthesisRequest {
            query: "test".to_string(),
            status: "ok".to_string(),
            work_tempo: "active".to_string(),
            clusters: vec![],
            suggested_reads: vec![SuggestedReadInput {
                path: "surprise_file.rs".to_string(),
                priority: 2,
                surprise: true,
            }],
            context_package_content: None,
            key_metrics: None,
            evidence_sources: vec![],
        };

        let msg = build_user_message(&request);
        assert!(msg.contains("Read 1: surprise_file.rs (priority=2, surprise)"));
    }

    #[test]
    fn user_message_overflows_cluster_files() {
        let files: Vec<String> = (0..12).map(|i| format!("file_{}.rs", i)).collect();
        let request = ContextSynthesisRequest {
            query: "test".to_string(),
            status: "ok".to_string(),
            work_tempo: "active".to_string(),
            clusters: vec![ClusterInput {
                files,
                access_pattern: "pattern".to_string(),
                pmi_score: 1.0,
            }],
            suggested_reads: vec![],
            context_package_content: None,
            key_metrics: None,
            evidence_sources: vec![],
        };

        let msg = build_user_message(&request);
        assert!(msg.contains("(+4 more)"));
        assert!(msg.contains("file_7.rs")); // 8th file (0-indexed)
        assert!(!msg.contains("file_8.rs")); // 9th file should be hidden
    }

    // ---- Mock tool_use → response parsing tests (TDD #5) ----

    #[test]
    fn parse_tool_use_input_to_response() {
        let input = serde_json::json!({
            "one_liner": "Nickel transcript worker is production-ready",
            "narrative": "You built a multi-provider ingestion system. The worker processes transcripts from 4 providers.",
            "cluster_names": ["Core Pipeline", "Type Contracts"],
            "cluster_interpretations": ["Active development files", "Shared type definitions"],
            "suggested_read_reasons": ["Start here for architecture", "Check type contracts"]
        });

        let one_liner = input["one_liner"].as_str().unwrap().to_string();
        let narrative = input["narrative"].as_str().unwrap().to_string();
        let cluster_names: Vec<String> = input["cluster_names"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();
        let cluster_interpretations: Vec<String> = input["cluster_interpretations"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();
        let suggested_read_reasons: Vec<String> = input["suggested_read_reasons"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();

        assert_eq!(
            one_liner,
            "Nickel transcript worker is production-ready"
        );
        assert_eq!(cluster_names.len(), 2);
        assert_eq!(cluster_names[0], "Core Pipeline");
        assert_eq!(cluster_interpretations.len(), 2);
        assert_eq!(suggested_read_reasons.len(), 2);
        assert!(narrative.contains("multi-provider"));
    }

    #[test]
    fn parse_tool_use_handles_empty_arrays() {
        let input = serde_json::json!({
            "one_liner": "Empty project",
            "narrative": "No activity.",
            "cluster_names": [],
            "cluster_interpretations": [],
            "suggested_read_reasons": []
        });

        let cluster_names: Vec<String> = input["cluster_names"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();

        assert!(cluster_names.is_empty());
    }

    // ---- Tool definition tests ----

    #[test]
    fn tool_definition_matches_ts_schema() {
        let tool = tool_definition();
        assert_eq!(tool.name, "output_context_synthesis");

        let schema = &tool.input_schema;
        assert_eq!(schema["type"], "object");

        let required = schema["required"].as_array().unwrap();
        assert_eq!(required.len(), 5);
        assert!(required.contains(&serde_json::json!("one_liner")));
        assert!(required.contains(&serde_json::json!("narrative")));
        assert!(required.contains(&serde_json::json!("cluster_names")));
        assert!(required.contains(&serde_json::json!("cluster_interpretations")));
        assert!(required.contains(&serde_json::json!("suggested_read_reasons")));

        let props = &schema["properties"];
        assert_eq!(props["one_liner"]["type"], "string");
        assert_eq!(props["cluster_names"]["type"], "array");
        assert_eq!(props["cluster_names"]["items"]["type"], "string");
    }
}
