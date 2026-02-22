use std::collections::VecDeque;
use std::sync::Arc;

use serde_json::{Value, json};
use tokio::sync::{Mutex, oneshot};

use crate::types::{Tool, ToolDefinition};

/// Abstracts how user input is collected.
///
/// Implementations handle CLI prompts, TUI selection, or mock responses for tests.
#[async_trait::async_trait]
pub trait InputHandler: Send + Sync {
    async fn ask(&self, question: &str, options: &[String]) -> anyhow::Result<String>;
}

/// A request sent through a channel to collect user input asynchronously.
///
/// Used by [`ChannelInputHandler`] to communicate with a TUI event loop.
pub struct UserInputRequest {
    pub question: String,
    pub options: Vec<String>,
    pub response_tx: oneshot::Sender<String>,
}

/// Tool that lets the LLM ask the user a clarifying question.
pub struct AskTool {
    definition: ToolDefinition,
    handler: Arc<dyn InputHandler>,
}

impl AskTool {
    pub fn new(handler: Arc<dyn InputHandler>) -> Self {
        Self {
            definition: ToolDefinition::new(
                "ask_user",
                "Ask the user a clarifying question. Use this when you need more information \
                 before proceeding. The user will see your question and can provide a free-text \
                 answer or choose from the options you provide.",
            )
            .param("question", "string", "The question to ask the user", true)
            .param_raw(
                "options",
                json!({
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Optional list of choices to present to the user"
                }),
                false,
            ),
            handler,
        }
    }
}

#[async_trait::async_trait]
impl Tool for AskTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn call(&self, args: Value) -> anyhow::Result<String> {
        let question = args
            .get("question")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing required parameter: question"))?;

        let options: Vec<String> = args
            .get("options")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        self.handler.ask(question, &options).await
    }
}

/// Prints the question to stdout and reads the answer from stdin.
///
/// Suitable for simple CLI applications.
pub struct CliInputHandler;

#[async_trait::async_trait]
impl InputHandler for CliInputHandler {
    async fn ask(&self, question: &str, options: &[String]) -> anyhow::Result<String> {
        use std::io::{self, BufRead, Write};

        let q = question.to_string();
        let opts = options.to_vec();

        tokio::task::spawn_blocking(move || {
            println!("\n  {q}");
            if !opts.is_empty() {
                for (i, opt) in opts.iter().enumerate() {
                    println!("    {}) {opt}", i + 1);
                }
                print!("  > ");
            } else {
                print!("  > ");
            }
            io::stdout().flush()?;

            let mut line = String::new();
            io::stdin().lock().read_line(&mut line)?;
            let answer = line.trim().to_string();

            // If options provided and user entered a number, resolve it
            if !opts.is_empty()
                && let Ok(n) = answer.parse::<usize>()
                && n >= 1
                && n <= opts.len()
            {
                return Ok(opts[n - 1].clone());
            }

            Ok(answer)
        })
        .await?
    }
}

/// Sends a [`UserInputRequest`] through a channel and awaits the response.
///
/// Designed for TUI applications where input collection happens in a
/// separate event loop.
pub struct ChannelInputHandler {
    tx: tokio::sync::mpsc::UnboundedSender<UserInputRequest>,
}

impl ChannelInputHandler {
    pub fn new(tx: tokio::sync::mpsc::UnboundedSender<UserInputRequest>) -> Self {
        Self { tx }
    }
}

#[async_trait::async_trait]
impl InputHandler for ChannelInputHandler {
    async fn ask(&self, question: &str, options: &[String]) -> anyhow::Result<String> {
        let (response_tx, response_rx) = oneshot::channel();
        self.tx.send(UserInputRequest {
            question: question.to_string(),
            options: options.to_vec(),
            response_tx,
        })?;
        Ok(response_rx.await?)
    }
}

/// Returns pre-configured answers in sequence. Used in tests.
pub struct MockInputHandler {
    answers: Mutex<VecDeque<String>>,
}

impl MockInputHandler {
    pub fn new(answers: VecDeque<String>) -> Self {
        Self {
            answers: Mutex::new(answers),
        }
    }
}

#[async_trait::async_trait]
impl InputHandler for MockInputHandler {
    async fn ask(&self, _question: &str, _options: &[String]) -> anyhow::Result<String> {
        self.answers
            .lock()
            .await
            .pop_front()
            .ok_or_else(|| anyhow::anyhow!("MockInputHandler: no more answers"))
    }
}
