use serde_json::Value;

use crate::error::Error;
use crate::llm::chat::{
    ChatRequest, ChatResponse, Content, ContentPart, FinishReason, InferenceSpeed, Message,
    Reasoning, ReasoningEffort, Role, ToolCall, ToolChoice, Usage,
};
use crate::tool::FromToolSpec;

use super::request::{
    ContentBlock, Effort, ImageSource, InputMessage, MessagesRequest, OutputConfig,
    Role as AnthropicRole, Speed, Thinking,
};
use super::response::{MessagesResponse, OutputBlock};
use super::tool::{Tool, ToolChoice as AnthropicToolChoice};

/// Anthropic's minimum extended-thinking budget.
const MIN_THINKING_BUDGET: u32 = 1024;

/// Translates a neutral [`ChatRequest`] into a Messages API request.
///
/// `default_max_tokens` is used when the request does not set a token limit,
/// because the Messages API requires `max_tokens`.
pub(crate) fn build_request(
    request: &ChatRequest,
    stream: bool,
    default_max_tokens: u32,
) -> Result<MessagesRequest, Error> {
    if request.temperature.is_some() || request.top_p.is_some() {
        return Err(Error::InvalidRequest {
            message: "Anthropic API version 2026-10-21 does not support `temperature` or `top_p`"
                .to_string(),
        });
    }

    let max_tokens = request.max_tokens.unwrap_or(default_max_tokens);
    let thinking = match request.reasoning {
        Some(Reasoning::Disabled) => Some(Thinking::Disabled),
        Some(Reasoning::Enabled | Reasoning::Effort(_)) => Some(Thinking::Adaptive),
        Some(Reasoning::Budget(budget_tokens)) => {
            if budget_tokens < MIN_THINKING_BUDGET || budget_tokens >= max_tokens {
                return Err(Error::InvalidRequest {
                    message: format!(
                        "Anthropic manual thinking budget must be at least {MIN_THINKING_BUDGET} and less than max_tokens ({max_tokens})"
                    ),
                });
            }
            Some(Thinking::Enabled { budget_tokens })
        }
        None => None,
    };

    let (system, messages) = translate_messages(request);

    let tools = (!request.tools.is_empty())
        .then(|| request.tools.iter().map(Tool::from_tool_spec).collect());

    Ok(MessagesRequest {
        model: request.model.clone(),
        max_tokens,
        messages,
        system,
        tools,
        tool_choice: request.tool_choice.as_ref().map(translate_tool_choice),
        stop_sequences: request.stop.clone(),
        stream: stream.then_some(true),
        speed: (request.inference_speed == Some(InferenceSpeed::Fast)).then_some(Speed::Fast),
        thinking,
        output_config: match request.reasoning {
            Some(Reasoning::Effort(effort)) => Some(OutputConfig {
                effort: Some(to_effort(effort)),
            }),
            _ => None,
        },
    })
}

/// Maps a neutral effort level onto `output_config.effort`'s scale.
fn to_effort(effort: ReasoningEffort) -> Effort {
    match effort {
        ReasoningEffort::Low => Effort::Low,
        ReasoningEffort::Medium => Effort::Medium,
        ReasoningEffort::High => Effort::High,
        ReasoningEffort::Max => Effort::Max,
    }
}

