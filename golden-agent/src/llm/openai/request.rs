use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::message::Message;
use super::tool::{FunctionDefinition, Tool, ToolChoice};

/// A chat completion request body, following the OpenAI Chat Completions schema.
///
/// Optional fields are omitted from the serialized body when unset.
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
    /// How many chat completion choices to generate for each input message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<u32>,
    /// Whether to stream the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    /// Streaming-specific options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_options: Option<StreamOptions>,
    /// Stop sequences.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Stop>,
    /// An upper bound on the number of tokens to generate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_completion_tokens: Option<u32>,
    /// An upper bound on the number of tokens to generate.
    #[deprecated(note = "deprecated by OpenAI; use `max_completion_tokens` instead")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    /// Positive values penalize new tokens based on their existing frequency
    /// (-2.0 to 2.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,
    /// Positive values penalize new tokens based on whether they appear in the
    /// text so far (-2.0 to 2.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,
    /// A map of token ids to a bias in the range -100 to 100.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logit_bias: Option<HashMap<String, i32>>,
    /// Whether to return log probabilities of the output tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<bool>,
    /// The number of most likely tokens to return at each position (0 to 20).
    /// Requires `logprobs` to be `true`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_logprobs: Option<u32>,
    /// The format the model must output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,
    /// If specified, the backend makes a best effort to sample deterministically.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,
    /// The tools the model may call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    /// Controls which tool the model may call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,
    /// Whether to allow the model to call multiple tools in a single turn.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parallel_tool_calls: Option<bool>,
    /// Constrains the reasoning effort for reasoning models.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<ReasoningEffort>,
    /// The output modalities to enable, e.g. `["text", "audio"]`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modalities: Option<Vec<Modality>>,
    /// Audio output parameters, required when `audio` is a listed modality.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<AudioOutput>,
    /// A static prediction of the content the model will produce.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prediction: Option<Value>,
    /// Whether to store the output for model distillation or evals.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub store: Option<bool>,
    /// Up to 16 key/value pairs attached to the object.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
    /// The service tier to use for processing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<ServiceTier>,
    /// A stable identifier used to help detect unsafe use.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_identifier: Option<String>,
    /// Options for the built-in web search tool.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub web_search_options: Option<Value>,
    /// A unique identifier for the end user.
    #[deprecated(note = "deprecated by OpenAI; use `safety_identifier` instead")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    /// Legacy function definitions.
    #[deprecated(note = "deprecated by OpenAI; use `tools` instead")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub functions: Option<Vec<FunctionDefinition>>,
    /// Legacy function selection control.
    #[deprecated(note = "deprecated by OpenAI; use `tool_choice` instead")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_call: Option<Value>,
}

/// Options that apply only when `stream` is `true`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StreamOptions {
    /// Whether to include a final chunk with the full token usage.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_usage: Option<bool>,
}

impl StreamOptions {
    /// Constructs `include_usage: true` stream options.
    pub fn include_usage() -> Self {
        Self {
            include_usage: Some(true),
        }
    }
}

/// The format the model must produce.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseFormat {
    /// Plain text output (the default).
    Text,
    /// A JSON object the model populates.
    JsonObject,
    /// A JSON object conforming to a supplied JSON Schema.
    JsonSchema { json_schema: JsonSchema },
}

/// A JSON Schema response format definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonSchema {
    /// The name of the schema.
    pub name: String,
    /// A description of the schema, used by the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The JSON Schema the output must conform to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<Value>,
    /// Whether to enable strict schema adherence.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

/// The reasoning effort level for reasoning models.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningEffort {
    Minimal,
    Low,
    Medium,
    High,
}

/// An output modality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Modality {
    Text,
    Audio,
}

/// Audio output configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioOutput {
    /// The audio format the model outputs.
    pub format: AudioFormat,
    /// The voice to use, e.g. `alloy`.
    pub voice: String,
}

/// An audio output container format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AudioFormat {
    Wav,
    Aac,
    Mp3,
    Flac,
    Opus,
    Pcm16,
}

/// The service tier used to process the request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ServiceTier {
    Auto,
    Default,
    Flex,
    Priority,
}

/// A stop sequence: either a single string or a list of strings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Stop {
    Single(String),
    Multiple(Vec<String>),
}

#[allow(deprecated)]
impl ChatRequest {
    /// Creates a new request for the given model.
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            messages: Vec::new(),
            temperature: None,
            top_p: None,
            n: None,
            stream: None,
            stream_options: None,
            stop: None,
            max_completion_tokens: None,
            max_tokens: None,
            frequency_penalty: None,
            presence_penalty: None,
            logit_bias: None,
            logprobs: None,
            top_logprobs: None,
            response_format: None,
            seed: None,
            tools: None,
            tool_choice: None,
            parallel_tool_calls: None,
            reasoning_effort: None,
            modalities: None,
            audio: None,
            prediction: None,
            store: None,
            metadata: None,
            service_tier: None,
            safety_identifier: None,
            web_search_options: None,
            user: None,
            functions: None,
            function_call: None,
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

