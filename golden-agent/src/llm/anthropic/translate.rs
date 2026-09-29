use serde_json::Value;

use crate::llm::chat::{
    ChatRequest, ChatResponse, Content, ContentPart, FinishReason, Message, Role, ToolCall,
    ToolChoice, Usage,
};
use crate::tool::FromToolSpec;

use super::request::{
    ContentBlock, ImageSource, InputMessage, MessagesRequest, Role as AnthropicRole,
};
use super::response::{MessagesResponse, OutputBlock};
use super::tool::{Tool, ToolChoice as AnthropicToolChoice};

/// Translates a neutral [`ChatRequest`] into a Messages API request.
///
/// `default_max_tokens` is used when the request does not set a token limit,
/// because the Messages API requires `max_tokens`.
pub(crate) fn build_request(
    request: &ChatRequest,
    stream: bool,
    default_max_tokens: u32,
) -> MessagesRequest {
    let (system, messages) = translate_messages(request);

    let tools = (!request.tools.is_empty())
        .then(|| request.tools.iter().map(Tool::from_tool_spec).collect());

    MessagesRequest {
        model: request.model.clone(),
        max_tokens: request.max_tokens.unwrap_or(default_max_tokens),
        messages,
        system,
        tools,
        tool_choice: request.tool_choice.as_ref().map(translate_tool_choice),
        temperature: request.temperature.map(clamp_temperature),
        top_p: request.top_p,
        stop_sequences: request.stop.clone(),
        stream: stream.then_some(true),
    }
}

/// Translates a Messages API response into a neutral [`ChatResponse`].
pub(crate) fn into_chat_response(response: MessagesResponse) -> ChatResponse {
    let mut text = String::new();
    let mut tool_calls: Vec<ToolCall> = Vec::new();

    for block in response.content {
        match block {
            OutputBlock::Text { text: value } => text.push_str(&value),
            OutputBlock::ToolUse { id, name, input } => tool_calls.push(ToolCall {
                id,
                name,
                arguments: input.to_string(),
            }),
            OutputBlock::Other => {}
        }
    }

    ChatResponse {
        id: response.id,
        model: response.model,
        message: Message {
            role: Role::Assistant,
            content: Content::Text(text),
            name: None,
            tool_calls,
            tool_call_id: None,
        },
        finish_reason: response.stop_reason.as_deref().map(parse_finish_reason),
        usage: Some(Usage {
            prompt_tokens: response.usage.input_tokens,
            completion_tokens: response.usage.output_tokens,
            total_tokens: response.usage.input_tokens + response.usage.output_tokens,
        }),
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
                content: vec![ContentBlock::ToolResult {
                    tool_use_id: message.tool_call_id.clone().unwrap_or_default(),
                    content: message.content.text().to_string(),
                }],
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
            .map(|part| match part {
                ContentPart::Text { text } => ContentBlock::Text { text: text.clone() },
                ContentPart::ImageUrl { url } => ContentBlock::Image {
                    source: ImageSource::Url { url: url.clone() },
                },
            })
            .collect(),
    }
}

/// Builds the content blocks of an assistant turn: text plus any tool calls.
fn assistant_content(message: &Message) -> Vec<ContentBlock> {
    let mut blocks = Vec::new();

    let text = message.content.text();
    if !text.is_empty() {
        blocks.push(ContentBlock::Text {
            text: text.to_string(),
        });
    }

    for call in &message.tool_calls {
        blocks.push(ContentBlock::ToolUse {
            id: call.id.clone(),
            name: call.name.clone(),
            input: serde_json::from_str(&call.arguments).unwrap_or(Value::Null),
        });
    }

    blocks
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

/// Anthropic's temperature range is 0.0 to 1.0, narrower than OpenAI's.
fn clamp_temperature(value: f32) -> f32 {
    value.clamp(0.0, 1.0)
}
