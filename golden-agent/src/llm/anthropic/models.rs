/// Commonly used Anthropic models.
///
/// Mirrors the model ids accepted by the Messages API, cross-checked against the
/// `Model` type of the official SDK as of 2026-09. This is a curated set of the
/// current models and is deliberately not exhaustive: dated snapshots (e.g.
/// `claude-opus-4-5-20251101`), fine-tunes, and custom deployments are reached
/// through [`AnthropicModel::Custom`].
///
/// Variants carrying `#[deprecated]` have fallen out of the current model
/// lineup and should be migrated away from.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum AnthropicModel {
    // Claude 5.x family.
    /// `claude-fable-5-1`
    ClaudeFable51,
    /// `claude-fable-5`
    ClaudeFable5,
    /// `claude-mythos-5-1`
    ClaudeMythos51,
    /// `claude-mythos-5`
    ClaudeMythos5,
    /// `claude-mythos-preview`
    ClaudeMythosPreview,
    /// `claude-opus-5-5`
    ClaudeOpus55,
    /// `claude-opus-5`
    ClaudeOpus5,
    /// `claude-sonnet-5-5`
    ClaudeSonnet55,
    /// `claude-sonnet-5`
    #[default]
    ClaudeSonnet5,
    // Claude 4.x family.
    /// `claude-opus-4-8`
    ClaudeOpus48,
    /// `claude-opus-4-7`
    ClaudeOpus47,
    /// `claude-opus-4-6`
    ClaudeOpus46,
    /// `claude-opus-4-5`
    ClaudeOpus45,
    /// `claude-sonnet-4-6`
    ClaudeSonnet46,
    /// `claude-sonnet-4-5`
    ClaudeSonnet45,
    /// `claude-haiku-4-5`
    ClaudeHaiku45,
    // Legacy, deprecated.
    #[deprecated(note = "no longer in the current model lineup; use `ClaudeSonnet5` or newer")]
    /// `claude-3-5-sonnet-latest`
    Claude35Sonnet,
    #[deprecated(note = "no longer in the current model lineup; use `ClaudeHaiku45` or newer")]
    /// `claude-3-5-haiku-latest`
    Claude35Haiku,
    /// Any other model id, such as a dated snapshot or a custom deployment.
    Custom(String),
}

impl AnthropicModel {
    /// Returns the model id sent in the request.
    #[allow(deprecated)]
    pub fn as_str(&self) -> &str {
        match self {
            AnthropicModel::ClaudeFable51 => "claude-fable-5-1",
            AnthropicModel::ClaudeFable5 => "claude-fable-5",
            AnthropicModel::ClaudeMythos51 => "claude-mythos-5-1",
            AnthropicModel::ClaudeMythos5 => "claude-mythos-5",
            AnthropicModel::ClaudeMythosPreview => "claude-mythos-preview",
            AnthropicModel::ClaudeOpus55 => "claude-opus-5-5",
            AnthropicModel::ClaudeOpus5 => "claude-opus-5",
            AnthropicModel::ClaudeSonnet55 => "claude-sonnet-5-5",
            AnthropicModel::ClaudeSonnet5 => "claude-sonnet-5",
            AnthropicModel::ClaudeOpus48 => "claude-opus-4-8",
            AnthropicModel::ClaudeOpus47 => "claude-opus-4-7",
            AnthropicModel::ClaudeOpus46 => "claude-opus-4-6",
            AnthropicModel::ClaudeOpus45 => "claude-opus-4-5",
            AnthropicModel::ClaudeSonnet46 => "claude-sonnet-4-6",
            AnthropicModel::ClaudeSonnet45 => "claude-sonnet-4-5",
            AnthropicModel::ClaudeHaiku45 => "claude-haiku-4-5",
            AnthropicModel::Claude35Sonnet => "claude-3-5-sonnet-latest",
            AnthropicModel::Claude35Haiku => "claude-3-5-haiku-latest",
            AnthropicModel::Custom(model) => model,
        }
    }
}
