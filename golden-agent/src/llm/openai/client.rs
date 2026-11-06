use std::collections::{BTreeMap, VecDeque};

use async_trait::async_trait;
use bytes::Bytes;
use futures_core::Stream;
use futures_util::StreamExt;

use crate::error::{ApiErrorBody, Error};
use crate::llm::chat::{
    ChatEvent, ChatModel, ChatRequest, ChatResponse, ChatStream, PartialToolCall,
};
use crate::llm::http::HttpConfig;
use crate::llm::models::{ModelCatalog, ModelInfo};

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
    supports_fast_mode: bool,
}

impl OpenAiLlm {
    /// Constructs a client from a base URL and API key (no provider defaults).
    /// Constructs with an explicit API key.
    ///
    /// An empty `api_key` omits the `Authorization` header entirely, for
    /// local OpenAI-compatible servers (llama.cpp, Ollama, LM Studio) that
    /// do not require authentication.
    pub fn new(base_url: impl Into<String>, api_key: impl Into<String>) -> Self {
        let config = HttpConfig::default();
        Self {
            http: config.build_client(),
            api_key: api_key.into(),
            base_url: base_url.into(),
            config,
            supports_fast_mode: false,
        }
    }

    /// Constructs a client pointed at the official OpenAI endpoint.
    pub fn openai_default(api_key: impl Into<String>) -> Self {
        let mut client = Self::new("https://api.openai.com/v1", api_key);
        client.supports_fast_mode = true;
        client
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

    /// Fetches the models exposed by this OpenAI-compatible server.
    ///
    /// DeepSeek and other compatible servers can return a display name in
    /// `name` and a context-window size in `context_window`. These fields
    /// remain optional because the OpenAI model-listing schema omits them.
    pub async fn list_models(&self) -> Result<Vec<ModelInfo>, Error> {
        let mut request = self.http.get(self.endpoint("/models"));
        if !self.api_key.is_empty() {
            request = request.bearer_auth(&self.api_key);
        }
        let response = self.config.bound(request).send().await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.json::<ApiErrorBody>().await.ok();
            return Err(Error::api(status, body));
        }

        let response = response.json::<ModelsResponse>().await?;
        Ok(response.data.into_iter().map(ModelInfo::from).collect())
    }

    fn endpoint(&self, path: &str) -> String {
        format!("{}{}", self.base_url.trim_end_matches('/'), path)
    }
}

#[async_trait]
impl ModelCatalog for OpenAiLlm {
    async fn list_models(&self) -> Result<Vec<ModelInfo>, Error> {
        OpenAiLlm::list_models(self).await
    }
}

#[async_trait]
impl ChatModel for OpenAiLlm {
    async fn chat(&self, request: &ChatRequest) -> Result<ChatResponse, Error> {
        let body = translate::to_wire_request(request, false, self.supports_fast_mode);

        let mut req = self
            .http
            .post(self.endpoint("/chat/completions"))
            .json(&body);
        if !self.api_key.is_empty() {
            req = req.bearer_auth(&self.api_key);
        }
        let response = self.config.bound(req).send().await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.json::<ApiErrorBody>().await.ok();
            return Err(Error::api(status, body));
        }

