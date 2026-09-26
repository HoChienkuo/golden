# Golden

An annotation-driven web framework built on [Axum](https://github.com/tokio-rs/axum).

Golden provides a concise application entry point and keeps the runtime, router, and HTTP server wiring out of your application code.

[中文文档](README_zh.md)

## Crates

| Crate | Description |
| --- | --- |
| [`golden-boot`](./golden-boot) | The user-facing framework: mapping annotations, `#[golden_boot_application]` entry point, `RequestEntity`, and response helpers. |
| [`golden-kernel`](./golden-kernel) | Core runtime: routing, application state, request entities, and response helpers. |
| [`golden-macros`](./golden-macros) | Procedural macros backing GoldenBoot. |

## Getting started

Add `golden-boot` to your `Cargo.toml`:

```toml
[dependencies]
golden-boot = "0.1"
serde = { version = "1", features = ["derive"] }
```

Then annotate an `async fn main`:

```rust
use golden_boot::golden_boot_application;

#[golden_boot_application]
async fn main() {
    println!("initializing application");
}
```

Run it and the server listens on `http://0.0.0.0:8080` by default.

See the [golden-boot README](./golden-boot/README.md) for routing, request entities, application state, and response helpers.

## Examples

The `examples/` directory contains runnable demonstrations:

- [`hello-world`](./examples/hello-world) — a minimal GoldenBoot application.
- [`article-server`](./examples/article-server) — a SQLite-backed article CRUD server.

## License

Licensed under either of [MIT](./LICENSE-MIT) or [Apache-2.0](./LICENSE-APACHE), at your option.
