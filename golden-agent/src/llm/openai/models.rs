/// Commonly used OpenAI models.
///
/// A curated set of chat models from the official OpenAI API catalog as of
/// 2026-11-06. Dated snapshots, fine-tunes, and specialized models are available
/// through [`OpenAiModel::Custom`].
///
/// Deprecated variants retain their original IDs for callers that reference them.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum OpenAiModel {
    // GPT-5.x family.
    #[deprecated(note = "deprecated by OpenAI; use `Gpt61Sol` instead")]
    Gpt5,
    #[deprecated(note = "deprecated by OpenAI; use `Gpt6Luna` instead")]
    Gpt5Mini,
    #[deprecated(note = "deprecated by OpenAI; use `Gpt6Luna` instead")]
    Gpt5Nano,
    #[deprecated(note = "deprecated by OpenAI; use `Gpt61Sol` instead")]
    Gpt5Pro,
    #[deprecated(note = "deprecated by OpenAI; use `Gpt61Sol` instead")]
    Gpt51,
    Gpt52,
    Gpt52Pro,
    Gpt54,
    Gpt54Mini,
    #[deprecated(note = "deprecated by OpenAI; use `Gpt6Luna` instead")]
    Gpt54Nano,
    Gpt54Pro,
    Gpt55,
    Gpt55Pro,
    Gpt56,
    Gpt56Luna,
    Gpt56Sol,
    Gpt56Terra,
    // GPT-6 family.
    Gpt6Astra,
    Gpt6Luna,
    Gpt6Sol,
    /// `gpt-6.1-sol`
    #[default]
    Gpt61Sol,
    /// `gpt-6.1-terra`
    Gpt61Terra,
    // Reasoning (o-series).
    #[deprecated(note = "deprecated by OpenAI (retiring 2026-10-23); use `Gpt61Sol` instead")]
    O1,
    #[deprecated(note = "deprecated by OpenAI (retiring 2026-10-23); use `Gpt61Sol` instead")]
    O1Pro,
    #[deprecated(note = "deprecated by OpenAI; use `Gpt61Sol` instead")]
    O3,
    #[deprecated(note = "deprecated by OpenAI (retiring 2026-10-23); use `Gpt6Luna` instead")]
    O3Mini,
    #[deprecated(note = "deprecated by OpenAI; use `Gpt61Sol` instead")]
    O3Pro,
    #[deprecated(note = "deprecated by OpenAI (retiring 2026-10-23); use `Gpt6Luna` instead")]
    O4Mini,
    // Legacy GPT-4.x models.
    #[deprecated(note = "deprecated by OpenAI; use `Gpt61Sol` instead")]
    Gpt41,
    #[deprecated(note = "deprecated by OpenAI; use `Gpt6Luna` instead")]
    Gpt41Mini,
    #[deprecated(note = "deprecated by OpenAI (retiring 2026-10-23); use `Gpt6Luna` instead")]
    Gpt41Nano,
    #[deprecated(note = "deprecated by OpenAI; use `Gpt61Sol` instead")]
    Gpt4o,
    #[deprecated(note = "deprecated by OpenAI; use `Gpt6Luna` instead")]
    Gpt4oMini,
    // Audio-capable models.
    #[deprecated(note = "deprecated by OpenAI; use `GptAudio15` instead")]
    GptAudio,
    #[deprecated(note = "deprecated by OpenAI; use `GptAudio15` instead")]
    GptAudioMini,
    /// `gpt-audio-1.5`
    GptAudio15,
    // Legacy models (deprecated).
    #[deprecated(note = "deprecated by OpenAI (retiring 2026-10-23); use `Gpt6Luna` instead")]
    Gpt35Turbo,
    #[deprecated(note = "deprecated by OpenAI (retiring 2026-10-23); use `Gpt61Sol` instead")]
    Gpt4,
    #[deprecated(note = "deprecated by OpenAI (retiring 2026-10-23); use `Gpt61Sol` instead")]
    Gpt4Turbo,
    /// Any other model id, such as a dated snapshot or a fine-tune.
    Custom(String),
}