/// Translates a Messages API response into a neutral [`ChatResponse`].
pub(crate) fn into_chat_response(response: MessagesResponse) -> ChatResponse {
    let inference_speed = response
        .usage
        .speed
        .clone()
        .map(InferenceSpeed::from_provider);
    let parts: Vec<ContentPart> = response
        .content
        .into_iter()
        .filter_map(|block| match block {
            OutputBlock::Text { text } => Some(ContentPart::Text { text }),
            OutputBlock::Thinking {
                thinking,
                signature,
            } => Some(ContentPart::Reasoning {
                text: thinking,
                signature: (!signature.is_empty()).then_some(signature),
            }),
            OutputBlock::RedactedThinking { data } => Some(ContentPart::OpaqueReasoning { data }),
            OutputBlock::ToolUse { id, name, input } => Some(ContentPart::ToolCall(ToolCall {
                id,
                name,
                arguments: input.to_string(),
            })),
            OutputBlock::Other => None,
        })
        .collect();

    let reasoning_tokens = response
        .usage
        .output_tokens_details
        .as_ref()
        .map(|details| details.thinking_tokens);

    ChatResponse {
        id: response.id,
        model: response.model,
        message: Message::parts(Role::Assistant, parts),
        finish_reason: response.stop_reason.as_deref().map(parse_finish_reason),
        usage: Some(Usage {
            prompt_tokens: response.usage.input_tokens,
            completion_tokens: response.usage.output_tokens,
            total_tokens: response.usage.input_tokens + response.usage.output_tokens,
            reasoning_tokens,
        }),
        inference_speed,
    }
}

/// Splits neutral messages into a top-level system prompt and Anthropic turns.
fn translate_messages(request: &ChatRequest) -> (Option<String>, Vec<InputMessage>) {
    let mut system_lines: Vec<String> = Vec::new();
    let mut messages: Vec<InputMessage> = Vec::new();

    for message in &request.messages {
        match message.role {
            Role::System => {
                if let Some(text) = message.content.as_text() {
                    system_lines.push(text.to_string());
                }
            }
            Role::User => messages.push(InputMessage {
                role: AnthropicRole::User,
                content: user_content(&message.content),
            }),
            Role::Assistant => {
                let content = assistant_content(message);
                if !content.is_empty() {
                    messages.push(InputMessage {
                        role: AnthropicRole::Assistant,
                        content,
                    });
                }
            }
            Role::Tool => messages.push(InputMessage {
                role: AnthropicRole::User,
                content: tool_result_content(message),
            }),
        }
    }

    let system = (!system_lines.is_empty()).then(|| system_lines.join("\n"));
    (system, messages)
}

/// Builds the content blocks of a user turn.
fn user_content(content: &Content) -> Vec<ContentBlock> {
    match content {
        Content::Text(text) => vec![ContentBlock::Text { text: text.clone() }],
        Content::Parts(parts) => parts
            .iter()
            .filter_map(|part| match part {
                ContentPart::Text { text } => Some(ContentBlock::Text { text: text.clone() }),
                ContentPart::ImageUrl { url } => Some(ContentBlock::Image {
                    source: ImageSource::Url { url: url.clone() },
                }),
                ContentPart::ToolResult {
                    tool_call_id,
                    content,
                    is_error,
                } => Some(ContentBlock::ToolResult {
                    tool_use_id: tool_call_id.clone(),
                    content: content.clone(),
                    is_error: is_error.then_some(true),
                }),
                ContentPart::Reasoning { .. }
                | ContentPart::OpaqueReasoning { .. }
                | ContentPart::ToolCall(_) => None,
            })
            .collect(),
    }
}

/// Builds the content blocks of an assistant turn: the signed thinking block
/// first (Anthropic requires thinking to lead the turn it produced), then the
/// text and any tool calls.
///
/// Ordered content parts are replayed exactly. Legacy flattened fields are used
/// only for messages created before ordered block support.
fn assistant_content(message: &Message) -> Vec<ContentBlock> {
    if let Content::Parts(parts) = &message.content {
        let blocks: Vec<_> = parts
            .iter()
            .filter_map(|part| match part {
                ContentPart::Text { text } => Some(ContentBlock::Text { text: text.clone() }),
                ContentPart::Reasoning { text, signature } => {
                    signature.as_ref().map(|signature| ContentBlock::Thinking {
                        thinking: text.clone(),
                        signature: signature.clone(),
                    })
                }
                ContentPart::OpaqueReasoning { data } => {
                    Some(ContentBlock::RedactedThinking { data: data.clone() })
                }
                ContentPart::ToolCall(call) => Some(tool_use_block(call)),
                ContentPart::ImageUrl { .. } | ContentPart::ToolResult { .. } => None,
            })
            .collect();
        if !blocks.is_empty() {
            return blocks;
        }
    }

    let mut blocks = Vec::new();
    if let (Some(thinking), Some(signature)) = (&message.reasoning, &message.reasoning_signature) {
        blocks.push(ContentBlock::Thinking {
            thinking: thinking.clone(),
            signature: signature.clone(),
        });
    }
    if !message.content.text().is_empty() {
        blocks.push(ContentBlock::Text {
            text: message.content.text().to_string(),
        });
    }
    blocks.extend(message.tool_calls.iter().map(tool_use_block));
    blocks
}

