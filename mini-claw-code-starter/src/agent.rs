use crate::types::*;

/// Format a one-line summary of a tool call for terminal output.
fn tool_summary(call: &ToolCall) -> String {
    // Pick the most useful argument to display:
    // "command" for bash, "path" for read/write/edit.
    let detail = call
        .arguments
        .get("command")
        .or_else(|| call.arguments.get("path"))
        .and_then(|v| v.as_str());

    match detail {
        Some(s) => format!("    [{}: {}]", call.name, s),
        None => format!("    [{}]", call.name),
    }
}

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
///      b. If found, call it. Catch errors with `.unwrap_or_else(|e| format!("error: {e}"))`.
///      If not found, return error string. Never crash on tool failure.
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

    /// Run the agent loop, accumulating into the provided message history.
    ///
    /// # Chapter 7: The CLI
    ///
    /// This is `run()` adapted for multi-turn conversation:
    /// 1. The caller pushes `Message::User(…)` before calling
    /// 2. The loop is the same as `run()` — provider → match → tools → repeat
    /// 3. On `StopReason::Stop`, clone `turn.text` BEFORE pushing
    ///    `Message::Assistant(turn)` (the push moves `turn`)
    /// 4. Push the assistant turn into messages so the history is complete
    /// 5. Return the cloned text
    pub async fn chat(&self, _messages: &mut Vec<Message>) -> anyhow::Result<String> {
        unimplemented!(
            "Same loop as run(), but use the provided messages vec instead of creating a new one"
        )
    }
}
