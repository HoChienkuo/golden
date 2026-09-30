# GoldenAgent

GoldenAgent is the agent half of [Golden](https://github.com/HoChienkuo/golden): an annotation-driven
framework for building LLM applications in Rust. Describe tools next to your code and call chat
models without hand-wiring the protocol.

This is a long-term project under active development. The roadmap below tracks what is available
today and what is planned; for runnable code, browse the
[examples](https://github.com/HoChienkuo/golden/tree/master/golden-agent/examples).

[中文文档](README_zh.md)

## Quick start

Add GoldenAgent to your `Cargo.toml`:

```toml
[dependencies]
golden-agent = "0.1"
```

## Minimal example

Annotate a function with `#[tool]` to expose it to the model, then make one chat call:

```rust
use golden_agent::{tool, ChatModel, ChatRequest, DeepSeekLlm, DeepSeekModel, Message};

/// Get the current weather for a city.
#[tool]
async fn get_weather(city: String) -> String {
    format!("{city}: 20°C")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let request = ChatRequest::new(DeepSeekModel::Flash.as_str())
        .add_message(Message::user("What's the weather in Beijing?"))
        .tools(golden_agent::tool_specs()?);

    let response = DeepSeekLlm::from_env().chat(&request).await?;
    println!("{}", response.text());
    Ok(())
}
```

This needs `DEEPSEEK_API_KEY` and network access. More examples — the full tool-calling loop and
streaming — live in
[`golden-agent/examples`](https://github.com/HoChienkuo/golden/tree/master/golden-agent/examples).

## Features

- [x] Non-streaming and streaming chat
- [x] OpenAI-compatible protocol (OpenAI, DeepSeek)
- [x] `#[tool]` tool declaration, registration, and per-provider rendering
- [x] Anthropic Messages API
- [ ] Google Gemini API
- [ ] Ollama
- [x] String / prompt templates
- [ ] Conversation history and context management
- [x] Automatic tool-calling loop (agents)
- [ ] Stateful tools
- [ ] MCP tool sources
- [ ] Web endpoints (HTTP / SSE)

## License

Licensed under either of [MIT](https://github.com/HoChienkuo/golden/blob/master/LICENSE-MIT) or [Apache-2.0](https://github.com/HoChienkuo/golden/blob/master/LICENSE-APACHE), at your option.