        let response = response.json::<super::response::ChatResponse>().await?;
        Ok(translate::from_wire_response(response))
    }

    fn chat_stream(&self, request: &ChatRequest) -> ChatStream<'_> {
        let body = translate::to_wire_request(request, true, self.supports_fast_mode)
            .stream_options(super::request::StreamOptions::include_usage());

        let http = self.http.clone();
        let api_key = self.api_key.clone();
        let url = self.endpoint("/chat/completions");

        Box::pin(async_stream::stream! {
            let mut req = http.post(&url).json(&body);
            if !api_key.is_empty() {
                req = req.bearer_auth(&api_key);
            }
            let response = match req.send().await {
                Ok(response) => response,
                Err(error) => { yield Err(Error::Request(error)); return; }
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
                buffer: Vec::new(),
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
    buffer: Vec<u8>,
    done: bool,
    pending: VecDeque<ChatEvent>,
    tool_calls: BTreeMap<usize, PartialToolCall>,
}

impl<S> SseLineStream<S> {
    /// Merges a parsed chunk into the pending events and tool-call buffers.
    fn handle_chunk(&mut self, chunk: StreamData) {
        if let Some(usage) = chunk.usage {
            self.pending
                .push_back(ChatEvent::Usage(translate::from_wire_usage(usage)));
        }
        for choice in chunk.choices {
            if let Some(reasoning) = choice.delta.reasoning_content
                && !reasoning.is_empty()
            {
                self.pending.push_back(ChatEvent::Reasoning(reasoning));
            }

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

            if let Some(position) = this.buffer.iter().position(|&byte| byte == b'\n') {
                let line_bytes = &this.buffer[..position];
                // A trailing multi-byte character split across chunks leaves an
                // incomplete sequence at the end of the line; keep the line in the
                // buffer until the next chunk completes it.
                if let Err(error) = std::str::from_utf8(line_bytes)
                    && error.error_len().is_none()
                {
                    match futures_util::ready!(this.inner.poll_next_unpin(cx)) {
                        Some(Ok(bytes)) => {
                            this.buffer.extend_from_slice(&bytes);
                            continue;
                        }
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

                let line_bytes: Vec<u8> = this.buffer.drain(..=position).collect();
                let line = String::from_utf8_lossy(&line_bytes);
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
                Some(Ok(bytes)) => this.buffer.extend_from_slice(&bytes),
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
struct ModelsResponse {
    #[serde(default)]
    data: Vec<ModelObject>,
}

#[derive(Debug, serde::Deserialize)]
struct ModelObject {
    id: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    owned_by: Option<String>,
    #[serde(default)]
    created: Option<u64>,
    #[serde(default)]
    context_window: Option<u64>,
}

impl From<ModelObject> for ModelInfo {
    fn from(model: ModelObject) -> Self {
        Self {
            id: model.id,
            display_name: model.name,
            owned_by: model.owned_by,
            created_at: model.created.map(|created| created.to_string()),
            context_window: model.context_window,
            max_input_tokens: None,
            max_output_tokens: None,
        }
    }
}

#[derive(Debug, serde::Deserialize)]
struct StreamData {
    #[serde(default)]
    choices: Vec<StreamDeltaChoice>,
    usage: Option<super::response::Usage>,
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
    /// The reasoning (thinking) delta, sent by DeepSeek and other
    /// OpenAI-compatible reasoning providers.
    #[serde(default)]
    reasoning_content: Option<String>,
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Feeds `chunks` through [`SseLineStream`] and joins the emitted text.
    async fn collect_text(chunks: Vec<&'static [u8]>) -> String {
        let stream = futures_util::stream::iter(
            chunks
                .into_iter()
                .map(|chunk| Ok::<_, reqwest::Error>(Bytes::from_static(chunk))),
        );
        let mut lines = SseLineStream {
            inner: stream,
            buffer: Vec::new(),
            done: false,
            pending: VecDeque::new(),
            tool_calls: BTreeMap::new(),
        };

        let mut text = String::new();
        while let Some(item) = lines.next().await {
            if let ChatEvent::Text(delta) = item.unwrap() {
                text.push_str(&delta);
            }
        }
        text
    }

    #[test]
    fn model_list_preserves_optional_capability_metadata() {
        let response: ModelsResponse = serde_json::from_value(serde_json::json!({
            "object": "list",
            "data": [{
                "id": "gpt-test",
                "object": "model",
                "created": 1_700_000_000,
                "owned_by": "openai",
                "name": "Test Model",
                "context_window": 128_000
            }]
        }))
        .unwrap();

        let models: Vec<ModelInfo> = response.data.into_iter().map(ModelInfo::from).collect();
        assert_eq!(
            models,
            vec![ModelInfo {
                id: "gpt-test".to_string(),
                display_name: Some("Test Model".to_string()),
                owned_by: Some("openai".to_string()),
                created_at: Some("1700000000".to_string()),
                context_window: Some(128_000),
                max_input_tokens: None,
                max_output_tokens: None,
            }]
        );
    }

    #[test]
    fn official_model_list_may_omit_context_window() {
        let response: ModelsResponse = serde_json::from_value(serde_json::json!({
            "data": [{"id": "gpt-test", "owned_by": "openai"}]
        }))
        .unwrap();
        let model = ModelInfo::from(response.data.into_iter().next().unwrap());

        assert_eq!(model.display_name, None);
        assert_eq!(model.context_window, None);
    }

    #[tokio::test]
    async fn multibyte_character_split_across_chunks_is_not_mangled() {
        // "你好" split mid-character: [0xE4, 0xBD] | [0xA0, 0xE5, 0xA5, 0xBD].
        let text = collect_text(vec![
            b"data: {\"choices\":[{\"delta\":{\"content\":\"\xe4\xbd".as_slice(),
            b"\xa0\xe5\xa5\xbd\"}}]}\n\n",
        ])
        .await;

        assert_eq!(text, "你好");
        assert!(!text.contains('\u{FFFD}'));
    }

    #[tokio::test]
    async fn final_stream_chunk_reports_usage_even_without_choices() {
        let inner = futures_util::stream::iter([Ok(Bytes::from_static(
            b"data: {\"choices\":[],\"usage\":{\"prompt_tokens\":33000,\"completion_tokens\":100,\"total_tokens\":33100}}\n\ndata: [DONE]\n\n",
        ))]);
        let mut stream = SseLineStream {
            inner,
            buffer: Vec::new(),
            done: false,
            pending: VecDeque::new(),
            tool_calls: BTreeMap::new(),
        };

        let ChatEvent::Usage(usage) = stream.next().await.unwrap().unwrap() else {
            panic!("expected usage");
        };
        assert_eq!(usage.prompt_tokens, 33_000);
        assert_eq!(usage.total_tokens, 33_100);
        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn complete_events_are_consumed_and_the_stream_terminates() {
        // Two complete events in a single chunk: the first must be parsed and
        // removed from the buffer, the second must terminate the stream rather
        // than being re-read forever.
        let text = collect_text(vec![
            b"data: {\"choices\":[{\"delta\":{\"content\":\"hi\"}}]}\n\n\
              data: [DONE]\n\n"
                .as_slice(),
        ])
        .await;

        assert_eq!(text, "hi");
    }
}
