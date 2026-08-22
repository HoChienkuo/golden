use crate::crate_path;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use std::collections::HashSet;
use syn::{Error, ItemFn, LitStr};

#[derive(Clone, Copy)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
    Trace,
    Connect,
}

impl HttpMethod {
    fn name(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Patch => "PATCH",
            Self::Delete => "DELETE",
            Self::Head => "HEAD",
            Self::Options => "OPTIONS",
            Self::Trace => "TRACE",
            Self::Connect => "CONNECT",
        }
    }

    fn routing_function(self) -> syn::Ident {
        let name = match self {
            Self::Get => "get",
            Self::Post => "post",
            Self::Put => "put",
            Self::Patch => "patch",
            Self::Delete => "delete",
            Self::Head => "head",
            Self::Options => "options",
            Self::Trace => "trace",
            Self::Connect => "connect",
        };

        syn::Ident::new(name, proc_macro2::Span::call_site())
    }

    fn register_function(self, handler_name: &syn::Ident) -> syn::Ident {
        let method = self.routing_function();

        format_ident!("__golden_register_{}_{}", method, handler_name,)
    }
}

pub fn expand(
    arguments: TokenStream,
    item: TokenStream,
    method: HttpMethod,
) -> syn::Result<TokenStream2> {
    let path = syn::parse::<LitStr>(arguments)?;
    let function = syn::parse::<ItemFn>(item)?;

    validate_path(&path)?;
    validate_handler(&function)?;

    let function_name = &function.sig.ident;
    let register_function = method.register_function(function_name);

    let routing_function = method.routing_function();
    let method_name = method.name();

    let golden_boot = crate_path::golden_boot()?;

    Ok(quote! {
        #function

        #[doc(hidden)]
        fn #register_function(
            router: #golden_boot::__private::axum::Router,
        ) -> #golden_boot::__private::axum::Router {
            router.route(
                #path,
                #golden_boot::__private::axum::routing::#routing_function(
                    #function_name
                ),
            )
        }

        #golden_boot::__private::inventory::submit! {
            #golden_boot::__private::RouteDefinition {
                method: #method_name,
                path: #path,
                handler_name: concat!(
                    module_path!(),
                    "::",
                    stringify!(#function_name),
                ),
                register: #register_function,
            }
        }
    })
}

fn validate_path(path: &LitStr) -> syn::Result<()> {
    let value = path.value();

    if value.is_empty() {
        return Err(Error::new_spanned(path, "mapping path cannot be empty"));
    }

    if !value.starts_with('/') {
        return Err(Error::new_spanned(path, "mapping path must start with `/`"));
    }

    if value.contains('?') {
        return Err(Error::new_spanned(
            path,
            "query parameters must not be declared in a mapping path",
        ));
    }

    if value.contains('#') {
        return Err(Error::new_spanned(
            path,
            "fragments must not be declared in a mapping path",
        ));
    }

    validate_path_segments(path, &value)
}

fn validate_handler(function: &ItemFn) -> syn::Result<()> {
    if function.sig.asyncness.is_none() {
        return Err(Error::new_spanned(
            function.sig.fn_token,
            "mapping handler must be async",
        ));
    }

    if !function.sig.generics.params.is_empty() {
        return Err(Error::new_spanned(
            &function.sig.generics,
            "mapping handler cannot declare generic parameters",
        ));
    }

    if function.sig.constness.is_some() {
        return Err(Error::new_spanned(
            function.sig.constness,
            "mapping handler cannot be const",
        ));
    }

    if function.sig.unsafety.is_some() {
        return Err(Error::new_spanned(
            function.sig.unsafety,
            "mapping handler cannot be unsafe",
        ));
    }

    Ok(())
}

fn validate_path_segments(path: &LitStr, value: &str) -> syn::Result<()> {
    let mut parameters = HashSet::new();

    for segment in value.split('/').skip(1) {
        if segment.is_empty() {
            if value == "/" {
                continue;
            }

            return Err(Error::new_spanned(
                path,
                "mapping path cannot contain an empty segment",
            ));
        }

        if segment.starts_with(':') {
            return Err(Error::new_spanned(
                path,
                "GoldenBoot only supports `{id}` path parameters",
            ));
        }

        if segment.starts_with('*') {
            return Err(Error::new_spanned(
                path,
                "wildcard path parameters are not supported",
            ));
        }

        let contains_open = segment.contains('{');
        let contains_close = segment.contains('}');

        if !contains_open && !contains_close {
            continue;
        }

        if !segment.starts_with('{')
            || !segment.ends_with('}')
            || segment.matches('{').count() != 1
            || segment.matches('}').count() != 1
        {
            return Err(Error::new_spanned(
                path,
                "path parameter must occupy an entire segment, such as `{id}`",
            ));
        }

        let parameter = &segment[1..segment.len() - 1];

        if parameter.is_empty() {
            return Err(Error::new_spanned(
                path,
                "path parameter name cannot be empty",
            ));
        }

        if parameter.starts_with('*') {
            return Err(Error::new_spanned(
                path,
                "wildcard path parameters are not supported",
            ));
        }

        if syn::parse_str::<syn::Ident>(parameter).is_err() {
            return Err(Error::new_spanned(
                path,
                "path parameter must be a valid Rust identifier",
            ));
        }

        if !parameters.insert(parameter.to_owned()) {
            return Err(Error::new_spanned(
                path,
                format!("duplicate path parameter `{parameter}`"),
            ));
        }
    }

    Ok(())
}
