use std::collections::HashSet;

use tokio::sync::mpsc;

use crate::agent::{AgentEvent, tool_summary};
use crate::streaming::{StreamEvent, StreamProvider};
use crate::types::*;

/// A two-phase agent that separates planning (read-only) from execution (all tools).
///
/// During the **plan** phase, only read-only tools are available — the LLM cannot
/// see or call write/edit/bash-mutating tools. During the **execute** phase, all
/// registered tools are available. The caller drives the approval flow between
/// the two phases.
pub struct PlanAgent<P: StreamProvider> {
    provider: P,
    tools: ToolSet,
    read_only: HashSet<&'static str>,
}

impl<P: StreamProvider> PlanAgent<P> {
    /// Create a new `PlanAgent` with default read-only tools: `bash` and `read`.
    pub fn new(provider: P) -> Self {
        Self {
            provider,
            tools: ToolSet::new(),
            read_only: HashSet::from(["bash", "read", "ask_user"]),
        }
    }

    /// Register a tool (builder pattern, same as `SimpleAgent`).
    pub fn tool(mut self, t: impl Tool + 'static) -> Self {
        self.tools.push(t);
        self
    }

    /// Override the set of tool names allowed during the planning phase.
    pub fn read_only(mut self, names: &[&'static str]) -> Self {
        self.read_only = names.iter().copied().collect();
        self
    }

    /// Run the **planning** phase: only read-only tools are available.
    ///
    /// The LLM sees only read-only tool definitions. If it somehow calls a
    /// blocked tool, the agent returns an error `ToolResult` instead of
    /// executing it.
    pub async fn plan(
        &self,
        messages: &mut Vec<Message>,
        events: mpsc::UnboundedSender<AgentEvent>,
    ) -> anyhow::Result<String> {
        self.run_loop(messages, Some(&self.read_only), events).await
    }

    /// Run the **execution** phase: all registered tools are available.
    pub async fn execute(
        &self,
        messages: &mut Vec<Message>,
        events: mpsc::UnboundedSender<AgentEvent>,
    ) -> anyhow::Result<String> {
        self.run_loop(messages, None, events).await
    }

    /// Shared agent loop. When `allowed` is `Some`, only those tool names are
    /// sent to the LLM and permitted for execution (double defense).
    async fn run_loop(
        &self,
        messages: &mut Vec<Message>,
        allowed: Option<&HashSet<&'static str>>,
        events: mpsc::UnboundedSender<AgentEvent>,
    ) -> anyhow::Result<String> {
        let all_defs = self.tools.definitions();
        let defs: Vec<&ToolDefinition> = match allowed {
            Some(names) => all_defs
                .into_iter()
                .filter(|d| names.contains(d.name))
                .collect(),
            None => all_defs,
        };

        loop {
            // Set up stream channel and forward text deltas to the UI
            let (stream_tx, mut stream_rx) = mpsc::unbounded_channel();
            let events_clone = events.clone();
            let forwarder = tokio::spawn(async move {
                while let Some(event) = stream_rx.recv().await {
                    if let StreamEvent::TextDelta(text) = event {
                        let _ = events_clone.send(AgentEvent::TextDelta(text));
                    }
                }
            });

            let turn = match self.provider.stream_chat(messages, &defs, stream_tx).await {
                Ok(t) => t,
                Err(e) => {
                    let _ = events.send(AgentEvent::Error(e.to_string()));
                    return Err(e);
                }
            };
            let _ = forwarder.await;

            match turn.stop_reason {
                StopReason::Stop => {
                    let text = turn.text.clone().unwrap_or_default();
                    let _ = events.send(AgentEvent::Done(text.clone()));
                    messages.push(Message::Assistant(turn));
                    return Ok(text);
                }
                StopReason::ToolUse => {
                    let mut results = Vec::with_capacity(turn.tool_calls.len());
                    for call in &turn.tool_calls {
                        // Execution guard: block tools not in the allowed set
                        if let Some(names) = allowed
                            && !names.contains(call.name.as_str())
                        {
                            results.push((
                                call.id.clone(),
                                format!(
                                    "error: tool '{}' is not available in planning mode",
                                    call.name
                                ),
                            ));
                            continue;
                        }

                        let _ = events.send(AgentEvent::ToolCall {
                            name: call.name.clone(),
                            summary: tool_summary(call),
                        });
                        let content = match self.tools.get(&call.name) {
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
                }
            }
        }
    }
}
