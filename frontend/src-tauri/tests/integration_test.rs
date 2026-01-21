//! Integration tests for Tauri commands using context-os-core directly.
//!
//! Phase 2: TDD Tests - Replace CLI subprocess with library calls
//! Reference: specs/implementation/phase_02_tauri_integration/SPEC.md

// Tests import core library types to verify linking works

// =============================================================================
// Test 1: Core Library Imports Successfully
// =============================================================================

#[test]
fn test_core_library_available() {
    // Should be able to import types from core
    use context_os_core::QueryFlexInput;

    // Types should exist and be constructible
    let _input = QueryFlexInput {
        files: None,
        time: Some("7d".to_string()),
        chain: None,
        session: None,
        agg: vec!["count".to_string()],
        limit: Some(10),
        sort: None,
    };

    // If this compiles, the test passes
    // The core library is properly linked and types are accessible
}

// =============================================================================
// Test 2: AppState Provides QueryEngine
// =============================================================================

/// Test that AppState can lazily initialize and provide a QueryEngine.
/// This test uses app_lib::AppState directly.
#[tokio::test]
async fn test_app_state_provides_query_engine() {
    use app_lib::AppState;

    // Create AppState with lazy query engine
    let state = AppState::new_for_test();

    // Should be able to get engine (may fail if no DB, that's ok)
    let result = state.get_query_engine().await;

    // Either succeeds or returns proper error type (config error if no DB)
    match result {
        Ok(_engine) => {
            // Engine exists and we can call methods on it
            assert!(true, "QueryEngine initialized successfully");
        }
        Err(e) => {
            // Config error is expected if database doesn't exist
            // Other errors should fail the test
            let error_string = format!("{:?}", e);
            assert!(
                error_string.contains("Config") || error_string.contains("NotFound") || error_string.contains("database"),
                "Expected config/database error, got: {}", error_string
            );
        }
    }
}

// =============================================================================
// Test 3-6: Query Commands Use Core Library
// =============================================================================
// These tests verify that commands.rs uses context_os_core directly,
// not CLI subprocess. Static analysis test (Test 7) will verify no subprocess code.

/// Test 7: Verify no CLI subprocess code remains in commands.rs
/// This is a static analysis test that reads the source file
#[test]
fn test_no_cli_subprocess_code() {
    let commands_src = include_str!("../src/commands.rs");

    // These patterns indicate CLI subprocess usage that should NOT exist
    let forbidden_patterns = [
        "Command::new",           // std::process::Command
        "TASTEMATTER_CLI",        // Old CLI path env var
        "context-os.cmd",         // Old CLI name
        ".output()",              // Command output capture
        "cmd.args",               // Command argument building
    ];

    // Allow patterns in git commands (those are legitimate subprocess calls)
    let git_section_marker = "// Git commands";

    // Find where git commands section starts
    let git_section_start = commands_src.find(git_section_marker);

    // Only check the query commands section (before git commands)
    let query_section = match git_section_start {
        Some(pos) => &commands_src[..pos],
        None => commands_src,
    };

    for pattern in forbidden_patterns {
        // Skip checking Command::new since it's used legitimately for git
        if pattern == "Command::new" || pattern == ".output()" || pattern == "cmd.args" {
            // These are only forbidden in query_* functions, not git_* functions
            // Count occurrences in query section only
            let query_funcs = ["query_flex", "query_timeline", "query_sessions", "query_chains"];

            for func in query_funcs {
                if let Some(func_start) = query_section.find(&format!("pub async fn {}(", func)) {
                    // Find the function body (next closing brace at same level - simplified check)
                    let func_section = &query_section[func_start..];
                    if let Some(end) = func_section.find("\n}\n") {
                        let func_body = &func_section[..end];
                        assert!(
                            !func_body.contains(pattern),
                            "Found forbidden pattern '{}' in function '{}'. \
                             Query commands should use context_os_core directly, not subprocess.",
                            pattern, func
                        );
                    }
                }
            }
        } else {
            // For other patterns, check entire query section
            assert!(
                !query_section.contains(pattern),
                "Found forbidden pattern '{}' in query section. \
                 Query commands should use context_os_core directly, not CLI subprocess.",
                pattern
            );
        }
    }
}
