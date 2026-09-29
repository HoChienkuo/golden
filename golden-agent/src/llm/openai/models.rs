/// Commonly used OpenAI models.
///
/// Updated against the OpenAI chat lineup as of 2026-09, cross-checked via the
/// OpenRouter, LiteLLM and models.dev registries. This is a curated set of the
/// current chat models and is deliberately not exhaustive: dated snapshots
/// (e.g. `gpt-5.4-2026-03-05`), fine-tunes and specialized variants are reached
/// through [`OpenAiModel::Custom`].
///
/// Variants carrying `#[deprecated]` are on OpenAI's retirement schedule (see
/// each variant's note). They remain callable until that date but should be
/// migrated away from.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum OpenAiModel {
    // GPT-5.x family.
    #[default]
    Gpt5,
    Gpt5Mini,
    Gpt5Nano,
    Gpt5Pro,
    Gpt51,
    Gpt52,
    Gpt52Pro,
    Gpt54,
    Gpt54Mini,
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
    // Reasoning (o-series).
    #[deprecated(note = "deprecated by OpenAI (retiring 2026-10-23); use `Gpt5` instead")]
    O1,
    #[deprecated(note = "deprecated by OpenAI (retiring 2026-10-23); use `Gpt5Pro` instead")]
    O1Pro,
    O3,
    #[deprecated(note = "deprecated by OpenAI (retiring 2026-10-23); use `Gpt5Mini` instead")]
    O3Mini,
    O3Pro,
    #[deprecated(note = "deprecated by OpenAI (retiring 2026-10-23); use `Gpt5Mini` instead")]
    O4Mini,
    // GPT-4.x family (older, still available).
    Gpt41,
    Gpt41Mini,
    #[deprecated(note = "deprecated by OpenAI (retiring 2026-10-23); use `Gpt54Nano` instead")]
    Gpt41Nano,
    Gpt4o,
    Gpt4oMini,
    // Audio-capable models.
    GptAudio,
    GptAudioMini,
    // Legacy models (deprecated).
    #[deprecated(note = "deprecated by OpenAI (retiring 2026-10-23); use `Gpt5Mini` instead")]
    Gpt35Turbo,
    #[deprecated(note = "deprecated by OpenAI (retiring 2026-10-23); use `Gpt5` instead")]
    Gpt4,
    #[deprecated(note = "deprecated by OpenAI (retiring 2026-10-23); use `Gpt5` instead")]
    Gpt4Turbo,
    /// Any other model id, such as a dated snapshot or a fine-tune.
    Custom(String),
}

impl OpenAiModel {
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
            OpenAiModel::Gpt35Turbo => "gpt-3.5-turbo",
            OpenAiModel::Gpt4 => "gpt-4",
            OpenAiModel::Gpt4Turbo => "gpt-4-turbo",
            OpenAiModel::Custom(s) => s,
        }
    }
}
