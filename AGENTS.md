# Golden

## Project Overview

Golden is an **annotation-driven web and agent framework** for Rust. It is a Cargo workspace
(`resolver = "3"`, dual-licensed MIT OR Apache-2.0) targeting the
**2024 edition**. Procedural macros emit static registration entries that the runtime collects and
folds into an [Axum](https://github.com/tokio-rs/axum) router.

**Tech stack:** axum 0.8, tokio 1, inventory 0.3, thiserror 2, serde 1, serde_json 1,
validator 0.20, reqwest 0.12 (rustls-tls, no default features), proc-macro2/quote/syn 2,
proc-macro-crate 3, trybuild 1, futures 0.3, bytes 1, async-trait 0.1, async-stream 0.3.

### Architecture

```
golden-macros (compile time)  ->  golden-kernel / golden-agent (runtime)  ->  golden-boot (facade)
```

- Macros emit `inventory::submit!` entries of `RouteDefinition` (web) or `ToolDefinition` (agent).
- The kernel collects them via `inventory::iter`, validates them, and assembles an Axum `Router`.
- `golden-agent` collects `ToolDefinition` entries the same way, renders them per provider, and runs
  a ReAct loop (`Agent`): it calls the model, executes the requested tools, feeds the results back,
  and repeats until the model stops calling tools or `max_steps` is reached. `ToolSet` selects tools
  by wire name and `Middleware` hooks wrap each run, model call, and tool call.
- `#[golden_boot_application]` rewrites `async fn main` into a runtime bootstrap call (`run` / `run_fallible`).
- `golden-boot` only re-exports; it does not implement behavior.

### Crates

| Crate | Role |
| --- | --- |
| `golden-kernel` (lib name `golden_core`) | Tokio runtime bootstrap + graceful shutdown, `inventory` route discovery, router assembly, duplicate-route and state-type validation, `RequestEntity`, response helpers (`ApiResponse`, `ResponseEntity`, `Page`), error types, and Axum re-exports (`web`, `header`, `multipart`, `sse`, `__private`) |
| `golden-macros` | `proc-macro = true`; `#[golden_boot_application]`, HTTP mapping attributes, `#[derive(RequestEntity)]`, `#[tool]`; features `web`, `agent`, `all` (default) |
| `golden-boot` | User-facing facade re-exporting kernel + macros; the usual dependency for applications |
| `golden-agent` | `#[tool]` registration via `inventory` (`ToolDefinition`, `registered_tools` / `tool_specs` / `render_tools`); a provider-neutral tool model (`ToolSpec`, `Tool`, `FromToolSpec`, `ToolSet`); a provider-neutral `ChatModel` / `ChatStream` (`ChatRequest`, `ChatResponse`, `Message`, `ChatEvent`) with OpenAI, DeepSeek and Anthropic providers, each translating to its own wire format; `HttpConfig` (connect and request timeouts); and a ReAct loop (`Agent`, `AgentBuilder`, `AgentState`, `Middleware`, `AgentResult`) driving the model, tools, and per-call hooks, with a built-in `Retry` middleware |

### Examples

| Example | Description |
| --- | --- |
| `examples/hello-world` | Minimal GoldenBoot app |
| `examples/article-server` | SQLite-backed article CRUD via `sqlx` 0.9 |

## Setup & Development

- Install a Rust toolchain with 2024-edition support (Rust 1.85+), then build with
  `cargo build --workspace`.
- Run an example with `cargo run -p hello-world` (or `-p article-server`); see
  `examples/article-server/README.md` for SQLite setup and `oha` stress-test instructions.
- `golden-agent`'s examples (`agent`, `tools`, `chat_completion`, `chat_stream`) additionally need
  `DEEPSEEK_API_KEY` and network access.
- Add shared dependencies to the root `[workspace.dependencies]` and reference them in crate
  manifests with `dep.workspace = true`.
- Do not commit changes unless explicitly asked. Do not bump versions or hand-edit `Cargo.lock`.

## Testing Guidelines

- Run the full suite with `cargo test --workspace`. Do not generate new tests as part of a
  change unless explicitly asked.
- Write test names as sentences describing intent (e.g. `reports_the_actual_missing_header_name`).

| Test type | Location | Notes |
| --- | --- | --- |
| Runtime/integration | `golden-boot/tests/*_runtime.rs` | Drive the router with `tower::ServiceExt::oneshot` against `golden_boot::__private::{create_router, run_fallible}`; assert status codes and bodies; use `#[tokio::test]` |
| Compile-time | `golden-boot/tests/compile_tests.rs` + `tests/ui/{pass,fail}` | `trybuild`; each `fail/*.rs` has a committed `.stderr` pair |
| Unit | inline `#[cfg(test)] mod tests` (e.g. `golden-kernel/src/routing.rs`) | |

- For macro diagnostics, add or adjust the `tests/ui/fail` case together with its `.stderr` pair.
- Regenerate expected diagnostics with `TRYBUILD=overwrite cargo test --test compile_tests`.

## Code Style

- Rust 2024 edition. Before submitting, run `cargo fmt` and
  `cargo clippy --workspace --all-targets -- -D warnings`.
- Doc-comment every public item; macro entry points document behavior plus an ```` ```ignore ````
  example (see `golden-macros/src/lib.rs`).
- Use `thiserror` for error types; public error enums are `#[non_exhaustive]` with `#[source]` on
  wrapped errors (e.g. `ApplicationError`, `golden-agent::Error`).
- Keep proc-macro logic modular per concern (separate parse/model/expand files, as in
  `request_entity/`); return `syn::Error::into_compile_error()` on macro failure.
- Preserve the layering: runtime behavior lives in the kernel; `golden-boot` only re-exports.
- Keep changes minimal and consistent with surrounding code; prefer existing patterns
  (`inventory` registration, `__private` re-export modules) over new dependencies.

## Pull Requests

- Keep PRs small and focused on one change; describe the motivation and the public API impact.
- Before opening, all must pass: `cargo fmt`,
  `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace`.
- Update related docs/READMEs (`README.md`, `README_zh.md`, per-crate READMEs) in the same PR.
- Call out any breaking change to a public macro or type explicitly.

### Commit messages

Follow [Conventional Commits](https://www.conventionalcommits.org):

```
<type>(<scope>): <description>

[optional body]

[optional footer(s)]
```

- `type`: `feat`, `fix`, `docs`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, or `revert`.
- `scope`: optional; use the crate or area name — `golden-boot`, `golden-kernel`, `golden-macros`,
  `golden-agent`, or `examples`.
- `description`: imperative mood, lowercase, no trailing period; keep the subject within ~72
  characters.
- Body: optional; explain the *what* and *why*, using `-` bullets for multiple points.
- Breaking change: add `!` after the type/scope and a `BREAKING CHANGE:` footer describing the
  migration.

```
feat(golden-agent): add annotation-driven agent framework foundation
fix(request-entity): report the actual missing header name
docs: document application state validation

feat(golden-macros)!: drop the legacy `#[controller]` attribute

BREAKING CHANGE: use `#[get_mapping]` instead.
```
