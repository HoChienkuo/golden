mod application;
mod crate_path;
mod mappings;
mod request_entity;

use crate::mappings::HttpMethod;
use proc_macro::TokenStream;

/// Marks an async `main` function as a GoldenBoot application entry point.
///
/// The Axum server listens on port `8080` by default.
///
/// # Example
///
/// ```ignore
/// use golden_boot::golden_boot_application;
///
/// #[golden_boot_application(port = 9090)]
/// async fn main() {}
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
