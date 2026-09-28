use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{Error, FnArg, ItemFn, Pat, ReturnType, Type, parse::Parser};

/// Expands the `#[tool]` attribute macro.
pub fn expand(arguments: TokenStream, item: TokenStream) -> syn::Result<TokenStream2> {
    let function = syn::parse::<ItemFn>(item)?;

    validate_tool_function(&function)?;

    let attributes = parse_attributes(arguments)?;

    let function_name = &function.sig.ident;
    let register_ident = format_ident!("__golden_register_tool_{}", function_name);
    let schema_ident = format_ident!("__golden_schema_tool_{}", function_name);

    let tool_name = attributes.name.unwrap_or_else(|| function_name.to_string());
    let description = attributes
        .description
        .unwrap_or_else(|| extract_docs(&function));

    let golden_agent = crate::crate_path::golden_agent()?;

    // Build the JSON Schema body from the function's parameters.
    let schema_body = build_schema_body(&function, &golden_agent)?;

    // Parameter names and their types, used to deserialize the call arguments.
    let params = collect_params(&function)?;
    let param_names: Vec<&syn::Ident> = params.iter().map(|p| &p.name).collect();
    let param_types: Vec<&Type> = params.iter().map(|p| &p.ty).collect();

    // The return type, used to type the output. Defaults to serde_json::Value.
    let return_ty = match &function.sig.output {
        ReturnType::Default => quote!(#golden_agent::__private::serde_json::Value),
        ReturnType::Type(_, ty) => quote!(#ty),
    };

    Ok(quote! {
        #function

        #[doc(hidden)]
        fn #schema_ident() -> #golden_agent::__private::serde_json::Value {
            #schema_body
        }

        #[doc(hidden)]
        fn #register_ident(
            __golden_args: #golden_agent::__private::serde_json::Value,
        ) -> #golden_agent::__private::futures_util::future::BoxFuture<
            'static,
            ::std::result::Result<String, #golden_agent::Error>,
        > {
            ::std::boxed::Box::pin(async move {
                // Deserialize each parameter from the JSON arguments object.
                let __golden_args_obj = __golden_args
                    .as_object()
                    .cloned()
                    .unwrap_or_default();

                #(
                    let #param_names: #param_types =
                        match #golden_agent::__private::serde_json::from_value(
                            __golden_args_obj.get(stringify!(#param_names)).cloned().unwrap_or(#golden_agent::__private::serde_json::Value::Null)
                        ) {
                            ::std::result::Result::Ok(v) => v,
                            ::std::result::Result::Err(e) => {
                                return ::std::result::Result::Err(
                                    #golden_agent::Error::Stream(e.to_string())
                                );
                            }
                        };
                )*

                // Run the function to completion.
                let __golden_output: #return_ty = #function_name(#(#param_names,)*).await;

                // Serialize the result back to a JSON string.
                #golden_agent::__private::serde_json::to_string(&__golden_output)
                    .map_err(|e| #golden_agent::Error::Stream(e.to_string()))
            })
        }

        #golden_agent::__private::inventory::submit! {
            #golden_agent::__private::ToolDefinition {
                name: #tool_name,
                description: #description,
                schema: #schema_ident,
                call: #register_ident,
            }
        }
    })
}

struct ToolAttributes {
    name: Option<String>,
    description: Option<String>,
}

fn parse_attributes(arguments: TokenStream) -> syn::Result<ToolAttributes> {
    let mut name = None;
    let mut description = None;

    if arguments.is_empty() {
        return Ok(ToolAttributes { name, description });
    }

    let parser =
        syn::punctuated::Punctuated::<syn::MetaNameValue, syn::Token![,]>::parse_terminated;
    let arguments = parser.parse(arguments)?;

    for argument in &arguments {
        if argument.path.is_ident("name") {
            let value = match &argument.value {
                syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(s),
                    ..
                }) => s.value(),
                _ => {
                    return Err(Error::new_spanned(
                        &argument.value,
                        "`name` must be a string literal",
                    ));
                }
            };
            name = Some(value);
        } else if argument.path.is_ident("description") {
            let value = match &argument.value {
                syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(s),
                    ..
                }) => s.value(),
                _ => {
                    return Err(Error::new_spanned(
                        &argument.value,
                        "`description` must be a string literal",
                    ));
                }
            };
            description = Some(value);
        } else {
            return Err(Error::new_spanned(
                &argument.path,
                "unsupported `#[tool]` argument; expected `name` or `description`",
            ));
        }
    }

    Ok(ToolAttributes { name, description })
}

