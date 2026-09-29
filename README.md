# Golden

An annotation-driven **web and agent** framework for Rust.

Golden lets you declare HTTP handlers and LLM tools as annotated `async fn`s. Procedural macros
register them statically, and the runtime takes care of the wiring.

[中文文档](README_zh.md)

## Crates

| Crate | Description |
| --- | --- |
| [`golden-boot`](./golden-boot) | The user-facing web framework: mapping annotations, `#[golden_boot_application]` entry point, `RequestEntity`, and response helpers. |
| [`golden-kernel`](./golden-kernel) | Core web runtime: routing, application state, request entities, and response helpers. |
| [`golden-macros`](./golden-macros) | Procedural macros backing GoldenBoot and GoldenAgent. |
| [`golden-agent`](./golden-agent) | Agent framework: `#[tool]` declaration, per-provider tool rendering, and chat models (OpenAI, DeepSeek, Anthropic). |

## Getting started

### Web

Add `golden-boot` and annotate an `async fn main`:

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

The server listens on `http://0.0.0.0:8080` by default. See the
[golden-boot README](./golden-boot/README.md) for routing, request entities, application state, and
response helpers.

### Agent

Add `golden-agent` and annotate tools with `#[tool]`:

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

See the [golden-agent README](./golden-agent/README.md) for calling chat models and the roadmap.

## Examples

- [`examples/hello-world`](./examples/hello-world) — a minimal GoldenBoot application.
- [`examples/article-server`](./examples/article-server) — a SQLite-backed article CRUD server.
- [`golden-agent/examples`](./golden-agent/examples) — tool calling, chat completion, and streaming.

## License

Licensed under either of [MIT](./LICENSE-MIT) or [Apache-2.0](./LICENSE-APACHE), at your option.