fn tool_use_block(call: &ToolCall) -> ContentBlock {
    ContentBlock::ToolUse {
        id: call.id.clone(),
        name: call.name.clone(),
        input: serde_json::from_str(&call.arguments).unwrap_or(Value::Null),
    }
}

fn tool_result_content(message: &Message) -> Vec<ContentBlock> {
    if let Content::Parts(parts) = &message.content {
        let blocks: Vec<_> = parts
            .iter()
            .filter_map(|part| match part {
                ContentPart::ToolResult {
                    tool_call_id,
                    content,
                    is_error,
                } => Some(ContentBlock::ToolResult {
                    tool_use_id: tool_call_id.clone(),
                    content: content.clone(),
                    is_error: is_error.then_some(true),
                }),
                _ => None,
            })
            .collect();
        if !blocks.is_empty() {
            return blocks;
        }
    }

    vec![ContentBlock::ToolResult {
        tool_use_id: message.tool_call_id.clone().unwrap_or_default(),
        content: message.content.text().to_string(),
        is_error: None,
    }]
}

/// Translates a neutral tool choice into Anthropic's.
fn translate_tool_choice(choice: &ToolChoice) -> AnthropicToolChoice {
    match choice {
        ToolChoice::None => AnthropicToolChoice::None,
        ToolChoice::Auto => AnthropicToolChoice::Auto,
        ToolChoice::Required => AnthropicToolChoice::Any,
        ToolChoice::Tool(name) => AnthropicToolChoice::Tool { name: name.clone() },
    }
}

