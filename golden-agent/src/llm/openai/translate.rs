use crate::llm::chat::{
    ChatRequest, ChatResponse, Content, ContentPart, FinishReason, Message, Role, ToolCall,
    ToolChoice, Usage,
};
use crate::tool::FromToolSpec;

use super::{
    ChatRequest as WireRequest, ChatResponse as WireResponse, Content as WireContent,
    ContentPart as WireContentPart, FunctionCall, ImageUrl, Message as WireMessage, NamedFunction,
    NamedToolChoice, ResponseMessage as WireResponseMessage, Stop, Tool as WireTool,
    ToolCall as WireToolCall, ToolChoice as WireToolChoice, ToolChoiceMode, ToolType,
    Usage as WireUsage,
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
    wire
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
        Role::Assistant => WireMessage::assistant(message.text())
            .tool_calls(message.tool_calls.iter().map(to_wire_tool_call)),
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
    }
}
