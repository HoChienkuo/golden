use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Error};

use super::parse::parse_fields;
use crate::schema::{object_schema, type_to_schema, with_description};

pub fn expand(input: DeriveInput) -> syn::Result<TokenStream> {
    // A generic struct cannot be supported honestly: the generated impl would
    // have to forward the parameters (`impl<T> ToolSchema for Paged<T>`) and
    // bound every field type (`T: ToolSchema`), which the derive cannot infer.
    // Rejecting beats emitting an impl that fails with a confusing mismatch.
    if !input.generics.params.is_empty() {
        return Err(Error::new_spanned(
            &input.generics,
            "`ToolSchema` does not support generic structs; \
             wrap the generic type in a concrete struct instead",
        ));
    }

    let ident = input.ident.clone();
    let fields = parse_fields(&input)?;

    let golden_agent = crate::crate_path::golden_agent()?;

    let mut schema_fields = Vec::new();
    let mut required_names = Vec::new();

    for field in &fields {
        let name = field.ident.to_string();
        let field_schema = type_to_schema(&field.ty, &golden_agent)?;
        let field_schema =
            with_description(&golden_agent, field_schema, field.description.as_ref());

        if field
            .required
            .unwrap_or(!crate::schema::is_option(&field.ty))
        {
            required_names.push(name.clone());
        }

        schema_fields.push((name, field_schema));
    }

    let schema = object_schema(&golden_agent, &schema_fields, &required_names);

    Ok(quote! {
        impl #golden_agent::ToolSchema for #ident {
            fn schema() -> #golden_agent::__private::serde_json::Value {
                #schema
            }
        }
    })
}
