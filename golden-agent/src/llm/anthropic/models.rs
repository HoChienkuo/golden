/// Commonly used Anthropic models.
///
/// Mirrors the model ids accepted by the Messages API, cross-checked against Anthropic's
/// model overview and lifecycle tables as of 2026-11-06. This is a curated set of the
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
    #[deprecated(note = "deprecated by Anthropic; use `ClaudeMythos51` instead")]
    ClaudeMythosPreview,
    /// `claude-opus-5-5`
    ClaudeOpus55,
    /// `claude-opus-5`
    ClaudeOpus5,
    /// `claude-sonnet-5-5`
    #[default]
    ClaudeSonnet55,
    /// `claude-sonnet-5`
    ClaudeSonnet5,
    /// `claude-haiku-5-5`
    ClaudeHaiku55,
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
    #[deprecated(note = "retiring 2026-11-30; use `ClaudeSonnet55` instead")]
    ClaudeSonnet45,
    /// `claude-haiku-4-5`
    ClaudeHaiku45,
    // Legacy, deprecated.
    #[deprecated(note = "no longer in the current model lineup; use `ClaudeSonnet55`")]
    /// `claude-3-5-sonnet-latest`
    Claude35Sonnet,
    #[deprecated(note = "no longer in the current model lineup; use `ClaudeHaiku55`")]
    /// `claude-3-5-haiku-latest`
    Claude35Haiku,
    /// Any other model id, such as a dated snapshot or a custom deployment.
    Custom(String),
}

impl AnthropicModel {
    /// All non-deprecated built-in models, excluding custom model ids.
    pub const BUILT_IN: &'static [Self] = &[
        Self::ClaudeFable51,
        Self::ClaudeFable5,
        Self::ClaudeMythos51,
        Self::ClaudeMythos5,
        Self::ClaudeOpus55,
        Self::ClaudeOpus5,
        Self::ClaudeSonnet55,
        Self::ClaudeSonnet5,
        Self::ClaudeHaiku55,
        Self::ClaudeOpus48,
        Self::ClaudeOpus47,
        Self::ClaudeOpus46,
        Self::ClaudeOpus45,
        Self::ClaudeSonnet46,
        Self::ClaudeHaiku45,
    ];

    /// Iterates over all non-deprecated built-in models.
    pub fn built_in() -> impl ExactSizeIterator<Item = &'static Self> {
        Self::BUILT_IN.iter()
    }

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
            AnthropicModel::ClaudeHaiku55 => "claude-haiku-5-5",
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
