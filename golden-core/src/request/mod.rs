mod error;

pub use error::RequestEntityError;

/// A structured HTTP request extracted by GoldenBoot.
///
/// Implementations are normally generated with
/// `#[derive(RequestEntity)]`.
///
/// Fields declare where their values come from by using
/// `#[path_variable]`, `#[request_param]`,
/// `#[request_header]`, or `#[request_body]`.
///
/// # Example
///
/// ```ignore
/// use golden_boot::{
///     header,
///     RequestEntity,
/// };
///
/// #[derive(RequestEntity)]
/// #[request_entity(rejection = ApiError)]
/// struct CreateArticleRequest<T> {
///     #[path_variable]
///     id: u64,
///
///     #[request_param(default = 1)]
///     page: u32,
///
///     #[request_header(
///         name = header::AUTHORIZATION
///     )]
///     authorization: String,
///
///     #[request_body]
///     body: T,
/// }
/// ```
pub trait RequestEntity:
Sized + Send + 'static
{
}