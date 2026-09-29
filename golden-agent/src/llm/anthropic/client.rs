use std::collections::{BTreeMap, VecDeque};

use async_trait::async_trait;
use bytes::Bytes;
use futures_core::Stream;
use futures_util::StreamExt;
use serde_json::Value;

use crate::error::{ApiErrorBody, Error};
use crate::llm::chat::{
    ChatEvent, ChatModel, ChatRequest, ChatResponse, ChatStream, PartialToolCall,
};
use crate::llm::http::HttpConfig;

use super::response::MessagesResponse;
use super::translate;

/// The official Anthropic API base URL.
pub const BASE_URL: &str = "https://api.anthropic.com";

/// The `anthropic-version` header value sent with every request.
pub const API_VERSION: &str = "2023-06-01";

/// The `max_tokens` used when a request does not set one. Anthropic requires it.
const DEFAULT_MAX_TOKENS: u32 = 4096;

/// The Anthropic provider, speaking the Messages API.
///
/// See <https://platform.claude.com/docs/en/api/messages>.
#[derive(Clone)]
pub struct AnthropicLlm {
    http: reqwest::Client,
    api_key: String,
    base_url: String,
    config: HttpConfig,
}

impl AnthropicLlm {
    /// Constructs a client with an explicit API key.
    pub fn new(api_key: impl Into<String>) -> Self {
        let config = HttpConfig::default();
        Self {
            http: config.build_client(),
            api_key: api_key.into(),
            base_url: BASE_URL.to_string(),
            config,
        }
    }

    /// Constructs a client from the `ANTHROPIC_API_KEY` environment variable.
    pub fn from_env() -> Self {
        let key = std::env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY must be set");
        Self::new(key)
    }

    /// Replaces the HTTP reliability settings, rebuilding the underlying client.
    pub fn with_http_config(mut self, config: HttpConfig) -> Self {
        self.http = config.build_client();
        self.config = config;
        self
    }

    /// Returns the Messages API endpoint.
    fn endpoint(&self) -> String {
        format!("{}/v1/messages", self.base_url.trim_end_matches('/'))
    }
}

#[async_trait]
impl ChatModel for AnthropicLlm {
    async fn chat(&self, request: &ChatRequest) -> Result<ChatResponse, Error> {
        let body = translate::build_request(request, false, DEFAULT_MAX_TOKENS);

        let response = self
            .config
            .bound(
                self.http
                    .post(self.endpoint())
                    .header("x-api-key", &self.api_key)
                    .header("anthropic-version", API_VERSION)
                    .json(&body),
            )
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.json::<ApiErrorBody>().await.ok();
            return Err(Error::api(status, body));
        }

        let response = response.json::<MessagesResponse>().await?;
        Ok(translate::into_chat_response(response))
    }

    fn chat_stream(&self, request: &ChatRequest) -> ChatStream<'_> {
        let body = translate::build_request(request, true, DEFAULT_MAX_TOKENS);

        let http = self.http.clone();
        let api_key = self.api_key.clone();
        let url = self.endpoint();

        Box::pin(async_stream::stream! {
            let response = match http
                .post(&url)
                .header("x-api-key", &api_key)
                .header("anthropic-version", API_VERSION)
                .json(&body)
                .send()
                .await
            {
                Ok(response) => response,
                Err(error) => {
                    yield Err(Error::Request(error));
                    return;
                }
            };

            let status = response.status();
            if !status.is_success() {
                let body = response.json::<ApiErrorBody>().await.ok();
                yield Err(Error::api(status, body));
                return;
            }

            let mut events = AnthropicSseStream {
                inner: response.bytes_stream(),
                buffer: String::new(),
                done: false,
                pending: VecDeque::new(),
                blocks: BTreeMap::new(),
            };

            while let Some(item) = events.next().await {
                yield item;
            }
        })
    }
}

/// A stream that turns Anthropic's SSE event stream into [`ChatEvent`]s.
///
/// Anthropic emits named server-sent events (`message_start`,
/// `content_block_start`, `content_block_delta`, `content_block_stop`,
/// `message_stop`, ...). Tool-use blocks are accumulated and surfaced as
/// complete [`ChatEvent::ToolCall`]s when the block closes.
struct AnthropicSseStream<S> {
    inner: S,
    buffer: String,
    done: bool,
    pending: VecDeque<ChatEvent>,
    blocks: BTreeMap<usize, PartialToolCall>,
}

