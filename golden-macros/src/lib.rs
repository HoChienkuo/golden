mod application;
mod crate_path;
mod mappings;
mod request_entity;
#[cfg(feature = "agent")]
mod schema;
#[cfg(feature = "agent")]
mod tool_attr;
#[cfg(feature = "agent")]
mod tool_schema;

use crate::mappings::HttpMethod;
use proc_macro::TokenStream;

/// Marks an async `main` function as a GoldenBoot application entry point.
///
/// The Axum server listens on port `8080` by default.
/// Return a cloneable value to expose application state through Axum's
/// `State<T>` extractor. Returning `()` creates a stateless application.
/// Every automatically registered `State<T>` handler must use the same `T`
/// returned by the application entry point.
///
/// Write the extractor directly as `State<AppState>` or
/// `golden_boot::State<AppState>`. Type aliases for `State<T>` cannot be
/// recognized during procedural macro expansion. Application state should be
/// cheap to clone, typically by containing `Arc<T>`, connection pools, and
/// other shared handles rather than large owned collections.
///
/// Initialization may fail by returning a type whose final path segment is
/// named `Result`, such as `Result<AppState, E>`, `std::io::Result<AppState>`,
/// or `anyhow::Result<AppState>`. Its error must implement
/// `Error + Send + Sync + 'static`. The original error is kept as the source of
/// `ApplicationError::Initialization`. A differently named type alias for
/// `Result` cannot be recognized during procedural macro expansion.
///
/// # Example
///
/// ```ignore
/// use golden_boot::golden_boot_application;
/// use std::sync::Arc;
///
/// #[golden_boot_application(port = 9090)]
/// async fn main() -> AppState {
///     AppState {
///         article_service: Arc::new(ArticleService::new()),
///     }
/// }
/// ```
///
/// A fallible initializer can use `?` normally:
///
/// ```ignore
/// #[golden_boot_application]
/// async fn main() -> Result<AppState, StartupError> {
///     let database = connect_database().await?;
///     Ok(AppState::new(database))
/// }
/// ```
#[proc_macro_attribute]
pub fn golden_boot_application(arguments: TokenStream, item: TokenStream) -> TokenStream {
    application::expand(arguments, item)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Maps an async handler to an HTTP GET request.
#[proc_macro_attribute]
pub fn get_mapping(arguments: TokenStream, item: TokenStream) -> TokenStream {
    expand_mapping(arguments, item, HttpMethod::Get)
}

/// Maps an async handler to an HTTP POST request.
#[proc_macro_attribute]
pub fn post_mapping(arguments: TokenStream, item: TokenStream) -> TokenStream {
    expand_mapping(arguments, item, HttpMethod::Post)
}

/// Maps an async handler to an HTTP PUT request.
#[proc_macro_attribute]
pub fn put_mapping(arguments: TokenStream, item: TokenStream) -> TokenStream {
    expand_mapping(arguments, item, HttpMethod::Put)
}

/// Maps an async handler to an HTTP PATCH request.
#[proc_macro_attribute]
pub fn patch_mapping(arguments: TokenStream, item: TokenStream) -> TokenStream {
    expand_mapping(arguments, item, HttpMethod::Patch)
}

/// Maps an async handler to an HTTP DELETE request.
#[proc_macro_attribute]
pub fn delete_mapping(arguments: TokenStream, item: TokenStream) -> TokenStream {
    expand_mapping(arguments, item, HttpMethod::Delete)
}

/// Maps an async handler to an HTTP HEAD request.
#[proc_macro_attribute]
pub fn head_mapping(arguments: TokenStream, item: TokenStream) -> TokenStream {
    expand_mapping(arguments, item, HttpMethod::Head)
}

/// Maps an async handler to an HTTP OPTIONS request.
#[proc_macro_attribute]
pub fn options_mapping(arguments: TokenStream, item: TokenStream) -> TokenStream {
    expand_mapping(arguments, item, HttpMethod::Options)
}

/// Maps an async handler to an HTTP TRACE request.
#[proc_macro_attribute]
pub fn trace_mapping(arguments: TokenStream, item: TokenStream) -> TokenStream {
    expand_mapping(arguments, item, HttpMethod::Trace)
}

/// Maps an async handler to an HTTP CONNECT request.
#[proc_macro_attribute]
pub fn connect_mapping(arguments: TokenStream, item: TokenStream) -> TokenStream {
    expand_mapping(arguments, item, HttpMethod::Connect)
}

fn expand_mapping(arguments: TokenStream, item: TokenStream, method: HttpMethod) -> TokenStream {
    mappings::expand(arguments, item, method)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Derives a GoldenBoot request extractor for a named-field struct.
///
/// The struct must declare its user-defined rejection type with
/// `#[request_entity(rejection = ErrorType)]`. Add `validate` to run its
/// [`validator::Validate`](https://docs.rs/validator/latest/validator/trait.Validate.html)
/// implementation after extraction.
///
/// # Field attributes
///
/// - `#[path_variable]` extracts a required path value. Use
///   `#[path_variable(name = "id")]` when the route name differs from the
///   Rust field name. `Option<T>` is not supported.
/// - `#[request_param]` extracts a required query value. It supports
///   `name = "..."`, `default`, and `default = expression`. `Option<T>` makes
///   the value optional and cannot be combined with `default`.
/// - `#[request_header(name = header_name)]` extracts a header. The name must
///   be an HTTP `HeaderName` expression. `Option<T>` makes the header optional.
/// - `#[request_body]` extracts a JSON body. A request entity can contain at
///   most one body field.
///
/// When `name` is omitted from a path variable or request parameter, the Rust
/// field name is used.
///
/// # Example
///
/// ```ignore
/// use golden_boot::{RequestEntity, header};
///
/// #[derive(RequestEntity)]
/// #[request_entity(rejection = ApiError)]
/// struct GetArticleRequest {
///     #[path_variable(name = "id")]
///     article_id: u64,
///
///     #[request_param(default = 1)]
///     page: u32,
///
///     #[request_header(name = header::AUTHORIZATION)]
///     authorization: Option<String>,
/// }
/// ```
#[proc_macro_derive(
    RequestEntity,
    attributes(
        request_entity,
        path_variable,
        request_param,
        request_header,
        request_body
    )
)]
pub fn derive_request_entity(item: TokenStream) -> TokenStream {
    request_entity::expand(item)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Registers an `async fn` as a tool callable by a language model.
///
/// The tool's name defaults to the function name and its description to the
/// doc comment. Both can be overridden with `#[tool(name = "...",
/// description = "...")]`.
///
/// Parameters are derived from the function signature and exposed to the model
/// as a JSON Schema. The return value is serialized to JSON and fed back to the
/// model.
///
/// A parameter's schema entry defaults to its Rust type. Describe a parameter
/// for the model with `#[param(description = "...")]`, and override whether it
/// is listed as required with `#[param(required = false)]` (by default
/// `Option<T>` parameters are optional and all others are required).
///
/// A parameter whose type is not a primitive (`String`, an integer, a float,
/// `bool`, `Vec<T>` or `Option<T>`) must implement
/// `ToolSchema` so the model can see its fields
/// instead of a bare `{"type": "object"}`; derive it with
/// `#[derive(ToolSchema)]` and annotate its fields with the same `#[param(...)]`
/// attributes.
///
/// # Example
///
/// ```ignore
/// use golden_agent::tool;
///
/// /// Get the current weather for a city.
/// #[tool]
/// async fn get_weather(
///     #[param(description = "City name, e.g. Taipei")]
///     city: String,
///     #[param(description = "Temperature unit: celsius or fahrenheit")]
///     unit: Option<String>,
/// ) -> String {
///     format!("{city}: 20°C")
/// }
/// ```
///
/// A custom parameter type expands its fields into the schema:
///
/// ```ignore
/// use golden_agent::ToolSchema;
///
/// #[derive(serde::Deserialize, ToolSchema)]
/// struct Order {
///     #[param(description = "Name of the item to order")]
///     item: String,
///     #[param(description = "How many units to order")]
///     quantity: u32,
/// }
/// ```
#[cfg(feature = "agent")]
#[proc_macro_attribute]
pub fn tool(arguments: TokenStream, item: TokenStream) -> TokenStream {
    tool_attr::expand(arguments, item)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Derives a JSON Schema for a struct used as a `#[tool]` parameter.
///
/// Every field becomes a property in the generated schema, so the model sees
/// the struct's shape instead of a bare `{"type": "object"}`. Field types are
/// mapped with the same rules `#[tool]` uses for function parameters:
/// primitives map to their JSON type, `Vec<T>` to an array, `Option<T>` to an
/// optional (non-required) property, and any other type must itself implement
/// `ToolSchema`.
///
/// # Field attributes
///
/// - `#[param(description = "...")]` documents the field for the model. When
///   absent, the field's `///` doc comment is used instead.
/// - `#[param(required = false)]` removes the field from the schema's
///   `required` list. By default every field is required except `Option<T>`.
///
/// # Limitations
///
/// The struct must be concrete. Deriving on a struct with type, lifetime, or
/// `const` generic parameters is a compile error: a correct impl would have to
/// forward them (`impl<T> ToolSchema for Paged<T>`) and bound every generic
/// field type (`T: ToolSchema`), which the derive cannot infer on its own.
/// Wrap the generic type in a concrete struct instead.
///
/// # Example
///
/// ```ignore
/// use golden_agent::ToolSchema;
///
/// #[derive(serde::Deserialize, ToolSchema)]
/// struct Order {
///     /// Name of the item to order.
///     item: String,
///
///     /// How many units to order; `#[param]` overrides are still available.
///     quantity: u32,
/// }
/// ```
#[cfg(feature = "agent")]
#[proc_macro_derive(ToolSchema, attributes(param))]
pub fn derive_tool_schema(item: TokenStream) -> TokenStream {
    tool_schema::expand(item)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
