use crate::llm::chat::{
    ChatRequest, ChatResponse, Content, ContentPart, FinishReason, InferenceSpeed, Message,
    Reasoning, ReasoningEffort, ResponseFormat, Role, ToolCall, ToolChoice, Usage,
};
use crate::tool::FromToolSpec;

use super::{
    ChatRequest as WireRequest, ChatResponse as WireResponse, Content as WireContent,
    ContentPart as WireContentPart, Effort as WireEffort, FunctionCall, ImageUrl,
    JsonSchema as WireJsonSchema, Message as WireMessage, NamedFunction, NamedToolChoice,
    ResponseFormat as WireResponseFormat, ResponseMessage as WireResponseMessage, ServiceTier,
    Stop, Thinking as WireThinking, ThinkingMode, Tool as WireTool, ToolCall as WireToolCall,
    ToolChoice as WireToolChoice, ToolChoiceMode, ToolType, Usage as WireUsage,
};

/// Translates a neutral [`ChatRequest`] into an OpenAI wire request.
pub(crate) fn to_wire_request(
    request: &ChatRequest,
    stream: bool,
    supports_fast_mode: bool,
) -> WireRequest {
    let mut wire = WireRequest::new(request.model.clone());
    wire.stream = Some(stream);
    wire.messages = request.messages.iter().map(to_wire_message).collect();
    wire.tools = (!request.tools.is_empty())
        .then(|| request.tools.iter().map(WireTool::from_tool_spec).collect());
    wire.tool_choice = request.tool_choice.as_ref().map(to_wire_tool_choice);
    wire.temperature = request.temperature;
    wire.top_p = request.top_p;
    wire.max_completion_tokens = request.max_tokens;
    wire.stop = request
        .stop
        .as_ref()
        .map(|sequences| Stop::Multiple(sequences.clone()));
    wire.response_format = request
        .response_format
        .as_ref()
        .map(to_wire_response_format);
    to_wire_reasoning(request.reasoning, &mut wire);
    if supports_fast_mode && request.inference_speed == Some(InferenceSpeed::Fast) {
        wire.service_tier = Some(ServiceTier::Fast);
    }
    wire
}

/// Maps the neutral reasoning setting onto the OpenAI wire format.
///
/// DeepSeek (and OpenAI-compatible reasoning providers) take an on/off
/// `thinking` object plus a `reasoning_effort` level. `thinking` wins over
/// `reasoning_effort` when both are present, so a disabled setting only sends
/// the object and an enabled one sends the matching effort. Leaving
/// [`ChatRequest::reasoning`] unset sends neither, which the API treats as
/// thinking on.
fn to_wire_reasoning(reasoning: Option<Reasoning>, wire: &mut WireRequest) {
    let Some(reasoning) = reasoning else {
        return;
    };

    let mode = match reasoning {
        Reasoning::Disabled => ThinkingMode::Disabled,
        _ => ThinkingMode::Enabled,
    };
    wire.thinking = Some(WireThinking { mode });

    if let Some(effort) = reasoning.effort() {
        wire.reasoning_effort = Some(to_wire_effort(effort));
    }
}

/// Maps a neutral effort level onto the wire format's documented scale.
///
/// See [`Effort`]'s `From` implementation for the DeepSeek fold
/// (`minimal` / `low` → `low`, `medium` / `xhigh` → `high`).
fn to_wire_effort(effort: ReasoningEffort) -> WireEffort {
    WireEffort::from(effort)
}

/// Translates a neutral [`ResponseFormat`] into the OpenAI wire format.
fn to_wire_response_format(format: &ResponseFormat) -> WireResponseFormat {
    match format {
        ResponseFormat::Text => WireResponseFormat::Text,
        ResponseFormat::JsonObject => WireResponseFormat::JsonObject,
        ResponseFormat::JsonSchema {
            name,
            schema,
            strict,
        } => WireResponseFormat::JsonSchema {
            json_schema: WireJsonSchema {
                name: name.clone(),
                description: None,
                schema: Some(schema.clone()),
                strict: Some(*strict),
            },
        },
    }
}

/// Translates an OpenAI wire response into a neutral [`ChatResponse`].
pub(crate) fn from_wire_response(response: WireResponse) -> ChatResponse {
    let inference_speed = response.service_tier.map(InferenceSpeed::from_provider);
    let Some(choice) = response.choices.into_iter().next() else {
        return ChatResponse {
            id: response.id,
            model: response.model,
            message: Message::new(Role::Assistant, Content::Text(String::new())),
            finish_reason: None,
            usage: response.usage.map(from_wire_usage),
            inference_speed,
        };
    };

    ChatResponse {
        id: response.id,
        model: response.model,
        message: from_wire_message(choice.message),
        finish_reason: choice.finish_reason.map(parse_finish_reason),
        usage: response.usage.map(from_wire_usage),
        inference_speed,
    }
}

