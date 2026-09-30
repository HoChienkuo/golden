use syn::{Data, DeriveInput, Error, Fields};

use super::model::SchemaField;
use crate::schema::parse_param_attributes;

/// Reads the named struct's fields and their `#[param(...)]` metadata.
pub fn parse_fields(input: &DeriveInput) -> syn::Result<Vec<SchemaField>> {
    let Data::Struct(data) = &input.data else {
        return Err(Error::new_spanned(
            input,
            "ToolSchema can only be derived for structs",
        ));
    };

    let Fields::Named(fields) = &data.fields else {
        return Err(Error::new_spanned(
            input,
            "ToolSchema requires named fields",
        ));
    };

    let mut result = Vec::new();

    for field in &fields.named {
        let Some(ident) = field.ident.clone() else {
            return Err(Error::new_spanned(
                field,
                "ToolSchema requires named fields",
            ));
        };

        let meta = parse_param_attributes(&field.attrs)?;

        result.push(SchemaField {
            ident,
            ty: field.ty.clone(),
            description: meta.description,
            required: meta.required,
        });
    }

    Ok(result)
}