fn extract_docs(function: &ItemFn) -> String {
    let docs: Vec<String> = function
        .attrs
        .iter()
        .filter_map(|attr| {
            attr.meta
                .require_name_value()
                .ok()
                .filter(|nv| nv.path.is_ident("doc"))
                .and_then(|nv| match &nv.value {
                    syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(s),
                        ..
                    }) => Some(s.value()),
                    _ => None,
                })
        })
        .collect();

    docs.join("\n").trim().to_string()
}

struct Param {
    name: syn::Ident,
    ty: Type,
}

fn collect_params(function: &ItemFn) -> syn::Result<Vec<Param>> {
    let mut params = Vec::new();

    for input in &function.sig.inputs {
        let FnArg::Typed(pat_type) = input else {
            continue;
        };

        let Pat::Ident(pat_ident) = pat_type.pat.as_ref() else {
            return Err(Error::new_spanned(
                &pat_type.pat,
                "`#[tool]` parameters must be simple identifiers (no patterns)",
            ));
        };

        params.push(Param {
            name: pat_ident.ident.clone(),
            ty: (*pat_type.ty).clone(),
        });
    }

    Ok(params)
}

fn build_schema_body(function: &ItemFn, golden_agent: &TokenStream2) -> syn::Result<TokenStream2> {
    let params = collect_params(function)?;

    // Collect the per-parameter schema expressions and their names.
    let mut name_tokens = Vec::new();
    let mut schema_tokens = Vec::new();

    for param in &params {
        let name = param.name.to_string();
        let ty_schema = type_to_schema(&param.ty, golden_agent)?;
        name_tokens.push(name);
        schema_tokens.push(ty_schema);
    }

    Ok(quote! {
        {
            let mut __schema = #golden_agent::__private::serde_json::Map::new();
            __schema.insert(
                "type".to_string(),
                #golden_agent::__private::serde_json::Value::String("object".to_string()),
            );
            let mut __properties = #golden_agent::__private::serde_json::Map::new();
            #(
                __properties.insert(#name_tokens.to_string(), #schema_tokens);
            )*
            __schema.insert(
                "properties".to_string(),
                #golden_agent::__private::serde_json::Value::Object(__properties),
            );
            let __required: ::std::vec::Vec<#golden_agent::__private::serde_json::Value> =
                ::std::vec![#(#golden_agent::__private::serde_json::Value::String(#name_tokens.to_string())),*];
            __schema.insert(
                "required".to_string(),
                #golden_agent::__private::serde_json::Value::Array(__required),
            );
            #golden_agent::__private::serde_json::Value::Object(__schema)
        }
    })
}

/// Maps a Rust type to a JSON Schema expression (a `serde_json::Value` builder).
fn type_to_schema(ty: &Type, golden_agent: &TokenStream2) -> syn::Result<TokenStream2> {
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
        "u8" | "u16" | "u32" | "u64" | "usize" | "i8" | "i16" | "i32" | "i64" | "isize" | "f32"
        | "f64" => Ok(primitive("number")),
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
        _ => Ok(primitive("object")),
    }
}

fn first_type_argument(segment: &syn::PathSegment) -> syn::Result<&Type> {
    let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return Err(Error::new_spanned(segment, "expected a type argument"));
    };

    let Some(syn::GenericArgument::Type(inner)) = arguments.args.first() else {
        return Err(Error::new_spanned(segment, "expected a type argument"));
    };

    Ok(inner)
}

fn validate_tool_function(function: &ItemFn) -> syn::Result<()> {
    if function.sig.asyncness.is_none() {
        return Err(Error::new_spanned(
            function.sig.fn_token,
            "`#[tool]` can only be applied to async functions",
        ));
    }

    if !function.sig.generics.params.is_empty() {
        return Err(Error::new_spanned(
            &function.sig.generics,
            "`#[tool]` functions cannot have generic parameters",
        ));
    }

    if function.sig.constness.is_some() {
        return Err(Error::new_spanned(
            function.sig.constness,
            "`#[tool]` functions cannot be const",
        ));
    }

    if function.sig.unsafety.is_some() {
        return Err(Error::new_spanned(
            function.sig.unsafety,
            "`#[tool]` functions cannot be unsafe",
        ));
    }

    Ok(())
}
