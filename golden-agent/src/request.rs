use serde::Serialize;

use crate::message::Message;

#[derive(Debug, Clone, Serialize)]
pub struct ChatRequest {
    /// The model name.
    pub model: String,
    /// The message history.
    pub messages: Vec<Message>,
    /// Sampling temperature (0.0 to 2.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Nucleus sampling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    /// Whether to stream the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    /// Maximum number of tokens to generate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    /// Stop sequences.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Stop>,
    /// A unique identifier for the end user.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

/// A stop sequence: either a single string or a list of strings.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum Stop {
    Single(String),
    Multiple(Vec<String>),
}

impl ChatRequest {
    /// Creates a new request for the given model.
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            messages: Vec::new(),
            temperature: None,
            top_p: None,
            stream: None,
            max_tokens: None,
            stop: None,
            user: None,
        }
    }

    /// Appends a message.
    pub fn add_message(mut self, message: Message) -> Self {
        self.messages.push(message);
        self
    }

    /// Appends multiple messages.
    pub fn messages(mut self, messages: impl IntoIterator<Item = Message>) -> Self {
        self.messages.extend(messages);
        self
    }

    /// Sets the sampling temperature.
    pub fn temperature(mut self, value: f32) -> Self {
        self.temperature = Some(value);
        self
    }

    /// Sets the maximum number of tokens.
    pub fn max_tokens(mut self, value: u32) -> Self {
        self.max_tokens = Some(value);
        self
    }

    /// Sets the stop sequence.
    pub fn stop(mut self, value: impl Into<Stop>) -> Self {
        self.stop = Some(value.into());
        self
    }
}

impl From<String> for Stop {
    fn from(value: String) -> Self {
        Stop::Single(value)
    }
}

impl From<&str> for Stop {
    fn from(value: &str) -> Self {
        Stop::Single(value.to_string())
    }
}

impl From<Vec<String>> for Stop {
    fn from(value: Vec<String>) -> Self {
        Stop::Multiple(value)
    }
}
