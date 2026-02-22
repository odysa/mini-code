use anyhow::Context;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::types::*;

// ---------------------------------------------------------------------------
// OpenAI-compatible request/response types (provided — do not modify)
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub(crate) struct ChatRequest<'a> {
    pub(crate) model: &'a str,
    pub(crate) messages: Vec<ApiMessage>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) tools: Vec<ApiTool>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct ApiMessage {
    pub(crate) role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tool_calls: Option<Vec<ApiToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tool_call_id: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct ApiToolCall {
    pub(crate) id: String,
    #[serde(rename = "type")]
    pub(crate) type_: String,
    pub(crate) function: ApiFunction,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct ApiFunction {
    pub(crate) name: String,
    pub(crate) arguments: String,
}

#[derive(Serialize, Debug)]
pub(crate) struct ApiTool {
    #[serde(rename = "type")]
    pub(crate) type_: &'static str,
    pub(crate) function: ApiToolDef,
}

#[derive(Serialize, Debug)]
pub(crate) struct ApiToolDef {
    pub(crate) name: &'static str,
    pub(crate) description: &'static str,
    pub(crate) parameters: Value,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ResponseMessage,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct ResponseMessage {
    content: Option<String>,
    tool_calls: Option<Vec<ApiToolCall>>,
}

// ---------------------------------------------------------------------------
// Provider implementation
// ---------------------------------------------------------------------------

/// An HTTP provider that talks to OpenRouter (or any OpenAI-compatible API).
///
/// # Chapter 6: The HTTP Provider
///
/// Your task: Implement the constructor methods and the Provider trait.
/// The serde types above handle serialization — you write the logic.
pub struct OpenRouterProvider {
    client: reqwest::Client,
    api_key: String,
    model: String,
    base_url: String,
}

impl OpenRouterProvider {
    /// Create a new provider with the given API key and model name.
    ///
    /// Hint: Use `reqwest::Client::new()` and `.into()` for string conversion.
    pub fn new(_api_key: impl Into<String>, _model: impl Into<String>) -> Self {
        unimplemented!("Initialize all four fields: client, api_key, model, base_url")
    }

    /// Override the base URL (useful for testing with a local server).
    ///
    /// Default is "https://openrouter.ai/api/v1".
    pub fn base_url(mut self, _url: impl Into<String>) -> Self {
        unimplemented!("Set self.base_url and return self")
    }

    /// Create a provider from the OPENROUTER_API_KEY environment variable.
    pub fn from_env_with_model(_model: impl Into<String>) -> anyhow::Result<Self> {
        unimplemented!(
            "Load .env with dotenvy::dotenv(), read OPENROUTER_API_KEY from env, call Self::new()"
        )
    }

    /// Create a provider from env with the default model.
    pub fn from_env() -> anyhow::Result<Self> {
        unimplemented!("Call from_env_with_model with \"openrouter/free\"")
    }

    /// Convert our Message types to the API's message format.
    ///
    /// Hint: Match on each Message variant and create the corresponding ApiMessage.
    /// - System -> role: "system", content: Some(text.clone())
    /// - User -> role: "user", content: Some(text.clone())
    /// - Assistant -> role: "assistant", content: turn.text.clone(),
    ///   tool_calls: convert each ToolCall to ApiToolCall (arguments.to_string() for Value→String)
    ///   Set tool_calls to None (not Some(vec![])) when empty.
    /// - ToolResult -> role: "tool", content: Some(content.clone()), tool_call_id: Some(id.clone())
    pub(crate) fn convert_messages(_messages: &[Message]) -> Vec<ApiMessage> {
        unimplemented!(
            "Convert each Message to an ApiMessage — note: ToolCall.arguments (Value) must be serialized to String"
        )
    }

    /// Convert our ToolDefinition types to the API's tool format.
    ///
    /// Each tool becomes: { type: "function", function: { name, description, parameters } }
    pub(crate) fn convert_tools(_tools: &[&ToolDefinition]) -> Vec<ApiTool> {
        unimplemented!("Convert each ToolDefinition to an ApiTool")
    }
}

impl Provider for OpenRouterProvider {
    /// Send a chat request to the API and parse the response.
    ///
    /// Steps:
    /// 1. Build a ChatRequest with model, converted messages, and converted tools
    /// 2. POST to {base_url}/chat/completions with bearer auth
    /// 3. Parse the JSON response as ChatResponse
    /// 4. Extract the first choice's message
    /// 5. Convert any tool_calls back to our ToolCall type
    ///    (parse function.arguments from String to Value with serde_json::from_str)
    /// 6. Determine stop_reason from choice.finish_reason:
    ///    - "tool_calls" → StopReason::ToolUse
    ///    - anything else → StopReason::Stop
    async fn chat(
        &self,
        _messages: &[Message],
        _tools: &[&ToolDefinition],
    ) -> anyhow::Result<AssistantTurn> {
        unimplemented!("Build request, send HTTP POST, parse response into AssistantTurn")
    }
}
