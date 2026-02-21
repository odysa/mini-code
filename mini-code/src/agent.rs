use crate::types::*;

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
                let content = match tools.get(&call.name) {
                    Some(t) => t.call(call.arguments.clone()).await?,
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

    pub async fn run(&self, prompt: &str) -> anyhow::Result<String> {
        let defs = self.tools.definitions();
        let mut messages = vec![Message::User(prompt.to_string())];

        loop {
            let turn = self.provider.chat(&messages, &defs).await?;

            match turn.stop_reason {
                StopReason::Stop => return Ok(turn.text.unwrap_or_default()),
                StopReason::ToolUse => {
                    let mut results = Vec::with_capacity(turn.tool_calls.len());
                    for call in &turn.tool_calls {
                        let content = match self.tools.get(&call.name) {
                            Some(t) => t.call(call.arguments.clone()).await?,
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