impl<S> AnthropicSseStream<S> {
    /// Handles one SSE line, returning a fatal error if it carries one.
    fn handle_line(&mut self, line: &str) -> Option<Error> {
        let data = line.strip_prefix("data:")?.trim();
        if data.is_empty() {
            return None;
        }

        let value: Value = match serde_json::from_str(data) {
            Ok(value) => value,
            Err(error) => return Some(Error::Stream(error.to_string())),
        };

        match value.get("type").and_then(Value::as_str) {
            Some("content_block_start") => self.start_block(&value),
            Some("content_block_delta") => self.push_delta(&value),
            Some("content_block_stop") => self.stop_block(&value),
            Some("message_stop") => self.done = true,
            Some("error") => {
                let message = value
                    .pointer("/error/message")
                    .and_then(Value::as_str)
                    .unwrap_or("anthropic stream error");
                return Some(Error::Stream(message.to_string()));
            }
            _ => {}
        }

        None
    }

    /// Records a `tool_use` block that is about to stream its arguments.
    fn start_block(&mut self, value: &Value) {
        let block = value.get("content_block");
        if block
            .and_then(|block| block.get("type"))
            .and_then(Value::as_str)
            != Some("tool_use")
        {
            return;
        }

        let index = value.get("index").and_then(Value::as_u64).unwrap_or(0) as usize;
        let id = block
            .and_then(|block| block.get("id"))
            .and_then(Value::as_str)
            .unwrap_or_default();
        let name = block
            .and_then(|block| block.get("name"))
            .and_then(Value::as_str)
            .unwrap_or_default();

        self.blocks.insert(
            index,
            PartialToolCall {
                id: id.to_string(),
                name: name.to_string(),
                arguments: String::new(),
            },
        );
    }

    /// Pushes a text delta or appends to the open tool-use block's arguments.
    fn push_delta(&mut self, value: &Value) {
        let index = value.get("index").and_then(Value::as_u64).unwrap_or(0) as usize;
        let delta = value.get("delta");

        match delta
            .and_then(|delta| delta.get("type"))
            .and_then(Value::as_str)
        {
            Some("text_delta") => {
                let text = delta
                    .and_then(|delta| delta.get("text"))
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                if !text.is_empty() {
                    self.pending.push_back(ChatEvent::Text(text.to_string()));
                }
            }
            Some("input_json_delta") => {
                let partial = delta
                    .and_then(|delta| delta.get("partial_json"))
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                if let Some(block) = self.blocks.get_mut(&index) {
                    block.arguments.push_str(partial);
                }
            }
            _ => {}
        }
    }

    /// Completes a tool-use block into a pending tool call.
    fn stop_block(&mut self, value: &Value) {
        let index = value.get("index").and_then(Value::as_u64).unwrap_or(0) as usize;
        if let Some(block) = self.blocks.remove(&index) {
            self.pending
                .push_back(ChatEvent::ToolCall(block.into_tool_call()));
        }
    }
}

impl<S> Stream for AnthropicSseStream<S>
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Unpin,
{
    type Item = Result<ChatEvent, Error>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        use std::task::Poll;

        let this = self.get_mut();

        loop {
            if let Some(event) = this.pending.pop_front() {
                return Poll::Ready(Some(Ok(event)));
            }

            if this.done {
                return Poll::Ready(None);
            }

            if let Some(position) = this.buffer.find('\n') {
                let line: String = this.buffer.drain(..=position).collect();
                if let Some(error) = this.handle_line(line.trim()) {
                    this.done = true;
                    return Poll::Ready(Some(Err(error)));
                }
                continue;
            }

            match futures_util::ready!(this.inner.poll_next_unpin(cx)) {
                Some(Ok(bytes)) => this.buffer.push_str(&String::from_utf8_lossy(&bytes)),
                Some(Err(error)) => {
                    this.done = true;
                    return Poll::Ready(Some(Err(Error::Request(error))));
                }
                None => {
                    this.done = true;
                    continue;
                }
            }
        }
    }
}
