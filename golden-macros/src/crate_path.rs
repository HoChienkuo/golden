use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

pub fn golden_boot() -> syn::Result<TokenStream> {
    match crate_name("golden-boot") {
        Ok(FoundCrate::Itself) => Ok(quote!(crate)),

        Ok(FoundCrate::Name(name)) => {
            let identifier = format_ident!("{}", name.replace('-', "_"),);

            Ok(quote!(::#identifier))
        }

        Err(error) => Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            format!("unable to locate the `golden-boot` crate: {error}"),
        )),
    }
}

pub fn golden_agent() -> syn::Result<TokenStream> {
    match crate_name("golden-agent") {
        // A bare `crate` would break in this crate's own examples and tests,
        // which are separate crates. `::golden_agent` resolves everywhere: the
        // lib aliases itself with `extern crate self as golden_agent`, and
        // examples/tests see the lib as an extern crate.
        Ok(FoundCrate::Itself) => Ok(quote!(::golden_agent)),

        Ok(FoundCrate::Name(name)) => {
            let identifier = format_ident!("{}", name.replace('-', "_"));

            Ok(quote!(::#identifier))
        }

        Err(error) => Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            format!("unable to locate the `golden-agent` crate: {error}"),
        )),
    }
}
