pub mod openai;

use tokio::sync::mpsc;

/// Provider response after completing a chat request.
#[derive(Debug)]
pub enum CompletionOutput {
    Full {
        text: String,
        tool_calls: Vec<ToolCall>,
        usage: Option<TokenUsage>,
    },
    Streamed(mpsc::Receiver<StreamEvent>),
}

/// Description of a tool call requested by the LLM.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

/// Token usage information returned by some providers.
#[derive(Debug, Clone, Default)]
pub struct TokenUsage {
    pub prompt: u32,
    pub completion: u32,
    pub total: u32,
}

/// Events emitted during streaming completion.
#[derive(Debug)]
pub enum StreamEvent {
    Token(String),
    ToolCall(ToolCall),
    Done {
        text: String,
        tool_calls: Vec<ToolCall>,
    },
}

/// A completion request sent to the provider.
/// The model is configured on the provider at construction time, not per-request.
#[derive(Debug, Clone)]
pub struct CompletionRequest {
    pub messages: Vec<Message>,
    pub tools: Vec<ToolDefinition>,
    pub temperature: f32,
    pub max_tokens: Option<u32>,
    pub stream: bool,
}

/// A single message in a conversation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

/// A tool definition sent to the LLM describing an available function.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolDefinition {
    #[serde(rename = "type")]
    pub kind: String, // "function"
    pub function: ToolFunction,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolFunction {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// Provider abstraction — implemented once per API family.
#[async_trait::async_trait]
pub trait Provider: Send + Sync {
    async fn complete(
        &self,
        req: CompletionRequest,
    ) -> Result<CompletionOutput, crate::error::AgentError>;
    fn name(&self) -> &str;
}
