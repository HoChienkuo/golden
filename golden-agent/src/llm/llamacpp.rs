use crate::error::Error;
use crate::llm::chat::{
    ChatModel, ChatRequest, ChatResponse, ChatStream, Reasoning, ReasoningEffort,
};
use crate::llm::http::HttpConfig;
use crate::llm::models::{ModelCatalog, ModelInfo};
use crate::llm::openai::{ChatRequest as OpenAiRequest, OpenAiLlm};
use async_trait::async_trait;
use serde::Serialize;

/// The llama.cpp provider, speaking llama-server's OpenAI-compatible protocol.
///
/// llama.cpp runs local GGUF models behind an OpenAI-compatible
/// `/v1/chat/completions` endpoint. No authentication is required by default,
/// so the api key may be empty. Explicit reasoning settings use llama-server's
/// reasoning effort and chat template controls.
///
/// ```ignore
/// use golden_agent::llm::LlamaCppLlm;
///
/// // Connect to a server started with:
/// //   ./llama-server -m /path/to/model.gguf --port 8080
/// let llm = LlamaCppLlm::new("http://localhost:8080/v1");
/// ```
#[derive(Clone)]
pub struct LlamaCppLlm(OpenAiLlm);

impl LlamaCppLlm {
    /// Constructs from a base URL, without an api key.
    ///
    /// `base_url` should include the scheme and the `/v1` suffix, for example
    /// `http://localhost:8080/v1`. Use an empty `api_key` for servers that do
    /// not require authentication.
    pub fn new(base_url: impl Into<String>) -> Self {
        Self(OpenAiLlm::new(base_url, ""))
    }

    /// Constructs from a base URL with an explicit api key, for servers
    /// configured to require one.
    pub fn with_api_key(base_url: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self(OpenAiLlm::new(base_url, api_key))
    }

    /// Replaces the HTTP reliability settings, rebuilding the underlying client.
    pub fn with_http_config(self, config: HttpConfig) -> Self {
        Self(self.0.with_http_config(config))
    }

    /// Exposes the underlying OpenAI client. Call this wrapper's chat methods
    /// to apply llama-server's reasoning controls.
    pub fn inner(&self) -> &OpenAiLlm {
        &self.0
    }

    /// Fetches the models exposed by llama-server at `/v1/models`.
    pub async fn list_models(&self) -> Result<Vec<ModelInfo>, Error> {
        self.0.list_models().await
    }

    fn wire_request(&self, request: &ChatRequest, stream: bool) -> LlamaCppRequest {
        let mut base = self.0.wire_request(request, stream);
        base.thinking = None;
        base.reasoning_effort = None;

        let reasoning_effort = match request.reasoning {
            None | Some(Reasoning::Enabled) => None,
            Some(reasoning) => Some(match reasoning.effort() {
                None => "none",
                Some(ReasoningEffort::Low) => "low",
                Some(ReasoningEffort::Medium) => "medium",
                Some(ReasoningEffort::High) => "high",
                Some(ReasoningEffort::Max) => "max",
            }),
        };
        LlamaCppRequest {
            base,
            reasoning_effort,
            chat_template_kwargs: request
                .reasoning
                .filter(Reasoning::is_enabled)
                .map(|_| serde_json::json!({"enable_thinking": true})),
        }
    }
}

#[async_trait]
impl ModelCatalog for LlamaCppLlm {
    async fn list_models(&self) -> Result<Vec<ModelInfo>, Error> {
        LlamaCppLlm::list_models(self).await
    }
}

#[async_trait]
impl ChatModel for LlamaCppLlm {
    async fn chat(&self, request: &ChatRequest) -> Result<ChatResponse, Error> {
        self.0.chat_wire(&self.wire_request(request, false)).await
    }

    fn chat_stream(&self, request: &ChatRequest) -> ChatStream<'_> {
        self.0.chat_stream_wire(&self.wire_request(request, true))
    }
}

#[derive(Serialize)]
struct LlamaCppRequest {
    #[serde(flatten)]
    base: OpenAiRequest,
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning_effort: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    chat_template_kwargs: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::chat::Message;
    use futures_util::TryStreamExt;
    use serde_json::{Value, json};
    use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
    use tokio::net::TcpListener;

    async fn capture_request(listener: &TcpListener) -> Value {
        let (socket, _) = listener.accept().await.unwrap();
        let mut socket = BufReader::new(socket);
        let mut content_length = None;
        loop {
            let mut line = String::new();
            assert_ne!(socket.read_line(&mut line).await.unwrap(), 0);
            if line == "\r\n" {
                break;
            }
            if let Some((name, value)) = line.split_once(':')
                && name.eq_ignore_ascii_case("content-length")
            {
                content_length = Some(value.trim().parse().unwrap());
            }
        }
        let mut body = vec![0; content_length.unwrap()];
        socket.read_exact(&mut body).await.unwrap();
        socket
            .write_all(
                b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            )
            .await
            .unwrap();
        serde_json::from_slice(&body).unwrap()
    }

    #[tokio::test]
    async fn chat_and_stream_send_llamacpp_reasoning_fields() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let llm = LlamaCppLlm::new(format!("http://{}/v1", listener.local_addr().unwrap()));

        for (reasoning, effort, enable_thinking) in [
            (None, None, false),
            (Some(Reasoning::Disabled), Some("none"), false),
            (Some(Reasoning::Enabled), None, true),
            (
                Some(Reasoning::Effort(ReasoningEffort::Medium)),
                Some("medium"),
                true,
            ),
        ] {
            let mut request = ChatRequest::new("test-model");
            request.messages.push(Message::user("Hello"));
            request.reasoning = reasoning;

            for stream in [false, true] {
                let chat = async {
                    if stream {
                        llm.chat_stream(&request)
                            .try_collect::<Vec<_>>()
                            .await
                            .unwrap_err();
                    } else {
                        llm.chat(&request).await.unwrap_err();
                    }
                };
                let (_, body) = tokio::time::timeout(std::time::Duration::from_secs(5), async {
                    tokio::join!(chat, capture_request(&listener))
                })
                .await
                .unwrap();
                assert_eq!(body["stream"], stream);
                assert_eq!(body["model"], "test-model");
                assert_eq!(body["messages"][0]["content"], "Hello");
                assert_eq!(body.get("thinking"), None);
                assert_eq!(
                    body.get("reasoning_effort").cloned(),
                    effort.map(Value::from)
                );
                assert_eq!(
                    body.get("chat_template_kwargs").cloned(),
                    enable_thinking.then(|| json!({"enable_thinking": true}))
                );
            }
        }
    }
}
