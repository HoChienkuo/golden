mod application;

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
