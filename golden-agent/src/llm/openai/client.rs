use std::collections::{BTreeMap, VecDeque};

use async_trait::async_trait;
use bytes::Bytes;
use futures_core::Stream;
use futures_util::StreamExt;

use crate::error::{ApiErrorBody, Error};
use crate::llm::chat::{
    ChatEvent, ChatModel, ChatRequest, ChatResponse, ChatStream, PartialToolCall,
};
use crate::llm::http::{self, HttpConfig};

use super::translate;

/// The OpenAI provider, and the base implementation of the OpenAI protocol.
///
/// Other OpenAI-compatible providers (DeepSeek, Ollama) reuse it to express
/// themselves.
#[derive(Clone)]
pub struct OpenAiLlm {
    http: reqwest::Client,
    api_key: String,
    base_url: String,
    config: HttpConfig,
}

impl OpenAiLlm {
    /// Constructs a client from a base URL and API key (no provider defaults).
    pub fn new(base_url: impl Into<String>, api_key: impl Into<String>) -> Self {
        let config = HttpConfig::default();
        Self {
            http: config.build_client(),
            api_key: api_key.into(),
            base_url: base_url.into(),
            config,
        }
    }

    /// Constructs a client pointed at the official OpenAI endpoint.
    pub fn openai_default(api_key: impl Into<String>) -> Self {
        Self::new("https://api.openai.com/v1", api_key)
    }

    /// Replaces the HTTP reliability settings, rebuilding the underlying client.
    pub fn with_http_config(mut self, config: HttpConfig) -> Self {
        self.http = config.build_client();
        self.config = config;
        self
    }

    /// Returns the current base URL.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    fn endpoint(&self, path: &str) -> String {
        format!("{}{}", self.base_url.trim_end_matches('/'), path)
    }
}

#[async_trait]
impl ChatModel for OpenAiLlm {
    async fn chat(&self, request: &ChatRequest) -> Result<ChatResponse, Error> {
        let body = translate::to_wire_request(request, false);

        http::with_retry(&self.config, || async {
            let mut builder = self
                .http
                .post(self.endpoint("/chat/completions"))
                .bearer_auth(&self.api_key)
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

            let response = response.json::<super::response::ChatResponse>().await?;
            Ok(translate::from_wire_response(response))
        })
        .await
    }

    fn chat_stream(&self, request: &ChatRequest) -> ChatStream<'_> {
        let body = translate::to_wire_request(request, true);

        let http = self.http.clone();
        let api_key = self.api_key.clone();
        let url = self.endpoint("/chat/completions");
        let config = self.config.clone();

        Box::pin(async_stream::stream! {
            let response = match http::with_retry(&config, || async {
                let response = http
                    .post(&url)
                    .bearer_auth(&api_key)
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
                Err(error) => { yield Err(error); return; }
            };

            let inner = response.bytes_stream();
            let mut line_stream = SseLineStream {
                inner,
                buffer: String::new(),
                done: false,
                pending: VecDeque::new(),
                tool_calls: BTreeMap::new(),
            };

            while let Some(item) = line_stream.next().await {
                yield item;
            }
        })
    }
}

/// A stream that turns an OpenAI SSE byte stream into [`ChatEvent`]s.
///
/// Tool-call fragments spread across chunks are accumulated by index and emitted
/// as complete [`ChatEvent::ToolCall`]s when the stream ends.
struct SseLineStream<S> {
    inner: S,
    buffer: String,
    done: bool,
    pending: VecDeque<ChatEvent>,
    tool_calls: BTreeMap<usize, PartialToolCall>,
}

impl<S> SseLineStream<S> {
    /// Merges a parsed chunk into the pending events and tool-call buffers.
    fn handle_chunk(&mut self, chunk: StreamData) {
        for choice in chunk.choices {
            let content = choice.delta.content.unwrap_or_default();
            if !content.is_empty() {
                self.pending.push_back(ChatEvent::Text(content));
            }

            for tool_call in choice.delta.tool_calls {
                let partial = self.tool_calls.entry(tool_call.index).or_default();
                if let Some(id) = tool_call.id {
                    partial.id = id;
                }
                if let Some(function) = tool_call.function {
                    if let Some(name) = function.name {
                        partial.name.push_str(&name);
                    }
                    if let Some(arguments) = function.arguments {
                        partial.arguments.push_str(&arguments);
                    }
                }
            }
        }
    }

    /// Moves the accumulated tool calls into the pending events.
    fn flush_tool_calls(&mut self) {
        for (_, partial) in std::mem::take(&mut self.tool_calls) {
            self.pending
                .push_back(ChatEvent::ToolCall(partial.into_tool_call()));
        }
    }
}

impl<S> Stream for SseLineStream<S>
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
                let line = line.trim();

                if line == "data: [DONE]" {
                    this.done = true;
                    this.flush_tool_calls();
                    continue;
                }

                if let Some(data) = line.strip_prefix("data:") {
                    let data = data.trim();
                    if !data.is_empty() {
                        match serde_json::from_str::<StreamData>(data) {
                            Ok(chunk) => this.handle_chunk(chunk),
                            Err(error) => {
                                this.done = true;
                                return Poll::Ready(Some(Err(Error::Stream(error.to_string()))));
                            }
                        }
                    }
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
                    this.flush_tool_calls();
                    continue;
                }
            }
        }
    }
}

#[derive(Debug, serde::Deserialize)]
struct StreamData {
    #[serde(default)]
    choices: Vec<StreamDeltaChoice>,
}

#[derive(Debug, serde::Deserialize)]
struct StreamDeltaChoice {
    #[serde(default)]
    delta: StreamDeltaContent,
}

#[derive(Debug, Default, serde::Deserialize)]
struct StreamDeltaContent {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Vec<StreamToolCall>,
}

#[derive(Debug, serde::Deserialize)]
struct StreamToolCall {
    #[serde(default)]
    index: usize,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    function: Option<StreamFunction>,
}

#[derive(Debug, serde::Deserialize)]
struct StreamFunction {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    arguments: Option<String>,
}
