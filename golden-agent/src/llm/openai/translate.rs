use crate::llm::chat::{
    ChatRequest, ChatResponse, Content, ContentPart, FinishReason, Message, Reasoning,
    ReasoningEffort, ResponseFormat, Role, ToolCall, ToolChoice, Usage,
};
use crate::tool::FromToolSpec;

use super::{
    ChatRequest as WireRequest, ChatResponse as WireResponse, Content as WireContent,
    ContentPart as WireContentPart, Effort as WireEffort, FunctionCall, ImageUrl,
    JsonSchema as WireJsonSchema, Message as WireMessage, NamedFunction, NamedToolChoice,
    ResponseFormat as WireResponseFormat, ResponseMessage as WireResponseMessage, Stop,
    Thinking as WireThinking, ThinkingMode, Tool as WireTool, ToolCall as WireToolCall,
    ToolChoice as WireToolChoice, ToolChoiceMode, ToolType, Usage as WireUsage,
};

/// Translates a neutral [`ChatRequest`] into an OpenAI wire request.
pub(crate) fn to_wire_request(request: &ChatRequest, stream: bool) -> WireRequest {
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
    let Some(choice) = response.choices.into_iter().next() else {
        return ChatResponse {
            id: response.id,
            model: response.model,
            message: Message::new(Role::Assistant, Content::Text(String::new())),
            finish_reason: None,
            usage: response.usage.map(from_wire_usage),
        };
    };

    ChatResponse {
        id: response.id,
        model: response.model,
        message: from_wire_message(choice.message),
        finish_reason: choice.finish_reason.map(parse_finish_reason),
        usage: response.usage.map(from_wire_usage),
    }
}

fn to_wire_message(message: &Message) -> WireMessage {
    let mut wire = match message.role {
        Role::System => WireMessage::system(message.text()),
        Role::User => WireMessage::user(to_wire_content(&message.content)),
        Role::Assistant => {
            let mut wire = WireMessage::assistant(message.text());
            if !message.tool_calls.is_empty() {
                wire = wire.tool_calls(message.tool_calls.iter().map(to_wire_tool_call));
            }
            if let Some(reasoning) = &message.reasoning {
                wire = wire.reasoning_content(reasoning.clone());
            }
            wire
        }
        Role::Tool => WireMessage::tool(
            message.tool_call_id.clone().unwrap_or_default(),
            message.text(),
        ),
    };

    if let Some(name) = &message.name {
        wire = wire.name(name.clone());
    }

    wire
}

fn to_wire_content(content: &Content) -> WireContent {
    match content {
        Content::Text(text) => WireContent::Text(text.clone()),
        Content::Parts(parts) => WireContent::Parts(
            parts
                .iter()
                .map(|part| match part {
                    ContentPart::Text { text } => WireContentPart::Text { text: text.clone() },
                    ContentPart::ImageUrl { url } => WireContentPart::ImageUrl {
                        image_url: ImageUrl {
                            url: url.clone(),
                            detail: None,
                        },
                    },
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
    Message {
        role: Role::Assistant,
        content: Content::Text(message.content().to_string()),
        name: None,
        tool_calls: message
            .tool_calls
            .unwrap_or_default()
            .into_iter()
            .map(from_wire_tool_call)
            .collect(),
        tool_call_id: None,
        reasoning: message.reasoning_content,
        reasoning_signature: None,
        id: None,
        run_id: None,
        is_context: true,
        created_at: None,
    }
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

fn from_wire_usage(usage: WireUsage) -> Usage {
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
    use crate::llm::chat::{Message, Reasoning, ReasoningEffort};

    /// Serializes the wire body the API would receive for `request`.
    fn body(request: &ChatRequest) -> serde_json::Value {
        serde_json::to_value(to_wire_request(request, false)).unwrap()
    }

    fn request() -> ChatRequest {
        ChatRequest::new("deepseek-flash").add_message(Message::user("hi"))
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
