# Golden

一个面向 Rust 的、注解驱动的 **Web 与 Agent 框架**。

Golden 让你用带注解的 `async fn` 声明 HTTP 处理器与 LLM 工具；过程宏把它们静态注册，运行时负责装配。

[English Docs](README.md)

## 各 crate

| Crate | 说明 |
| --- | --- |
| [`golden-boot`](./golden-boot) | 面向用户的 Web 框架：映射注解、`#[golden_boot_application]` 入口、`RequestEntity` 以及响应助手。 |
| [`golden-kernel`](./golden-kernel) | Web 核心运行时：路由、应用状态、请求实体与响应助手。 |
| [`golden-macros`](./golden-macros) | GoldenBoot 与 GoldenAgent 背后的过程宏。 |
| [`golden-agent`](./golden-agent) | Agent 框架：`#[tool]` 声明、ReAct Agent 循环，以及聊天模型（OpenAI、DeepSeek、Anthropic）。 |

## 快速开始

### Web

添加 `golden-boot`，标注一个 `async fn main`：

```toml
[dependencies]
golden-boot = "0.1"
serde = { version = "1", features = ["derive"] }
```

```rust
use golden_boot::golden_boot_application;

#[golden_boot_application]
async fn main() {
    println!("initializing application");
}
```

服务默认监听 `http://0.0.0.0:8080`。关于路由、请求实体、应用状态与响应助手，请参阅 [golden-boot README](./golden-boot/README.md)。

### Agent

添加 `golden-agent`，用 `#[tool]` 标注工具：

```toml
[dependencies]
golden-agent = "0.1"
```

```rust
use golden_agent::tool;

/// Get the current weather for a city.
#[tool]
async fn get_weather(city: String) -> String {
    format!("{city}: 20°C")
}
```

关于调用聊天模型与路线图，请参阅 [golden-agent README](./golden-agent/README.md)。

## 示例

- [`examples/hello-world`](./examples/hello-world) — 一个最小的 GoldenBoot 应用。
- [`examples/article-server`](./examples/article-server) — 一个基于 SQLite 的文章 CRUD 服务。
- [`golden-agent/examples`](./golden-agent/examples) — Agent 循环、手工工具循环、聊天补全与流式。

## 许可证

在你选择的任一许可证下授权：[MIT](./LICENSE-MIT) 或 [Apache-2.0](./LICENSE-APACHE)。
