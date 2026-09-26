# GoldenBoot

GoldenBoot 是一个构建于 [Axum](https://github.com/tokio-rs/axum) 之上的、注解驱动的 Web 框架。

它为你提供了简洁的应用入口，并把运行时、路由以及 HTTP 服务器的装配细节从应用代码中剥离出去。

[English Docs](README.md)

## 特性

- **注解驱动的控制器注册** — 使用 `#[get_mapping]`、`#[post_mapping]` 等注解将处理器映射到 HTTP 方法。路由会被自动发现并注册，无需手工组装 Router。
- **简洁的入口** — 用 `#[golden_boot_application]` 标注 `async fn main`，即可启动运行时并对外提供 HTTP 服务。
- **`State<T>` 应用状态** — 从 `main` 返回一个 `Clone + Send + Sync` 的值，然后通过 Axum 的 `State<T>` 提取器访问它。
- **丰富的请求提取** — 通过派生 `RequestEntity`，把路径变量、查询参数、请求头和 JSON 请求体聚合到一个结构体中。
- **校验** — 结合 `validator` 的 `#[derive(Validate)]` 实现请求自动校验。
- **内置响应助手** — `ApiResponse`、`ResponseEntity`、`ResponseEntityBuilder` 和 `Page`，覆盖常见的 REST 与分页响应场景。

## 安装

在 `Cargo.toml` 中添加 GoldenBoot：

```toml
[dependencies]
golden-boot = "0.1"
serde = { version = "1", features = ["derive"] }
```

> GoldenBoot 已经重新导出了你需要的 `validator`、`serde` 以及 Axum 的 Web 类型，因此你通常无需再直接依赖它们。

## 快速开始

```rust
use golden_boot::golden_boot_application;

#[golden_boot_application]
async fn main() {
    println!("initializing application");
}
```

运行后，服务默认监听 `http://0.0.0.0:8080`：

```console
$ cargo run
GoldenBoot started
Listening on http://0.0.0.0:8080
```

使用 `port` 参数指定其他端口：

```rust
#[golden_boot_application(port = 9090)]
async fn main() {}
```

## 路由

使用映射注解把任意 `async fn` 映射到 HTTP 方法：

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

支持的注解：`get_mapping`、`post_mapping`、`put_mapping`、`patch_mapping`、`delete_mapping`、`head_mapping`、`options_mapping`、`trace_mapping` 和 `connect_mapping`。

### 路径参数

使用 `{name}` 段。直接写出提取器 `Path<T>`，GoldenBoot 会自动完成装配：

```rust
use golden_boot::{Path, get_mapping};

#[get_mapping("/articles/{id}")]
async fn get_article(Path(id): Path<u64>) -> String {
    format!("article {id}")
}
```

### 应用状态

从入口返回一个 `Clone + Send + Sync + 'static` 的值，再通过 `State<T>` 读取：

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

所有标注了 `State<T>` 的处理器必须使用与 `main` 返回值相同的 `T`。请直接写出 `State<AppState>`（或 `golden_boot::State<AppState>`）——类型别名在宏展开过程中无法被识别。

可失败的初始化函数可以返回 `Result`，并正常使用 `?`：

```rust
#[golden_boot_application]
async fn main() -> Result<AppState, std::io::Error> {
    let database = connect_database().await?;
    Ok(AppState { database })
}
```

## 请求实体

派生 `RequestEntity`，把路径变量、查询参数、请求头和 JSON 请求体聚合到一个结构体：

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

字段属性：

| 属性 | 说明 |
| --- | --- |
| `#[path_variable]` | 必需的路径值。当路由名与字段名不同时，使用 `#[path_variable(name = "id")]`。 |
| `#[request_param]` | 必需的查询参数。支持 `name = "..."`、`default` 以及 `default = expr`。用 `Option<T>` 包裹可使其变为可选。 |
| `#[request_header(name = ...)]` | 请求头，以 `HeaderName` 表示。`Option<T>` 可使其变为可选。 |
| `#[request_body]` | JSON 请求体。每个实体最多只能有一个请求体字段。 |

当路径变量或请求参数省略 `name` 时，将使用 Rust 字段名。

### 拒绝类型（rejection）

每个 `RequestEntity` 都必须通过 `#[request_entity(rejection = ErrorType)]` 声明一个用户自定义的拒绝类型，该类型必须实现 `IntoResponse`。

### 校验

添加 `validate`，在提取完成后执行 `validator` 的实现：

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

## 响应

### `ApiResponse`

标准的 `code` / `msg` / `data` 结构：

```rust
use golden_boot::ApiResponse;

async fn handler() -> ApiResponse<String> {
    ApiResponse::success("hello".to_owned())
}
```

### `ResponseEntity`

包含状态码、响应头和可选 JSON 响应体的响应：

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

从 1 开始计数的分页结果：

```rust
use golden_boot::Page;

async fn handler() -> Page<Item> {
    let items = vec![/* ... */];
    Page::new(items, 1, 10, 100).unwrap()
}
```

## 许可证

在你选择的任一许可证下授权：[MIT](https://github.com/HoChienkuo/golden/blob/master/LICENSE-MIT) 或 [Apache-2.0](https://github.com/HoChienkuo/golden/blob/master/LICENSE-APACHE)。