fn to_wire_message(message: &Message) -> WireMessage {
    let mut wire = match message.role {
        Role::System => WireMessage::system(message.text()),
        Role::User => WireMessage::user(to_wire_content(&message.content)),
        Role::Assistant => {
            let mut wire = WireMessage::assistant(flatten_text_parts(message));
            let part_calls: Vec<_> = message
                .content
                .parts()
                .into_iter()
                .flatten()
                .filter_map(|part| match part {
                    ContentPart::ToolCall(call) => Some(call),
                    _ => None,
                })
                .collect();
            if part_calls.is_empty() {
                if !message.tool_calls.is_empty() {
                    wire = wire.tool_calls(message.tool_calls.iter().map(to_wire_tool_call));
                }
            } else {
                wire = wire.tool_calls(part_calls.into_iter().map(to_wire_tool_call));
            }
            if let Some(reasoning) = flatten_reasoning_parts(message) {
                wire = wire.reasoning_content(reasoning);
            }
            wire
        }
        Role::Tool => {
            let (tool_call_id, content) = first_tool_result(message).unwrap_or_else(|| {
                (
                    message.tool_call_id.clone().unwrap_or_default(),
                    message.text().to_string(),
                )
            });
            WireMessage::tool(tool_call_id, content)
        }
    };

    if let Some(name) = &message.name {
        wire = wire.name(name.clone());
    }

    wire
}

fn first_tool_result(message: &Message) -> Option<(String, String)> {
    message.content.parts()?.iter().find_map(|part| match part {
        ContentPart::ToolResult {
            tool_call_id,
            content,
            ..
        } => Some((tool_call_id.clone(), content.clone())),
        _ => None,
    })
}

