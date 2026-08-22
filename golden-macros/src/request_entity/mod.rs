mod expand;
mod model;
mod parse;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use syn::DeriveInput;

pub fn expand(input: TokenStream) -> syn::Result<TokenStream2> {
    let input = syn::parse::<DeriveInput>(input)?;

    expand::expand(input)
}
