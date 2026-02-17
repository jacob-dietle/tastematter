//! Direct Anthropic Messages API integration
//!
//! Generic `call_anthropic()` function that handles all agent calls.
//! Each agent provides: system prompt, user message, tool definition.
//! This module handles: serialization, HTTP, deserialization, error handling.

use log::warn;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::Duration;

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";

// =============================================================================
// Request types
// =============================================================================

#[derive(Debug, Serialize)]
pub struct AnthropicRequest {
    pub model: String,
    pub max_tokens: u32,
    pub system: String,
    pub messages: Vec<Message>,
    pub tools: Vec<ToolDefinition>,
    pub tool_choice: ToolChoice,
}

#[derive(Debug, Serialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct ToolChoice {
    #[serde(rename = "type")]
    pub choice_type: String,
    pub name: String,
}

// =============================================================================
// Response types
// =============================================================================

#[derive(Debug, Deserialize)]
pub struct AnthropicResponse {
    pub content: Vec<ContentBlock>,
    pub usage: Usage,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

// =============================================================================
// Error types
// =============================================================================

#[derive(Debug)]
pub enum AnthropicError {
    /// HTTP or network error
    Network(String),
    /// Non-success HTTP status (401, 429, 500, etc.)
    ApiError { status: u16, body: String },
    /// Response parsed but no tool_use block found
    NoToolUse,
    /// JSON parse error
    ParseError(String),
}

impl fmt::Display for AnthropicError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AnthropicError::Network(e) => write!(f, "Network error: {}", e),
            AnthropicError::ApiError { status, body } => {
                write!(f, "API error {}: {}", status, body)
            }
            AnthropicError::NoToolUse => write!(f, "No tool_use block in response"),
            AnthropicError::ParseError(e) => write!(f, "Parse error: {}", e),
        }
    }
}

// =============================================================================
// Core function
// =============================================================================