    /// Sets the nucleus sampling probability.
    pub fn top_p(mut self, value: f32) -> Self {
        self.top_p = Some(value);
        self
    }

    /// Sets the number of completion choices to generate.
    pub fn n(mut self, value: u32) -> Self {
        self.n = Some(value);
        self
    }

    /// Sets whether to stream the response.
    pub fn stream(mut self, value: bool) -> Self {
        self.stream = Some(value);
        self
    }

    /// Sets the streaming options.
    pub fn stream_options(mut self, value: StreamOptions) -> Self {
        self.stream_options = Some(value);
        self
    }

    /// Sets the stop sequence.
    pub fn stop(mut self, value: impl Into<Stop>) -> Self {
        self.stop = Some(value.into());
        self
    }

    /// Sets the maximum number of completion tokens.
    pub fn max_completion_tokens(mut self, value: u32) -> Self {
        self.max_completion_tokens = Some(value);
        self
    }

    /// Sets the maximum number of tokens.
    #[deprecated(note = "deprecated by OpenAI; use `max_completion_tokens` instead")]
    pub fn max_tokens(mut self, value: u32) -> Self {
        self.max_tokens = Some(value);
        self
    }

    /// Sets the frequency penalty.
    pub fn frequency_penalty(mut self, value: f32) -> Self {
        self.frequency_penalty = Some(value);
        self
    }

    /// Sets the presence penalty.
    pub fn presence_penalty(mut self, value: f32) -> Self {
        self.presence_penalty = Some(value);
        self
    }

    /// Sets the token logit bias map.
    pub fn logit_bias(mut self, value: HashMap<String, i32>) -> Self {
        self.logit_bias = Some(value);
        self
    }

    /// Sets whether to return output log probabilities.
    pub fn logprobs(mut self, value: bool) -> Self {
        self.logprobs = Some(value);
        self
    }

    /// Sets the number of most likely tokens to return at each position.
    pub fn top_logprobs(mut self, value: u32) -> Self {
        self.top_logprobs = Some(value);
        self
    }

    /// Sets the response format.
    pub fn response_format(mut self, value: ResponseFormat) -> Self {
        self.response_format = Some(value);
        self
    }

    /// Sets the sampling seed.
    pub fn seed(mut self, value: i64) -> Self {
        self.seed = Some(value);
        self
    }

    /// Sets the tools the model may call.
    pub fn tools(mut self, value: impl IntoIterator<Item = Tool>) -> Self {
        self.tools = Some(value.into_iter().collect());
        self
    }

    /// Sets the tool choice.
    pub fn tool_choice(mut self, value: impl Into<ToolChoice>) -> Self {
        self.tool_choice = Some(value.into());
        self
    }

    /// Sets whether the model may call multiple tools in parallel.
    pub fn parallel_tool_calls(mut self, value: bool) -> Self {
        self.parallel_tool_calls = Some(value);
        self
    }

    /// Sets the reasoning effort.
    pub fn reasoning_effort(mut self, value: ReasoningEffort) -> Self {
        self.reasoning_effort = Some(value);
        self
    }

    /// Sets the output modalities.
    pub fn modalities(mut self, value: impl IntoIterator<Item = Modality>) -> Self {
        self.modalities = Some(value.into_iter().collect());
        self
    }

    /// Sets the audio output configuration.
    pub fn audio(mut self, value: AudioOutput) -> Self {
        self.audio = Some(value);
        self
    }

    /// Sets the static content prediction.
    pub fn prediction(mut self, value: Value) -> Self {
        self.prediction = Some(value);
        self
    }

    /// Sets whether to store the output for distillation or evals.
    pub fn store(mut self, value: bool) -> Self {
        self.store = Some(value);
        self
    }

    /// Sets the metadata key/value pairs.
    pub fn metadata(mut self, value: HashMap<String, String>) -> Self {
        self.metadata = Some(value);
        self
    }

    /// Sets the service tier.
    pub fn service_tier(mut self, value: ServiceTier) -> Self {
        self.service_tier = Some(value);
        self
    }

    /// Sets the safety identifier.
    pub fn safety_identifier(mut self, value: impl Into<String>) -> Self {
        self.safety_identifier = Some(value.into());
        self
    }

    /// Sets the built-in web search options.
    pub fn web_search_options(mut self, value: Value) -> Self {
        self.web_search_options = Some(value);
        self
    }

    /// Sets the end-user identifier.
    #[deprecated(note = "deprecated by OpenAI; use `safety_identifier` instead")]
    pub fn user(mut self, value: impl Into<String>) -> Self {
        self.user = Some(value.into());
        self
    }

    /// Sets legacy function definitions.
    #[deprecated(note = "deprecated by OpenAI; use `tools` instead")]
    pub fn functions(mut self, value: impl IntoIterator<Item = FunctionDefinition>) -> Self {
        self.functions = Some(value.into_iter().collect());
        self
    }

    /// Sets the legacy function call control.
    #[deprecated(note = "deprecated by OpenAI; use `tool_choice` instead")]
    pub fn function_call(mut self, value: Value) -> Self {
        self.function_call = Some(value);
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
