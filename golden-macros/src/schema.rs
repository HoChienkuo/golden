//! Shared JSON Schema helpers for `#[tool]` and `#[derive(ToolSchema)]`.
//!
//! Both macros describe parameters the same way — a type maps to a schema
//! expression, `#[param(...)]` metadata folds into it — so the rules live here
//! and stay identical between a function parameter and a struct field.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Error, Type, parse::Parser};

/// The `#[param(...)]` metadata read from a parameter or field.
#[derive(Default)]
pub struct ParamAttributes {
    /// The `description = "..."` value, if present.
    pub description: Option<String>,
    /// The `required = ...` override, if present.
    pub required: Option<bool>,
}

/// Reads `#[param(...)]` metadata from a parameter's or field's attributes.
///
/// The attributes themselves are consumed here and never reach the expanded
/// output of `#[tool]`, so `#[param]` needs no backing macro.
pub fn parse_param_attributes(attributes: &[syn::Attribute]) -> syn::Result<ParamAttributes> {
    let mut result = ParamAttributes::default();

    for attribute in attributes {
        if !attribute.path().is_ident("param") {
            continue;
        }

        let parser =
            syn::punctuated::Punctuated::<syn::MetaNameValue, syn::Token![,]>::parse_terminated;

        let syn::Meta::List(list) = &attribute.meta else {
            return Err(Error::new_spanned(
                attribute,
                "expected `#[param(...)]` with named arguments",
            ));
        };

        let arguments = parser.parse2(list.tokens.clone())?;

        for argument in &arguments {
            if argument.path.is_ident("description") {
                result.description = Some(string_literal(&argument.value, "description")?);
            } else if argument.path.is_ident("required") {
                result.required = Some(bool_literal(&argument.value, "required")?);
            } else {
                return Err(Error::new_spanned(
                    &argument.path,
                    "unsupported `#[param]` argument; expected `description` or `required`",
                ));
            }
        }
    }

    Ok(result)
}

/// Extracts the value of a string literal, reporting `name` on failure.
fn string_literal(expr: &syn::Expr, name: &str) -> syn::Result<String> {
    match expr {
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Str(s),
            ..
        }) => Ok(s.value()),
        _ => Err(Error::new_spanned(
            expr,
            format!("`{name}` must be a string literal"),
        )),
    }
}

/// Extracts the value of a boolean literal, reporting `name` on failure.
fn bool_literal(expr: &syn::Expr, name: &str) -> syn::Result<bool> {
    match expr {
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Bool(b),
            ..
        }) => Ok(b.value),
        _ => Err(Error::new_spanned(
            expr,
            format!("`{name}` must be `true` or `false`"),
        )),
    }
}

/// Whether the type is an `Option<T>` (by its last path segment).
pub fn is_option(ty: &Type) -> bool {
    matches!(ty, Type::Path(type_path)
        if type_path.path.segments.last().is_some_and(|segment| segment.ident == "Option"))
}

/// Builds the JSON Schema object for a list of named parameters.
///
/// `fields` pairs each name with the tokens producing its schema; `required`
/// names the parameters that must be present. Used by both `#[tool]` (the
/// tool's top-level `parameters`) and `#[derive(ToolSchema)]` (the struct's
/// schema, which is itself such an object).
pub fn object_schema(
    golden_agent: &TokenStream,
    fields: &[(String, TokenStream)],
    required: &[String],
) -> TokenStream {
    let names: Vec<&str> = fields.iter().map(|(name, _)| name.as_str()).collect();
    let schemas: Vec<&TokenStream> = fields.iter().map(|(_, schema)| schema).collect();

    quote! {
        {
            let mut __schema = #golden_agent::__private::serde_json::Map::new();
            __schema.insert(
                "type".to_string(),
                #golden_agent::__private::serde_json::Value::String("object".to_string()),
            );
            let mut __properties = #golden_agent::__private::serde_json::Map::new();
            #(
                __properties.insert(#names.to_string(), #schemas);
            )*
            __schema.insert(
                "properties".to_string(),
                #golden_agent::__private::serde_json::Value::Object(__properties),
            );
            let __required: ::std::vec::Vec<#golden_agent::__private::serde_json::Value> =
                ::std::vec![#(#golden_agent::__private::serde_json::Value::String(#required.to_string())),*];
            __schema.insert(
                "required".to_string(),
                #golden_agent::__private::serde_json::Value::Array(__required),
            );
            #golden_agent::__private::serde_json::Value::Object(__schema)
        }
    }
}

/// Folds an optional `#[param(description = "...")]` into a schema expression.
pub fn with_description(
    golden_agent: &TokenStream,
    schema: TokenStream,
    description: Option<&String>,
) -> TokenStream {
    match description {
        Some(description) => quote! {
            {
                let mut __field = #schema;
                if let #golden_agent::__private::serde_json::Value::Object(ref mut __map) =
                    __field
                {
                    __map.insert(
                        "description".to_string(),
                        #golden_agent::__private::serde_json::Value::String(
                            #description.to_string(),
                        ),
                    );
                }
                __field
            }
        },
        None => schema,
    }
}

/// Maps a Rust type to a JSON Schema expression (a `serde_json::Value` builder).
///
/// Primitive types map directly; any other path type is expected to implement
/// `golden_agent::ToolSchema`, which is how a custom parameter type exposes its
/// fields to the model instead of collapsing to `{"type": "object"}`.
pub fn type_to_schema(ty: &Type, golden_agent: &TokenStream) -> syn::Result<TokenStream> {
    let Type::Path(type_path) = ty else {
        return Err(Error::new_spanned(
            ty,
            "unsupported `#[tool]` parameter type",
        ));
    };

    let Some(segment) = type_path.path.segments.last() else {
        return Err(Error::new_spanned(
            ty,
            "unsupported `#[tool]` parameter type",
        ));
    };

    let ident = segment.ident.to_string();

    let primitive = |kind: &str| {
        quote! {
            #golden_agent::__private::serde_json::json!({ "type": #kind })
        }
    };

    match ident.as_str() {
        "String" | "str" | "&str" => Ok(primitive("string")),
        "u8" | "u16" | "u32" | "u64" | "usize" | "i8" | "i16" | "i32" | "i64" | "isize" => {
            Ok(primitive("integer"))
        }
        "f32" | "f64" => Ok(primitive("number")),
        "bool" => Ok(primitive("boolean")),
        "Vec" => {
            let inner = first_type_argument(segment)?;
            let inner_schema = type_to_schema(inner, golden_agent)?;
            Ok(quote! {
                #golden_agent::__private::serde_json::json!({ "type": "array", "items": #inner_schema })
            })
        }
        "Option" => {
            let inner = first_type_argument(segment)?;
            type_to_schema(inner, golden_agent)
        }
        // A custom type: it must implement `ToolSchema`, so the model sees the
        // type's fields instead of a bare object.
        _ => Ok(quote! {
            <#ty as #golden_agent::ToolSchema>::schema()
        }),
    }
}

/// Returns the first angle-bracketed type argument of a path segment.
pub fn first_type_argument(segment: &syn::PathSegment) -> syn::Result<&Type> {
    let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return Err(Error::new_spanned(segment, "expected a type argument"));
    };

    let Some(syn::GenericArgument::Type(inner)) = arguments.args.first() else {
        return Err(Error::new_spanned(segment, "expected a type argument"));
    };

    Ok(inner)
}
