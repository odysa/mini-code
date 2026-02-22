use crate::types::*;
use tokio::sync::mpsc;

/// Events emitted by the agent during execution.
#[derive(Debug)]
pub enum AgentEvent {
    /// A chunk of text streamed from the LLM (streaming mode only).
    TextDelta(String),
    /// A tool is being called.
    ToolCall { name: String, summary: String },
    /// The agent finished with a final response.
    Done(String),
    /// The agent encountered an error.
    Error(String),
}

/// Format a one-line summary of a tool call for terminal output.
pub(crate) fn tool_summary(call: &ToolCall) -> String {
    // Pick the most useful argument to display:
    // "command" for bash, "path" for read/write/edit, "question" for AskTool.
    let detail = call
        .arguments
        .get("command")
        .or_else(|| call.arguments.get("path"))
        .or_else(|| call.arguments.get("question"))
        .and_then(|v| v.as_str());

    match detail {
        Some(s) => format!("    [{}: {}]", call.name, s),
        None => format!("    [{}]", call.name),
    }
}

/// Handle a single prompt with at most one round of tool calls.
///
/// This function demonstrates the raw protocol:
/// 1. Send the prompt to the provider
/// 2. Match on stop_reason
/// 3. If Stop → return text
/// 4. If ToolUse → execute tools, send results, get final answer
pub async fn single_turn<P: Provider>(
    provider: &P,
    tools: &ToolSet,
    prompt: &str,
) -> anyhow::Result<String> {
    let defs = tools.definitions();
    let mut messages = vec![Message::User(prompt.to_string())];

    let turn = provider.chat(&messages, &defs).await?;

    match turn.stop_reason {
        StopReason::Stop => Ok(turn.text.unwrap_or_default()),
        StopReason::ToolUse => {
            let mut results = Vec::new();
            for call in &turn.tool_calls {
                print!("\x1b[2K\r{}\n", tool_summary(call));
                let content = match tools.get(&call.name) {
                    Some(t) => t
                        .call(call.arguments.clone())
                        .await
                        .unwrap_or_else(|e| format!("error: {e}")),
                    None => format!("error: unknown tool `{}`", call.name),
                };
                results.push((call.id.clone(), content));
            }

            messages.push(Message::Assistant(turn));
            for (id, content) in results {
                messages.push(Message::ToolResult { id, content });
            }

            let final_turn = provider.chat(&messages, &defs).await?;
            Ok(final_turn.text.unwrap_or_default())
        }
    }
}

pub struct SimpleAgent<P: Provider> {
    provider: P,
    tools: ToolSet,
}

impl<P: Provider> SimpleAgent<P> {
    pub fn new(provider: P) -> Self {
        Self {
            provider,
            tools: ToolSet::new(),
        }
    }

    pub fn tool(mut self, t: impl Tool + 'static) -> Self {
        self.tools.push(t);
        self
    }

    /// Execute all tool calls and return `(call_id, result_string)` pairs.
    async fn execute_tools(&self, calls: &[ToolCall]) -> Vec<(String, String)> {
        let mut results = Vec::with_capacity(calls.len());
        for call in calls {
            let content = match self.tools.get(&call.name) {
                Some(t) => t
                    .call(call.arguments.clone())
                    .await
                    .unwrap_or_else(|e| format!("error: {e}")),
                None => format!("error: unknown tool `{}`", call.name),
            };
            results.push((call.id.clone(), content));
        }
        results
    }

    /// Push an assistant turn and its tool results into the message history.
    fn push_results(
        messages: &mut Vec<Message>,
        turn: AssistantTurn,
        results: Vec<(String, String)>,
    ) {
        messages.push(Message::Assistant(turn));
        for (id, content) in results {
            messages.push(Message::ToolResult { id, content });
        }
    }

    /// Like [`run_with_events`](Self::run_with_events) but accepts an
    /// existing message history.  The caller pushes `Message::User(…)`
    /// before calling; on return the vec contains the full conversation
    /// including the assistant's final turn.
    pub async fn run_with_history(
        &self,
        mut messages: Vec<Message>,
        events: mpsc::UnboundedSender<AgentEvent>,
    ) -> Vec<Message> {
        let defs = self.tools.definitions();

        loop {
            let turn = match self.provider.chat(&messages, &defs).await {
                Ok(t) => t,
                Err(e) => {
                    let _ = events.send(AgentEvent::Error(e.to_string()));
                    return messages;
                }
            };

            match turn.stop_reason {
                StopReason::Stop => {
                    let _ = events.send(AgentEvent::Done(turn.text.clone().unwrap_or_default()));
                    messages.push(Message::Assistant(turn));
                    return messages;
                }
                StopReason::ToolUse => {
                    for call in &turn.tool_calls {
                        let _ = events.send(AgentEvent::ToolCall {
                            name: call.name.clone(),
                            summary: tool_summary(call),
                        });
                    }
                    let results = self.execute_tools(&turn.tool_calls).await;
                    Self::push_results(&mut messages, turn, results);
                }
            }
        }
    }

    /// Run the agent loop, sending events through the channel instead of
    /// printing to stdout. Sends `ToolCall` for each tool invocation,
    /// then `Done` or `Error` when finished.
    pub async fn run_with_events(&self, prompt: &str, events: mpsc::UnboundedSender<AgentEvent>) {
        let messages = vec![Message::User(prompt.to_string())];
        self.run_with_history(messages, events).await;
    }

    /// Run the agent loop, accumulating into the provided message history.
    ///
    /// The caller pushes `Message::User(…)` before calling; on return the
    /// vec contains the full conversation including the assistant's final
    /// turn.  Returns the text of the final response.
    pub async fn chat(&self, messages: &mut Vec<Message>) -> anyhow::Result<String> {
        let defs = self.tools.definitions();

        loop {
            let turn = self.provider.chat(messages, &defs).await?;

            match turn.stop_reason {
                StopReason::Stop => {
                    let text = turn.text.clone().unwrap_or_default();
                    messages.push(Message::Assistant(turn));
                    return Ok(text);
                }
                StopReason::ToolUse => {
                    for call in &turn.tool_calls {
                        print!("\x1b[2K\r{}\n", tool_summary(call));
                    }
                    let results = self.execute_tools(&turn.tool_calls).await;
                    Self::push_results(messages, turn, results);
                }
            }
        }
    }

    pub async fn run(&self, prompt: &str) -> anyhow::Result<String> {
        let mut messages = vec![Message::User(prompt.to_string())];
        self.chat(&mut messages).await
    }
}