fn parse_finish_reason(reason: &str) -> FinishReason {
    match reason {
        "end_turn" | "stop_sequence" | "pause_turn" => FinishReason::Stop,
        "max_tokens" | "model_context_window_exceeded" => FinishReason::Length,
        "tool_use" => FinishReason::ToolCalls,
        "refusal" => FinishReason::ContentFilter,
        other => FinishReason::Other(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::chat::{InferenceSpeed, Message, Reasoning, ReasoningEffort};

    fn body(reasoning: Option<Reasoning>) -> serde_json::Value {
        let mut request = ChatRequest::new("claude").add_message(Message::user("hi"));
        request.reasoning = reasoning;
        request_body(&request)
    }

    fn request_body(request: &ChatRequest) -> serde_json::Value {
        serde_json::to_value(build_request(request, false, 4096).unwrap()).unwrap()
    }

    #[test]
    fn fast_mode_maps_to_speed_and_is_independent_of_thinking() {
        let request = ChatRequest::new("claude")
            .add_message(Message::user("hi"))
            .max_tokens(16_000)
            .inference_speed(InferenceSpeed::Fast)
            .reasoning(Reasoning::Budget(8_000));
        let body = request_body(&request);
        assert_eq!(body["speed"], "fast");
        assert_eq!(body["thinking"]["budget_tokens"], 8_000);
    }

    #[test]
    fn speed_unset_sends_no_speed_field() {
        assert!(body(None).get("speed").is_none());
    }

    #[test]
    fn response_preserves_actual_speed() {
        let wire: MessagesResponse = serde_json::from_value(serde_json::json!({
            "id": "message-1",
            "model": "claude",
            "content": [],
            "usage": {
                "input_tokens": 1,
                "output_tokens": 2,
                "output_tokens_details": {"thinking_tokens": 1},
                "speed": "fast"
            }
        }))
        .unwrap();

        let response = into_chat_response(wire);
        assert_eq!(response.inference_speed, Some(InferenceSpeed::Fast));
        assert_eq!(response.usage.unwrap().reasoning_tokens, Some(1));
    }

    #[test]
    fn reasoning_unset_sends_no_thinking_fields() {
        let body = body(None);
        assert!(body.get("thinking").is_none());
        assert!(body.get("output_config").is_none());
    }

    #[test]
    fn disabled_sends_switched_off_thinking() {
        let body = body(Some(Reasoning::Disabled));
        assert_eq!(body["thinking"], serde_json::json!({"type": "disabled"}));
        assert!(body.get("output_config").is_none());
    }

    #[test]
    fn effort_goes_to_output_config() {
        let cases = [
            (ReasoningEffort::Low, "low"),
            (ReasoningEffort::Medium, "medium"),
            (ReasoningEffort::High, "high"),
            (ReasoningEffort::Max, "max"),
        ];
        for (effort, expected) in cases {
            let body = body(Some(Reasoning::Effort(effort)));
            assert_eq!(body["thinking"]["type"], "adaptive");
            assert_eq!(
                body["output_config"],
                serde_json::json!({ "effort": expected })
            );
        }
    }

    #[test]
    fn budget_sets_thinking_budget_tokens() {
        let body = body(Some(Reasoning::Budget(2_000)));
        assert_eq!(body["thinking"]["budget_tokens"], 2_000);
        assert!(body.get("output_config").is_none());
    }

    #[test]
    fn enabled_uses_adaptive_thinking() {
        let body = body(Some(Reasoning::Enabled));
        assert_eq!(body["thinking"], serde_json::json!({"type": "adaptive"}));
        assert!(body.get("output_config").is_none());
    }

    #[test]
    fn removed_sampling_parameters_are_rejected_locally() {
        let request = ChatRequest::new("claude")
            .add_message(Message::user("hi"))
            .temperature(0.5);
        let error = build_request(&request, false, 4096).unwrap_err();
        assert!(matches!(error, Error::InvalidRequest { .. }));
    }

    #[test]
    fn manual_thinking_budget_must_be_below_max_tokens() {
        let request = ChatRequest::new("claude")
            .add_message(Message::user("hi"))
            .max_tokens(2_000)
            .reasoning(Reasoning::Budget(2_000));
        let error = build_request(&request, false, 4096).unwrap_err();
        assert!(matches!(error, Error::InvalidRequest { .. }));
    }

    #[test]
    fn response_blocks_round_trip_in_original_order() {
        let wire: MessagesResponse = serde_json::from_value(serde_json::json!({
            "id": "message-1",
            "model": "claude",
            "content": [
                {"type": "thinking", "thinking": "first", "signature": "sig-1"},
                {"type": "tool_use", "id": "tool-1", "name": "lookup", "input": {"q": 1}},
                {"type": "redacted_thinking", "data": "encrypted"},
                {"type": "thinking", "thinking": "second", "signature": "sig-2"},
                {"type": "text", "text": "done"}
            ],
            "usage": {"input_tokens": 1, "output_tokens": 2}
        }))
        .unwrap();

        let response = into_chat_response(wire);
        let request = ChatRequest::new("claude")
            .add_message(Message::user("start"))
            .add_message(response.message);
        let body = request_body(&request);

        assert_eq!(body["messages"][1]["content"][0]["type"], "thinking");
        assert_eq!(body["messages"][1]["content"][0]["signature"], "sig-1");
        assert_eq!(body["messages"][1]["content"][1]["type"], "tool_use");
        assert_eq!(
            body["messages"][1]["content"][2]["type"],
            "redacted_thinking"
        );
        assert_eq!(body["messages"][1]["content"][3]["signature"], "sig-2");
        assert_eq!(body["messages"][1]["content"][4]["text"], "done");
    }

    #[test]
    fn failed_tool_result_sets_is_error() {
        let request = ChatRequest::new("claude")
            .add_message(Message::user("start"))
            .add_message(Message::tool_error("tool-1", "failed"));
        let body = request_body(&request);

        assert_eq!(body["messages"][1]["content"][0]["type"], "tool_result");
        assert_eq!(body["messages"][1]["content"][0]["is_error"], true);
    }
}
