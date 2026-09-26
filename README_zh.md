# Golden

一个构建于 [Axum](https://github.com/tokio-rs/axum) 之上的、注解驱动的 Web 框架。

Golden 提供了简洁的应用入口，并把运行时、路由以及 HTTP 服务器的装配细节从应用代码中剥离出去。

[English Docs](README.md)

## 各 crate

| Crate | 说明 |
| --- | --- |
| [`golden-boot`](./golden-boot) | 面向用户的框架：映射注解、`#[golden_boot_application]` 入口、`RequestEntity` 以及响应助手。 |
| [`golden-kernel`](./golden-kernel) | 核心运行时：路由、应用状态、请求实体与响应助手。 |
| [`golden-macros`](./golden-macros) | GoldenBoot 背后的过程宏。 |

## 快速开始

在 `Cargo.toml` 中添加 `golden-boot`：

```toml
[dependencies]
golden-boot = "0.1"
serde = { version = "1", features = ["derive"] }
```

然后标注一个 `async fn main`：

```rust
use golden_boot::golden_boot_application;

#[golden_boot_application]
async fn main() {
    println!("initializing application");
}
```

运行后，服务默认监听 `http://0.0.0.0:8080`。

关于路由、请求实体、应用状态以及响应助手的更多内容，请参阅 [golden-boot README](./golden-boot/README.md)。

## 示例

`examples/` 目录包含可运行的演示：

- [`hello-world`](./examples/hello-world) — 一个最小的 GoldenBoot 应用。
- [`article-server`](./examples/article-server) — 一个基于 SQLite 的文章 CRUD 服务。

## 许可证

在你选择的任一许可证下授权：[MIT](./LICENSE-MIT) 或 [Apache-2.0](./LICENSE-APACHE)。
