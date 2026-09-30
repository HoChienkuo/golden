# GoldenAgent

GoldenAgent 是 [Golden](https://github.com/HoChienkuo/golden) 的 agent 部分：一个用于在 Rust 中构建 LLM 应用的注解驱动框架。
把工具写在代码旁边，直接调用聊天模型，无需手工拼接协议。

这是一个长期迭代中的项目。下方路线图记录了当前已支持与规划中的能力；想直接看代码，请浏览
[examples](https://github.com/HoChienkuo/golden/tree/master/golden-agent/examples)。

[English Docs](README.md)

## 快速开始

在 `Cargo.toml` 中添加 GoldenAgent：

```toml
[dependencies]
golden-agent = "0.1"
```

## 最简单的一个例子

用 `#[tool]` 标注一个函数即可把它暴露给模型，然后发一次聊天调用：

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

运行需要 `DEEPSEEK_API_KEY` 和网络访问。更多示例——完整的工具调用循环、流式——见
[`golden-agent/examples`](https://github.com/HoChienkuo/golden/tree/master/golden-agent/examples)。

## 功能

- [x] 流式 / 非流式对话
- [x] OpenAI 兼容协议（OpenAI、DeepSeek）
- [x] `#[tool]` 工具声明、注册与按 provider 渲染
- [x] Anthropic Messages API
- [ ] Google Gemini API
- [ ] Ollama
- [x] 字符串模板 / prompt 模板
- [x] 结构化输出（`response_format` 与类型化解析）
- [ ] 会话历史与上下文管理
- [x] 自动工具调用循环（Agent）
- [ ] 有状态工具
- [ ] MCP 工具接入
- [ ] Web 端点（HTTP / SSE）

## 许可证

在你选择的任一许可证下授权：[MIT](https://github.com/HoChienkuo/golden/blob/master/LICENSE-MIT) 或 [Apache-2.0](https://github.com/HoChienkuo/golden/blob/master/LICENSE-APACHE)。
