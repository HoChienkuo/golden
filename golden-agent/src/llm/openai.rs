use async_trait::async_trait;
use bytes::Bytes;
use futures_core::Stream;
use futures_util::StreamExt;

use crate::{
    chat::{ChatModel, ChatStream},
    error::{ApiErrorBody, Error},
    request::ChatRequest,
    response::ChatResponse,
};

/// Commonly used OpenAI models.
///
/// This list is not exhaustive and may fall behind upstream releases;
/// use [`OpenAiModel::Custom`] for models not listed here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenAiModel {
    Gpt4o,
    Gpt4oMini,
    Gpt41,
    Gpt41Mini,
    Custom(String),
}

impl OpenAiModel {
    pub fn as_str(&self) -> &str {
        match self {
            OpenAiModel::Gpt4o => "gpt-4o",
            OpenAiModel::Gpt4oMini => "gpt-4o-mini",
            OpenAiModel::Gpt41 => "gpt-4.1",
            OpenAiModel::Gpt41Mini => "gpt-4.1-mini",
            OpenAiModel::Custom(s) => s,
        }
    }
}

impl Default for OpenAiModel {
    fn default() -> Self {
        OpenAiModel::Gpt4o
    }
}

/// The OpenAI provider, and the base implementation of the OpenAI protocol.
///
/// Other OpenAI-compatible providers (DeepSeek, Ollama) reuse it to express
/// themselves.
#[derive(Clone)]
pub struct OpenAiLlm {
    http: reqwest::Client,
    api_key: String,
    base_url: String,
}

impl OpenAiLlm {
    /// Constructs a client from a base URL and API key (no provider defaults).
    pub fn new(base_url: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            http: reqwest::Client::new(),
            api_key: api_key.into(),
            base_url: base_url.into(),
        }
    }

    /// Constructs a client pointed at the official OpenAI endpoint.
    pub fn openai_default(api_key: impl Into<String>) -> Self {
        Self::new("https://api.openai.com/v1", api_key)
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
        let mut req = request.clone();
        req.stream = Some(false);

        let response = self
            .http
            .post(self.endpoint("/chat/completions"))
            .bearer_auth(&self.api_key)
            .json(&req)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.json::<ApiErrorBody>().await.ok();
            return Err(Error::api(status, body));
        }

        Ok(response.json::<ChatResponse>().await?)
    }

    fn chat_stream(&self, request: &ChatRequest) -> ChatStream<'_> {
        let mut req = request.clone();
        req.stream = Some(true);

        let http = self.http.clone();
        let api_key = self.api_key.clone();
        let url = self.endpoint("/chat/completions");

        Box::pin(async_stream::stream! {
            let response = match http
                .post(&url)
                .bearer_auth(&api_key)
                .json(&req)
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => { yield Err(Error::Request(e)); return; }
            };

            let status = response.status();
            if !status.is_success() {
                let body = response.json::<ApiErrorBody>().await.ok();
                yield Err(Error::api(status, body));
                return;
            }

            let inner = response.bytes_stream();
            let mut line_stream = SseLineStream {
                inner,
                buffer: String::new(),
                done: false,
            };

            while let Some(item) = line_stream.next().await {
                yield item;
            }
        })
    }
}

/// A stream that extracts incremental text line-by-line from an SSE byte stream.
struct SseLineStream<S> {
    inner: S,
    buffer: String,
    done: bool,
}

impl<S> Stream for SseLineStream<S>
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Unpin,
{
    type Item = Result<String, Error>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let this = self.get_mut();

        loop {
            if this.done {
                return std::task::Poll::Ready(None);
            }

            if let Some(pos) = this.buffer.find('\n') {
                let line: String = this.buffer.drain(..=pos).collect();
                let line = line.trim();
                if line == "data: [DONE]" {
                    this.done = true;
                    return std::task::Poll::Ready(None);
                }
                if let Some(text) = parse_data_line(line) {
                    return std::task::Poll::Ready(Some(text));
                }
                continue;
            }

            match futures_util::ready!(this.inner.poll_next_unpin(cx)) {
                Some(Ok(bytes)) => {
                    this.buffer.push_str(&String::from_utf8_lossy(&bytes));
                }
                Some(Err(e)) => {
                    this.done = true;
                    return std::task::Poll::Ready(Some(Err(Error::Request(e))));
                }
                None => {
                    this.done = true;
                    return std::task::Poll::Ready(None);
                }
            }
        }
    }
}

fn parse_data_line(line: &str) -> Option<Result<String, Error>> {
    let line = line.trim();
    let data = line.strip_prefix("data:")?.trim();
    if data.is_empty() || data == "[DONE]" {
        return None;
    }
    match serde_json::from_str::<StreamData>(data) {
        Ok(chunk) => {
            let text = chunk
                .choices
                .first()
                .and_then(|c| c.delta.content.as_deref())
                .unwrap_or("");
            if text.is_empty() {
                None
            } else {
                Some(Ok(text.to_string()))
            }
        }
        Err(e) => Some(Err(Error::Stream(e.to_string()))),
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
}