impl OpenAiModel {
    /// All non-deprecated built-in models, excluding custom model ids.
    pub const BUILT_IN: &'static [Self] = &[
        Self::Gpt52,
        Self::Gpt52Pro,
        Self::Gpt54,
        Self::Gpt54Mini,
        Self::Gpt54Pro,
        Self::Gpt55,
        Self::Gpt55Pro,
        Self::Gpt56,
        Self::Gpt56Luna,
        Self::Gpt56Sol,
        Self::Gpt56Terra,
        Self::Gpt6Astra,
        Self::Gpt6Luna,
        Self::Gpt6Sol,
        Self::Gpt61Sol,
        Self::Gpt61Terra,
        Self::GptAudio15,
    ];

    /// Iterates over all non-deprecated built-in models.
    pub fn built_in() -> impl ExactSizeIterator<Item = &'static Self> {
        Self::BUILT_IN.iter()
    }

    /// Returns the model id sent in the request.
    #[allow(deprecated)]
    pub fn as_str(&self) -> &str {
        match self {
            OpenAiModel::Gpt5 => "gpt-5",
            OpenAiModel::Gpt5Mini => "gpt-5-mini",
            OpenAiModel::Gpt5Nano => "gpt-5-nano",
            OpenAiModel::Gpt5Pro => "gpt-5-pro",
            OpenAiModel::Gpt51 => "gpt-5.1",
            OpenAiModel::Gpt52 => "gpt-5.2",
            OpenAiModel::Gpt52Pro => "gpt-5.2-pro",
            OpenAiModel::Gpt54 => "gpt-5.4",
            OpenAiModel::Gpt54Mini => "gpt-5.4-mini",
            OpenAiModel::Gpt54Nano => "gpt-5.4-nano",
            OpenAiModel::Gpt54Pro => "gpt-5.4-pro",
            OpenAiModel::Gpt55 => "gpt-5.5",
            OpenAiModel::Gpt55Pro => "gpt-5.5-pro",
            OpenAiModel::Gpt56 => "gpt-5.6",
            OpenAiModel::Gpt56Luna => "gpt-5.6-luna",
            OpenAiModel::Gpt56Sol => "gpt-5.6-sol",
            OpenAiModel::Gpt56Terra => "gpt-5.6-terra",
            OpenAiModel::Gpt6Astra => "gpt-6-astra",
            OpenAiModel::Gpt6Luna => "gpt-6-luna",
            OpenAiModel::Gpt6Sol => "gpt-6-sol",
            OpenAiModel::Gpt61Sol => "gpt-6.1-sol",
            OpenAiModel::Gpt61Terra => "gpt-6.1-terra",
            OpenAiModel::O1 => "o1",
            OpenAiModel::O1Pro => "o1-pro",
            OpenAiModel::O3 => "o3",
            OpenAiModel::O3Mini => "o3-mini",
            OpenAiModel::O3Pro => "o3-pro",
            OpenAiModel::O4Mini => "o4-mini",
            OpenAiModel::Gpt41 => "gpt-4.1",
            OpenAiModel::Gpt41Mini => "gpt-4.1-mini",
            OpenAiModel::Gpt41Nano => "gpt-4.1-nano",
            OpenAiModel::Gpt4o => "gpt-4o",
            OpenAiModel::Gpt4oMini => "gpt-4o-mini",
            OpenAiModel::GptAudio => "gpt-audio",
            OpenAiModel::GptAudioMini => "gpt-audio-mini",
            OpenAiModel::GptAudio15 => "gpt-audio-1.5",
            OpenAiModel::Gpt35Turbo => "gpt-3.5-turbo",
            OpenAiModel::Gpt4 => "gpt-4",
            OpenAiModel::Gpt4Turbo => "gpt-4-turbo",
            OpenAiModel::Custom(s) => s,
        }
    }
}
