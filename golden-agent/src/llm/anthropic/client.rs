use async_trait::async_trait;
use bytes::Bytes;
use futures_core::Stream;
use futures_util::StreamExt;
use serde_json::Value;

use crate::error::{ApiErrorBody, Error};
use crate::llm::chat::{ChatModel, ChatRequest, ChatResponse, ChatStream};
use crate::llm::http::{self, HttpConfig};

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

        http::with_retry(&self.config, || async {
            let mut builder = self
                .http
                .post(self.endpoint())
                .header("x-api-key", &self.api_key)
                .header("anthropic-version", API_VERSION)
                .json(&body);
            if let Some(timeout) = self.config.timeout {
                builder = builder.timeout(timeout);
            }

            let response = builder.send().await?;

            let status = response.status();
            if !status.is_success() {
                let body = response.json::<ApiErrorBody>().await.ok();
                return Err(Error::api(status, body));
            }

            let response = response.json::<MessagesResponse>().await?;
            Ok(translate::into_chat_response(response))
        })
        .await
    }

    fn chat_stream(&self, request: &ChatRequest) -> ChatStream<'_> {
        let body = translate::build_request(request, true, DEFAULT_MAX_TOKENS);

        let http = self.http.clone();
        let api_key = self.api_key.clone();
        let url = self.endpoint();
        let config = self.config.clone();

        Box::pin(async_stream::stream! {
            let response = match http::with_retry(&config, || async {
                let response = http
                    .post(&url)
                    .header("x-api-key", &api_key)
                    .header("anthropic-version", API_VERSION)
                    .json(&body)
                    .send()
                    .await?;

                let status = response.status();
                if !status.is_success() {
                    let body = response.json::<ApiErrorBody>().await.ok();
                    return Err(Error::api(status, body));
                }

                Ok(response)
            })
            .await
            {
                Ok(response) => response,
                Err(error) => {
                    yield Err(error);
                    return;
                }
            };

            let mut events = AnthropicSseStream {
                inner: response.bytes_stream(),
                buffer: String::new(),
                done: false,
            };

            while let Some(item) = events.next().await {
                yield item;
            }
        })
    }
}

/// A stream that extracts text deltas from Anthropic's SSE event stream.
///
/// Anthropic emits named server-sent events (`message_start`,
/// `content_block_delta`, `message_delta`, `message_stop`, ...). Only text
/// deltas are surfaced here; the [`ChatStream`] abstraction is text-only.
struct AnthropicSseStream<S> {
    inner: S,
    buffer: String,
    done: bool,
}

impl<S> Stream for AnthropicSseStream<S>
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Unpin,
{
    type Item = Result<String, Error>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        use std::task::Poll;

        let this = self.get_mut();

        loop {
            if this.done {
                return Poll::Ready(None);
            }

            if let Some(position) = this.buffer.find('\n') {
                let line: String = this.buffer.drain(..=position).collect();
                match parse_event_line(line.trim()) {
                    Some(StreamEvent::Text(text)) => return Poll::Ready(Some(Ok(text))),
                    Some(StreamEvent::Stop) => {
                        this.done = true;
                        return Poll::Ready(None);
                    }
                    Some(StreamEvent::Error(message)) => {
                        this.done = true;
                        return Poll::Ready(Some(Err(Error::Stream(message))));
                    }
                    None => continue,
                }
            }

            match futures_util::ready!(this.inner.poll_next_unpin(cx)) {
                Some(Ok(bytes)) => this.buffer.push_str(&String::from_utf8_lossy(&bytes)),
                Some(Err(error)) => {
                    this.done = true;
                    return Poll::Ready(Some(Err(Error::Request(error))));
                }
                None => {
                    this.done = true;
                    return Poll::Ready(None);
                }
            }
        }
    }
}

/// A parsed Anthropic stream event, reduced to what [`ChatStream`] needs.
enum StreamEvent {
    Text(String),
    Stop,
    Error(String),
}

fn parse_event_line(line: &str) -> Option<StreamEvent> {
    let data = line.strip_prefix("data:")?.trim();
    if data.is_empty() {
        return None;
    }

    let value: Value = match serde_json::from_str(data) {
        Ok(value) => value,
        Err(error) => return Some(StreamEvent::Error(error.to_string())),
    };

    match value.get("type").and_then(Value::as_str) {
        Some("content_block_delta") => value
            .pointer("/delta/text")
            .and_then(Value::as_str)
            .filter(|text| !text.is_empty())
            .map(|text| StreamEvent::Text(text.to_string())),
        Some("message_stop") => Some(StreamEvent::Stop),
        Some("error") => {
            let message = value
                .pointer("/error/message")
                .and_then(Value::as_str)
                .unwrap_or("anthropic stream error");
            Some(StreamEvent::Error(message.to_string()))
        }
        _ => None,
    }
}
