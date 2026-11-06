use std::fmt;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::Error;
use crate::llm::chat::Usage;

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
    /// The total context window in tokens, when supplied by the provider.
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

    /// Measures a call's input usage against this model's known limit.
    ///
    /// Prefers the input limit and falls back to the total context window.
    /// A zero or missing limit is treated as unknown.
    pub fn context_usage(&self, usage: &Usage) -> ContextUsage {
        ContextUsage {
            input_tokens: usage.prompt_tokens,
            limit_tokens: self
                .max_input_tokens
                .filter(|&limit| limit > 0)
                .or(self.context_window.filter(|&limit| limit > 0)),
        }
    }
}

/// Input context occupancy measured for one completed model call.
///
/// Its display format is `33%  33k / 100k`, or just `33k` when the limit is unknown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContextUsage {
    /// The input tokens reported for the call.
    pub input_tokens: u32,
    /// The input limit, or the total context window if only that is known.
    pub limit_tokens: Option<u64>,
}

impl ContextUsage {
    /// Returns the input percentage of the limit without rounding or clamping.
    pub fn percent(&self) -> Option<f64> {
        self.limit_tokens
            .filter(|&limit| limit > 0)
            .map(|limit| f64::from(self.input_tokens) * 100.0 / limit as f64)
    }
}

impl fmt::Display for ContextUsage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let input = compact_tokens(u64::from(self.input_tokens));
        match (self.percent(), self.limit_tokens) {
            (Some(percent), Some(limit)) => {
                write!(f, "{percent:.0}%  {input} / {}", compact_tokens(limit))
            }
            _ => f.write_str(&input),
        }
    }
}

fn compact_tokens(tokens: u64) -> String {
    if tokens < 1_000 {
        return tokens.to_string();
    }
    let thousands = format!("{:.1}", tokens as f64 / 1_000.0);
    format!("{}k", thousands.trim_end_matches(".0"))
}

/// A provider capable of listing the models available to its credentials.
#[async_trait]
pub trait ModelCatalog: Send + Sync {
    /// Fetches all models exposed by the provider's model-listing endpoint.
    async fn list_models(&self) -> Result<Vec<ModelInfo>, Error>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_context_usage_with_or_without_a_known_limit() {
        let usage = Usage {
            prompt_tokens: 33_000,
            completion_tokens: 2_000,
            total_tokens: 35_000,
            ..Usage::default()
        };
        let mut model = ModelInfo::new("test");
        assert_eq!(model.context_usage(&usage).to_string(), "33k");

        model.context_window = Some(100_000);
        assert_eq!(model.context_usage(&usage).to_string(), "33%  33k / 100k");

        model.max_input_tokens = Some(50_000);
        assert_eq!(model.context_usage(&usage).to_string(), "66%  33k / 50k");
    }
}