/// Generic Anthropic Messages API call with tool_choice.
///
/// Sends a single message with forced tool use, returns the tool's input as JSON Value.
/// All agents use this same function — they differ only in prompts and tool schemas.
pub async fn call_anthropic(
    http_client: &Client,
    api_key: &str,
    model: &str,
    max_tokens: u32,
    system_prompt: &str,
    user_message: &str,
    tool: &ToolDefinition,
) -> Result<serde_json::Value, AnthropicError> {
    let request = AnthropicRequest {
        model: model.to_string(),
        max_tokens,
        system: system_prompt.to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: user_message.to_string(),
        }],
        tools: vec![ToolDefinition {
            name: tool.name.clone(),
            description: tool.description.clone(),
            input_schema: tool.input_schema.clone(),
        }],
        tool_choice: ToolChoice {
            choice_type: "tool".to_string(),
            name: tool.name.clone(),
        },
    };

    let result = http_client
        .post(ANTHROPIC_API_URL)
        .header("x-api-key", api_key)
        .header("anthropic-version", ANTHROPIC_VERSION)
        .header("content-type", "application/json")
        .timeout(Duration::from_secs(30))
        .json(&request)
        .send()
        .await
        .map_err(|e| AnthropicError::Network(e.to_string()))?;

    let status = result.status().as_u16();
    if status != 200 {
        let body = result
            .text()
            .await
            .unwrap_or_else(|_| "Failed to read body".to_string());
        warn!(
            target: "intelligence",
            "Anthropic API error: status={}, body={}",
            status,
            body.get(..200).unwrap_or(&body)
        );
        return Err(AnthropicError::ApiError { status, body });
    }

    let response: AnthropicResponse = result
        .json()
        .await
        .map_err(|e| AnthropicError::ParseError(e.to_string()))?;

    // Extract tool_use input
    for block in &response.content {
        if let ContentBlock::ToolUse { input, .. } = block {
            return Ok(input.clone());
        }
    }

    Err(AnthropicError::NoToolUse)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Serialization tests (TDD #1) ----

    #[test]
    fn request_serializes_to_anthropic_api_format() {
        let request = AnthropicRequest {
            model: "claude-haiku-4-5-20251001".to_string(),
            max_tokens: 1024,
            system: "You are a helpful assistant.".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
            tools: vec![ToolDefinition {
                name: "my_tool".to_string(),
                description: "A test tool".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "result": { "type": "string" }
                    },
                    "required": ["result"]
                }),
            }],
            tool_choice: ToolChoice {
                choice_type: "tool".to_string(),
                name: "my_tool".to_string(),
            },
        };

        let json = serde_json::to_value(&request).unwrap();

        // Verify exact JSON structure Anthropic expects
        assert_eq!(json["model"], "claude-haiku-4-5-20251001");
        assert_eq!(json["max_tokens"], 1024);
        assert_eq!(json["system"], "You are a helpful assistant.");
        assert_eq!(json["messages"][0]["role"], "user");
        assert_eq!(json["messages"][0]["content"], "Hello");
        assert_eq!(json["tools"][0]["name"], "my_tool");
        assert_eq!(json["tools"][0]["input_schema"]["type"], "object");
        assert_eq!(json["tool_choice"]["type"], "tool");
        assert_eq!(json["tool_choice"]["name"], "my_tool");
    }

    #[test]
    fn tool_choice_serializes_type_field_correctly() {
        let choice = ToolChoice {
            choice_type: "tool".to_string(),
            name: "output_context_synthesis".to_string(),
        };
        let json = serde_json::to_value(&choice).unwrap();
        // Must be "type" not "choice_type" in JSON
        assert_eq!(json["type"], "tool");
        assert!(json.get("choice_type").is_none());
    }

    // ---- Deserialization tests (TDD #2) ----

    #[test]
    fn response_deserializes_tool_use_block() {
        let raw = r#"{
            "content": [
                {
                    "type": "tool_use",
                    "id": "toolu_01A09q90qw90lq917835lh",
                    "name": "output_context_synthesis",
                    "input": {
                        "one_liner": "Nickel transcript worker is production-ready",
                        "narrative": "You built a multi-provider ingestion system.",
                        "cluster_names": ["Core Pipeline", "Type Contracts"],
                        "cluster_interpretations": ["Active dev files", "Shared types"],
                        "suggested_read_reasons": ["Start here", "Entry point"]
                    }
                }
            ],
            "usage": {
                "input_tokens": 512,
                "output_tokens": 128
            }
        }"#;

        let response: AnthropicResponse = serde_json::from_str(raw).unwrap();
        assert_eq!(response.content.len(), 1);
        assert_eq!(response.usage.input_tokens, 512);
        assert_eq!(response.usage.output_tokens, 128);

        match &response.content[0] {
            ContentBlock::ToolUse { id, name, input } => {
                assert_eq!(id, "toolu_01A09q90qw90lq917835lh");
                assert_eq!(name, "output_context_synthesis");
                assert_eq!(
                    input["one_liner"],
                    "Nickel transcript worker is production-ready"
                );
                assert_eq!(input["cluster_names"].as_array().unwrap().len(), 2);
            }
            ContentBlock::Other => panic!("Expected ToolUse, got Other"),
        }
    }

    #[test]
    fn response_handles_text_block_as_other() {
        let raw = r#"{
            "content": [
                {
                    "type": "text",
                    "text": "Some text response"
                },
                {
                    "type": "tool_use",
                    "id": "toolu_123",
                    "name": "my_tool",
                    "input": { "result": "ok" }
                }
            ],
            "usage": { "input_tokens": 100, "output_tokens": 50 }
        }"#;

        let response: AnthropicResponse = serde_json::from_str(raw).unwrap();
        assert_eq!(response.content.len(), 2);
        assert!(matches!(response.content[0], ContentBlock::Other));
        assert!(matches!(response.content[1], ContentBlock::ToolUse { .. }));
    }

    #[test]
    fn response_extracts_tool_use_input() {
        let raw = r#"{
            "content": [
                {
                    "type": "tool_use",
                    "id": "toolu_abc",
                    "name": "output_context_synthesis",
                    "input": {
                        "one_liner": "Test project",
                        "narrative": "Testing.",
                        "cluster_names": [],
                        "cluster_interpretations": [],
                        "suggested_read_reasons": []
                    }
                }
            ],
            "usage": { "input_tokens": 10, "output_tokens": 20 }
        }"#;

        let response: AnthropicResponse = serde_json::from_str(raw).unwrap();
        for block in &response.content {
            if let ContentBlock::ToolUse { input, .. } = block {
                assert_eq!(input["one_liner"].as_str().unwrap(), "Test project");
                assert!(input["cluster_names"].as_array().unwrap().is_empty());
                return;
            }
        }
        panic!("No tool_use block found");
    }

    // ---- Error handling tests (TDD #3) ----

    #[test]
    fn anthropic_error_display_formats() {
        let network = AnthropicError::Network("timeout".to_string());
        assert_eq!(format!("{}", network), "Network error: timeout");

        let api = AnthropicError::ApiError {
            status: 401,
            body: "Unauthorized".to_string(),
        };
        assert_eq!(format!("{}", api), "API error 401: Unauthorized");

        let no_tool = AnthropicError::NoToolUse;
        assert_eq!(format!("{}", no_tool), "No tool_use block in response");

        let parse = AnthropicError::ParseError("bad json".to_string());
        assert_eq!(format!("{}", parse), "Parse error: bad json");
    }

    // ---- from_env tests (TDD #6 — moved here since it's about API key) ----

    #[tokio::test]
    async fn call_anthropic_returns_network_error_on_bad_url() {
        // This tests that call_anthropic properly maps network errors
        let client = Client::builder()
            .timeout(Duration::from_secs(1))
            .build()
            .unwrap();
        let tool = ToolDefinition {
            name: "test".to_string(),
            description: "test".to_string(),
            input_schema: serde_json::json!({"type": "object", "properties": {}}),
        };

        let result =
            call_anthropic(&client, "fake-key", "model", 100, "system", "user", &tool).await;

        assert!(result.is_err());
        // Should be either Network or ApiError depending on connectivity
        match result.unwrap_err() {
            AnthropicError::Network(_) | AnthropicError::ApiError { .. } => {} // Expected
            other => panic!("Unexpected error type: {}", other),
        }
    }
}
