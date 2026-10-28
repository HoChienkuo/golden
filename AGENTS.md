# Golden

## Project Overview

Golden is an **annotation-driven web and agent framework** for Rust.

### Architecture

- Macros emit `inventory::submit!` entries of `RouteDefinition` (web) / `ToolDefinition` (agent);
  the kernel collects them via `inventory::iter`, validates, and assembles an Axum `Router`.
- `golden-agent` runs a ReAct loop (`Agent`) with `Middleware` hooks; see `golden-agent/README.md`.
- `#[golden_boot_application]` rewrites `async fn main` into a runtime bootstrap call
  (`run` / `run_fallible`).

### Crates

| Crate | Role |
| --- | --- |
| `golden-kernel` | Tokio runtime bootstrap + graceful shutdown, `inventory` route discovery, router assembly, and validation |
| `golden-macros` | `proc-macro = true`; `#[golden_boot_application]`, HTTP mapping attributes, `#[derive(RequestEntity)]`, `#[tool]` |
| `golden-boot` | User-facing facade re-exporting kernel + macros; the usual dependency for applications |
| `golden-agent` | Tool registration, provider-neutral chat models, and the ReAct agent loop; see `golden-agent/README.md` for its full API |

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
- Keep PRs small and focused; update related docs/READMEs (`README.md`, `README_zh.md`, per-crate
  READMEs) in the same PR.

## Commit messages

Follow [Conventional Commits](https://www.conventionalcommits.org). Scope is the crate or area
name (`golden-boot`, `golden-kernel`, `golden-macros`, `golden-agent`, `examples`).