fn flatten_text_parts(message: &Message) -> String {
    match &message.content {
        Content::Text(text) => text.clone(),
        Content::Parts(parts) => parts
            .iter()
            .filter_map(|part| match part {
                ContentPart::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect(),
    }
}

fn flatten_reasoning_parts(message: &Message) -> Option<String> {
    match &message.content {
        Content::Parts(parts) => {
            let reasoning: String = parts
                .iter()
                .filter_map(|part| match part {
                    ContentPart::Reasoning { text, .. } => Some(text.as_str()),
                    _ => None,
                })
                .collect();
            (!reasoning.is_empty()).then_some(reasoning)
        }
        Content::Text(_) => message.reasoning.clone(),
    }
}

fn to_wire_content(content: &Content) -> WireContent {
    match content {
        Content::Text(text) => WireContent::Text(text.clone()),
        Content::Parts(parts) => WireContent::Parts(
            parts
                .iter()
                .filter_map(|part| match part {
                    ContentPart::Text { text } => {
                        Some(WireContentPart::Text { text: text.clone() })
                    }
                    ContentPart::ImageUrl { url } => Some(WireContentPart::ImageUrl {
                        image_url: ImageUrl {
                            url: url.clone(),
                            detail: None,
                        },
                    }),
                    ContentPart::Reasoning { .. }
                    | ContentPart::OpaqueReasoning { .. }
                    | ContentPart::ToolCall(_)
                    | ContentPart::ToolResult { .. } => None,
                })
                .collect(),
        ),
    }
}

fn to_wire_tool_call(call: &ToolCall) -> WireToolCall {
    WireToolCall {
        id: call.id.clone(),
        r#type: ToolType::Function,
        function: FunctionCall {
            name: call.name.clone(),
            arguments: call.arguments.clone(),
        },
    }
}

fn to_wire_tool_choice(choice: &ToolChoice) -> WireToolChoice {
    match choice {
        ToolChoice::None => WireToolChoice::Mode(ToolChoiceMode::None),
        ToolChoice::Auto => WireToolChoice::Mode(ToolChoiceMode::Auto),
        ToolChoice::Required => WireToolChoice::Mode(ToolChoiceMode::Required),
        ToolChoice::Tool(name) => WireToolChoice::Named(NamedToolChoice {
            r#type: ToolType::Function,
            function: NamedFunction { name: name.clone() },
        }),
    }
}

fn from_wire_message(message: WireResponseMessage) -> Message {
    let mut parts = Vec::new();
    if let Some(reasoning) = message.reasoning_content {
        parts.push(ContentPart::Reasoning {
            text: reasoning,
            signature: None,
        });
    }
    if let Some(content) = message.content
        && !content.is_empty()
    {
        parts.push(ContentPart::Text { text: content });
    }
    parts.extend(
        message
            .tool_calls
            .unwrap_or_default()
            .into_iter()
            .map(from_wire_tool_call)
            .map(ContentPart::ToolCall),
    );
    Message::parts(Role::Assistant, parts)
}

fn from_wire_tool_call(call: WireToolCall) -> ToolCall {
    ToolCall {
        id: call.id,
        name: call.function.name,
        arguments: call.function.arguments,
    }
}

fn parse_finish_reason(reason: String) -> FinishReason {
    match reason.as_str() {
        "stop" => FinishReason::Stop,
        "length" => FinishReason::Length,
        "tool_calls" => FinishReason::ToolCalls,
        "content_filter" => FinishReason::ContentFilter,
        _ => FinishReason::Other(reason),
    }
}

pub(super) fn from_wire_usage(usage: WireUsage) -> Usage {
    Usage {
        prompt_tokens: usage.prompt_tokens,
        completion_tokens: usage.completion_tokens,
        total_tokens: usage.total_tokens,
        reasoning_tokens: usage
            .completion_tokens_details
            .and_then(|details| details.reasoning_tokens),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::chat::{InferenceSpeed, Message, Reasoning, ReasoningEffort};

    /// Serializes the wire body the API would receive for `request`.
    fn body(request: &ChatRequest) -> serde_json::Value {
        body_with_fast_mode_support(request, false)
    }

    fn body_with_fast_mode_support(
        request: &ChatRequest,
        supports_fast_mode: bool,
    ) -> serde_json::Value {
        serde_json::to_value(to_wire_request(request, false, supports_fast_mode)).unwrap()
    }

    fn request() -> ChatRequest {
        ChatRequest::new("deepseek-flash").add_message(Message::user("hi"))
    }

    #[test]
    fn existing_priority_service_tier_remains_supported() {
        let wire = WireRequest::new("gpt-5").service_tier(ServiceTier::Priority);
        let body = serde_json::to_value(wire).unwrap();
        assert_eq!(body["service_tier"], "priority");
    }

    #[test]
    fn fast_mode_maps_to_fast_service_tier_for_openai() {
        let request = request().inference_speed(InferenceSpeed::Fast);
        let body = body_with_fast_mode_support(&request, true);
        assert_eq!(body["service_tier"], "fast");
    }

    #[test]
    fn fast_mode_is_omitted_for_openai_compatible_providers() {
        let body = body(&request().inference_speed(InferenceSpeed::Fast));
        assert!(body.get("service_tier").is_none());
    }

    #[test]
    fn fast_mode_and_reasoning_are_independent() {
        let request = request()
            .inference_speed(InferenceSpeed::Fast)
            .reasoning(Reasoning::Effort(ReasoningEffort::High));
        let body = body_with_fast_mode_support(&request, true);
        assert_eq!(body["service_tier"], "fast");
        assert_eq!(body["reasoning_effort"], "high");
        assert_eq!(body["thinking"], serde_json::json!({"type": "enabled"}));
    }

    #[test]
    fn reasoning_unset_sends_neither_field() {
        let body = body(&request());
        assert!(body.get("thinking").is_none());
        assert!(body.get("reasoning_effort").is_none());
    }

    #[test]
    fn disabled_thinking_sends_only_the_switch() {
        let body = body(&request().reasoning(Reasoning::Disabled));
        assert_eq!(body["thinking"], serde_json::json!({"type": "disabled"}));
        assert!(body.get("reasoning_effort").is_none());
    }

    #[test]
    fn enabled_thinking_uses_the_default_high_effort() {
        let body = body(&request().reasoning(Reasoning::Enabled));
        assert_eq!(body["thinking"], serde_json::json!({"type": "enabled"}));
        assert_eq!(body["reasoning_effort"], "high");
    }

    #[test]
    fn effort_levels_fold_onto_deepseek_documented_scale() {
        // https://api-docs.deepseek.com/zh-cn/guides/thinking_mode:
        // minimal / low -> low, medium / xhigh -> high, ultra -> max.
        let cases = [
            (ReasoningEffort::Low, "low"),
            (ReasoningEffort::Medium, "high"),
            (ReasoningEffort::High, "high"),
            (ReasoningEffort::Max, "max"),
        ];
        for (effort, expected) in cases {
            let body = body(&request().reasoning(Reasoning::Effort(effort)));
            assert_eq!(body["reasoning_effort"], expected, "effort {effort:?}");
            assert_eq!(body["thinking"], serde_json::json!({"type": "enabled"}));
        }
    }

    #[test]
    fn response_preserves_actual_service_tier() {
        let wire: WireResponse = serde_json::from_value(serde_json::json!({
            "id": "response-1",
            "object": "chat.completion",
            "created": 0,
            "model": "gpt-5",
            "choices": [],
            "service_tier": "priority"
        }))
        .unwrap();

        let response = from_wire_response(wire);
        assert_eq!(
            response.inference_speed,
            Some(InferenceSpeed::Other("priority".into()))
        );
    }

    #[test]
    fn assistant_reasoning_is_echoed_back_with_tool_calls() {
        let mut assistant = Message::assistant("").tool_calls([ToolCall {
            id: "call-1".into(),
            name: "get_weather".into(),
            arguments: "{}".into(),
        }]);
        assistant.reasoning = Some("thinking...".into());

        let body = body(&request().add_message(assistant));
        let sent = &body["messages"][1];
        assert_eq!(sent["reasoning_content"], "thinking...");
        assert_eq!(sent["tool_calls"][0]["id"], "call-1");
    }
}
