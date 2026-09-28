use serde::Deserialize;

use crate::message::Role;

#[derive(Debug, Clone, Deserialize)]
pub struct ChatResponse {
    /// The unique response id.
    pub id: String,
    /// The object type, always "chat.completion".
    pub object: String,
    /// The creation timestamp (Unix seconds).
    pub created: u64,
    /// The model actually used.
    pub model: String,
    /// The completion choices.
    pub choices: Vec<Choice>,
    /// Token usage statistics.
    pub usage: Option<Usage>,
    /// A fingerprint identifying the system configuration.
    #[serde(default)]
    pub system_fingerprint: Option<String>,
}

/// 单个候选回复。
#[derive(Debug, Clone, Deserialize)]
pub struct Choice {
    /// The index of the choice.
    pub index: u32,
    /// The completion message.
    pub message: ResponseMessage,
    /// The reason the completion finished.
    pub finish_reason: Option<String>,
}

impl Choice {
    /// Returns the text content of this choice.
    pub fn text(&self) -> &str {
        self.message.content()
    }
}

/// The message returned in a completion response.
#[derive(Debug, Clone, Deserialize)]
pub struct ResponseMessage {
    pub role: Role,
    pub content: Option<String>,
}

impl ResponseMessage {
    pub fn content(&self) -> &str {
        self.content.as_deref().unwrap_or("")
    }
}

/// Token usage statistics.
#[derive(Debug, Clone, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}
