use proc_macro2::TokenStream;
use proc_macro_crate::{
    crate_name,
    FoundCrate,
};
use quote::{
    format_ident,
    quote,
};

pub fn golden_boot() -> syn::Result<TokenStream> {
    match crate_name("golden-boot") {
        Ok(FoundCrate::Itself) => {
            Ok(quote!(crate))
        }

        Ok(FoundCrate::Name(name)) => {
            let identifier = format_ident!(
                "{}",
                name.replace('-', "_"),
            );

            Ok(quote!(::#identifier))
        }

        Err(error) => Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            format!(
                "unable to locate the `golden-boot` crate: {error}"
            ),
        )),
    }
}