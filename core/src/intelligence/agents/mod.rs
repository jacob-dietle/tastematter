//! Agent modules — each agent is a system prompt + tool schema + caller.
//!
//! All agents follow the same pattern:
//! 1. Build system prompt (string)
//! 2. Build user message (from request data)
//! 3. Define tool schema (JSON)
//! 4. Call `call_anthropic()` → get tool_use input
//! 5. Deserialize into typed response

pub mod context_synthesis;
