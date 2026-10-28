# Golden-Boot

Crate-level rules for the web framework facade. Project-wide rules live in the root `AGENTS.md`.

## Testing

The web framework's test suite lives in this crate.

| Test type | Location | Notes |
| --- | --- | --- |
| Runtime/integration | `tests/*_runtime.rs` | Drive the router with `tower::ServiceExt::oneshot` against `golden_boot::__private::{create_router, run_fallible}`; assert status codes and bodies; use `#[tokio::test]` |
| Compile-time | `tests/compile_tests.rs` + `tests/ui/{pass,fail}` | `trybuild`; each `fail/*.rs` has a committed `.stderr` pair |
| Unit | inline `#[cfg(test)] mod tests` | |

- For macro diagnostics, add or adjust the `tests/ui/fail` case together with its `.stderr` pair.
- Regenerate expected diagnostics with `TRYBUILD=overwrite cargo test --test compile_tests`.
