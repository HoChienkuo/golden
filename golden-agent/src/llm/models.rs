use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::Error;

/// Metadata for a model returned by a provider's model-listing endpoint.
///
/// Provider APIs expose different capability metadata. For example, DeepSeek
/// can return [`context_window`](Self::context_window), which is `None` when
/// the response omits that field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelInfo {
    /// The model id accepted by [`crate::ChatRequest`].
    pub id: String,
    /// A human-readable model name, when supplied by the provider.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// The model owner, when supplied by the provider.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owned_by: Option<String>,
    /// The provider's creation timestamp, represented without normalizing its format.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    /// The maximum context window in tokens, when supplied by the provider.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_window: Option<u64>,
    /// The maximum input size in tokens, when supplied by the provider.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_input_tokens: Option<u64>,
    /// The maximum output size in tokens, when supplied by the provider.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u64>,
}

impl ModelInfo {
    /// Constructs model metadata containing only an id.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            display_name: None,
            owned_by: None,
            created_at: None,
            context_window: None,
            max_input_tokens: None,
            max_output_tokens: None,
        }
    }

    /// Sets the context-window size.
    ///
    /// This is useful when a provider lists a model but does not publish its
    /// context window through the model-listing endpoint.
    pub fn with_context_window(mut self, context_window: u64) -> Self {
        self.context_window = Some(context_window);
        self
    }
}

/// A provider capable of listing the models available to its credentials.
#[async_trait]
pub trait ModelCatalog: Send + Sync {
    /// Fetches all models exposed by the provider's model-listing endpoint.
    async fn list_models(&self) -> Result<Vec<ModelInfo>, Error>;
}
