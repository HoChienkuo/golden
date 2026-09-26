# GoldenBoot

GoldenBoot is an annotation-driven web framework built on [Axum](https://github.com/tokio-rs/axum).

It gives you a concise application entry point and keeps the runtime, router, and HTTP server wiring out of your application code.

[中文文档](README_zh.md)

## Features

- **Annotation-driven controller registration** — map handlers to HTTP methods with `#[get_mapping]`, `#[post_mapping]`, and friends. Routes are discovered and registered automatically, no manual router assembly.
- **Concise entry point** — annotate an `async fn main` with `#[golden_boot_application]` to boot the runtime and serve the HTTP server.
- **`State<T>` application state** — return a `Clone + Send + Sync` value from `main` and access it with Axum's `State<T>` extractor.
- **Rich request extraction** — derive `RequestEntity` to pull path variables, query parameters, headers, and JSON bodies into a single struct.
- **Validation** — combine with `validator`'s `#[derive(Validate)]` for automatic request validation.
- **Built-in response helpers** — `ApiResponse`, `ResponseEntity`, `ResponseEntityBuilder`, and `Page` for common REST and paginated responses.

## Installation

Add GoldenBoot to your `Cargo.toml`:

```toml
[dependencies]
golden-boot = "0.1"
serde = { version = "1", features = ["derive"] }
```

> GoldenBoot re-exports `validator`, `serde`, and the Axum web types you need, so you usually do not need to depend on them directly.

## Quick start

```rust
use golden_boot::golden_boot_application;

#[golden_boot_application]
async fn main() {
    println!("initializing application");
}
```

Run it and the server listens on `http://0.0.0.0:8080` by default:

```console
$ cargo run
GoldenBoot started
Listening on http://0.0.0.0:8080
```

Use the `port` argument to choose another port:

```rust
#[golden_boot_application(port = 9090)]
async fn main() {}
```

## Routing

Map any `async fn` to an HTTP method using the mapping annotations:

```rust
use golden_boot::{get_mapping, post_mapping};

#[get_mapping("/articles")]
async fn list_articles() -> &'static str {
    "[]"
}

#[post_mapping("/articles")]
async fn create_article() -> &'static str {
    "{}"
}
```

Supported annotations: `get_mapping`, `post_mapping`, `put_mapping`, `patch_mapping`, `delete_mapping`, `head_mapping`, `options_mapping`, `trace_mapping`, and `connect_mapping`.

### Path parameters

Use `{name}` segments. Write the extractor directly as `Path<T>` and GoldenBoot wires it up automatically:

```rust
use golden_boot::{Path, get_mapping};

#[get_mapping("/articles/{id}")]
async fn get_article(Path(id): Path<u64>) -> String {
    format!("article {id}")
}
```

### Application state

Return a `Clone + Send + Sync + 'static` value from the entry point and read it with `State<T>`:

```rust
use golden_boot::{State, get_mapping, golden_boot_application};

#[derive(Clone)]
struct AppState {
    name: &'static str,
}

#[get_mapping("/name")]
async fn name(State(state): State<AppState>) -> &'static str {
    state.name
}

#[golden_boot_application]
async fn main() -> AppState {
    AppState { name: "GoldenBoot" }
}
```

Every handler marked with `State<T>` must use the same `T` returned by `main`. Write `State<AppState>` (or `golden_boot::State<AppState>`) directly — type aliases are not recognized during macro expansion.

A fallible initializer may return a `Result` and use `?` normally:

```rust
#[golden_boot_application]
async fn main() -> Result<AppState, std::io::Error> {
    let database = connect_database().await?;
    Ok(AppState { database })
}
```

## Request entities

Derive `RequestEntity` to extract path variables, query parameters, headers, and a JSON body into one struct:

```rust
use golden_boot::{RequestEntity, header};

#[derive(RequestEntity)]
#[request_entity(rejection = ApiError)]
struct GetArticleRequest {
    #[path_variable(name = "id")]
    article_id: u64,

    #[request_param(default = 1)]
    page: u32,

    #[request_header(name = header::AUTHORIZATION)]
    authorization: Option<String>,
}

#[get_mapping("/articles/{id}")]
async fn get_article(request: GetArticleRequest) -> String {
    format!("id={};page={}", request.article_id, request.page)
}
```

Field attributes:

| Attribute | Description |
| --- | --- |
| `#[path_variable]` | A required path value. Use `#[path_variable(name = "id")]` when the route name differs from the field name. |
| `#[request_param]` | A required query value. Supports `name = "..."`, `default`, and `default = expr`. Wrapping in `Option<T>` makes it optional. |
| `#[request_header(name = ...)]` | A header value, given as a `HeaderName`. `Option<T>` makes it optional. |
| `#[request_body]` | A JSON request body. At most one body field per entity. |

When `name` is omitted from a path variable or request parameter, the Rust field name is used.

### Rejection type

Every `RequestEntity` must declare a user-defined rejection with `#[request_entity(rejection = ErrorType)]`. The rejection type must implement `IntoResponse`.

### Validation

Add `validate` to run the `validator` implementation after extraction:

```rust
use golden_boot::{RequestEntity, Validate};

#[derive(RequestEntity, Validate)]
#[request_entity(rejection = ApiError, validate)]
struct PageRequest {
    #[request_param]
    #[validate(range(min = 1, max = 100))]
    size: u32,
}
```

## Responses

### `ApiResponse`

A standard `code` / `msg` / `data` envelope:

```rust
use golden_boot::ApiResponse;

async fn handler() -> ApiResponse<String> {
    ApiResponse::success("hello".to_owned())
}
```

### `ResponseEntity`

A response with a status code, headers, and an optional JSON body:

```rust
use golden_boot::{ResponseEntity, header, get_mapping};

#[get_mapping("/created/{id}")]
async fn created() -> ResponseEntity<String> {
    ResponseEntity::created("done".to_owned())
        .try_with_header(header::X_REQUEST_ID.as_str(), "42")
        .unwrap()
}
```

### `Page`

A one-based paginated page:

```rust
use golden_boot::Page;

async fn handler() -> Page<Item> {
    let items = vec![/* ... */];
    Page::new(items, 1, 10, 100).unwrap()
}
```

## License

Licensed under either of [MIT](https://github.com/HoChienkuo/golden/blob/master/LICENSE-MIT) or [Apache-2.0](https://github.com/HoChienkuo/golden/blob/master/LICENSE-APACHE), at your option.
