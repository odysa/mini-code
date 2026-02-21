use crate::types::*;

/// Handle a single prompt with at most one round of tool calls.
///
/// # Chapter 3: Single Turn
///
/// Steps:
/// 1. Collect tool definitions with `tools.definitions()`
/// 2. Create messages starting with `vec![Message::User(prompt.to_string())]`
/// 3. Call `provider.chat(&messages, &defs).await?`
/// 4. Match on `turn.stop_reason`:
///    - `StopReason::Stop` → return `turn.text.unwrap_or_default()`
///    - `StopReason::ToolUse` → for each tool call:
///      a. Look up tool with `tools.get(&call.name)`
///      b. If found, call it. If not, return error string (don't crash).
///      c. Collect results BEFORE pushing `Message::Assistant(turn)` (ownership!)
///      d. Push `Message::Assistant(turn)` then `Message::ToolResult` for each result
///      e. Call provider again to get the final answer
pub async fn single_turn<P: Provider>(
    provider: &P,
    tools: &ToolSet,
    prompt: &str,
) -> anyhow::Result<String> {
    unimplemented!(
        "Send prompt to provider, match on stop_reason, execute tools if needed, return final text"
    )
}

/// A simple AI agent that connects a provider to tools via a loop.
///
/// # Chapter 5: The Agent Loop
///
/// The agent loop is just `single_turn()` wrapped in a loop:
/// 1. Send the user's prompt to the provider
/// 2. Match on stop_reason
/// 3. If Stop → return text
/// 4. If ToolUse → execute tools, feed results back, continue the loop
pub struct SimpleAgent<P: Provider> {
    provider: P,
    tools: ToolSet,
}

impl<P: Provider> SimpleAgent<P> {
    /// Create a new agent with the given provider and no tools.
    pub fn new(_provider: P) -> Self {
        unimplemented!("Initialize provider and an empty ToolSet")
    }

    /// Register a tool with the agent. Returns self for chaining (builder pattern).
    pub fn tool(mut self, _t: impl Tool + 'static) -> Self {
        unimplemented!("Push the tool into self.tools, return self")
    }

    /// Run the agent loop with the given prompt.
    pub async fn run(&self, _prompt: &str) -> anyhow::Result<String> {
        unimplemented!(
            "Loop: send messages to provider, match on stop_reason, execute tool calls, repeat until Stop"
        )
    }
}
