use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{DeriveInput, Generics, LitStr, Type, parse_quote};

use crate::crate_path;

use super::{
    model::{DefaultValue, FieldSource, RequestField, RequestOptions},
    parse,
};

pub fn expand(input: DeriveInput) -> syn::Result<TokenStream2> {
    let golden_boot = crate_path::golden_boot()?;
    let options = parse::parse_options(&input)?;

    let fields = parse::named_fields(&input)?
        .iter()
        .map(parse::parse_field)
        .collect::<syn::Result<Vec<_>>>()?;

    let body_count = fields
        .iter()
        .filter(|field| matches!(field.source, FieldSource::RequestBody))
        .count();

    if body_count > 1 {
        return Err(syn::Error::new_spanned(
            &input.ident,
            "a RequestEntity can contain at most one body-consuming field",
        ));
    }

    if body_count == 0 {
        expand_parts(&input, &fields, &options, &golden_boot)
    } else {
        expand_request(&input, &fields, &options, &golden_boot)
    }
}

fn expand_parts(
    input: &DeriveInput,
    fields: &[RequestField],
    options: &RequestOptions,
    golden_boot: &TokenStream2,
) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let rejection = &options.rejection;

    let (_, type_generics, _) = input.generics.split_for_impl();

    let marker_generics = marker_generics(input);

    let (marker_impl_generics, _, marker_where_clause) = marker_generics.split_for_impl();

    let extractor_generics = extractor_generics(input, fields, options, golden_boot);

    let (impl_generics, _, where_clause) = extractor_generics.split_for_impl();

    let has_path = fields
        .iter()
        .any(|field| matches!(field.source, FieldSource::PathVariable));

    let has_query = fields
        .iter()
        .any(|field| matches!(field.source, FieldSource::RequestParam { .. }));

    let path_setup = generate_path_setup(
        has_path,
        quote!(__golden_parts),
        rejection,
        golden_boot,
    );

    let query_setup = generate_query_setup(
        has_query,
        quote!(__golden_parts),
        rejection,
        golden_boot,
    );

    let extractions = fields
        .iter()
        .map(|field| generate_field_extraction(field, rejection, golden_boot))
        .collect::<syn::Result<Vec<_>>>()?;

    let field_names = fields.iter().map(|field| &field.ident);

    let validation = generate_validation(options, rejection, golden_boot);

    Ok(quote! {
        #[automatically_derived]
        impl #marker_impl_generics
            #golden_boot::RequestEntity
            for #name #type_generics
            #marker_where_clause
        {
        }

        #[automatically_derived]
        impl #impl_generics
            #golden_boot::__private::axum::extract::FromRequestParts<
                __GoldenBootState
            >
            for #name #type_generics
            #where_clause
        {
            type Rejection = #rejection;

            async fn from_request_parts(
                __golden_parts:
                    &mut #golden_boot::__private::axum::http::request::Parts,
                __golden_state:
                    &__GoldenBootState,
            ) -> ::std::result::Result<
                Self,
                Self::Rejection,
            > {
                #path_setup
                #query_setup

                #(#extractions)*

                let __golden_entity = Self {
                    #(#field_names),*
                };

                #validation

                Ok(__golden_entity)
            }
        }
    })
}

fn expand_request(
    input: &DeriveInput,
    fields: &[RequestField],
    options: &RequestOptions,
    golden_boot: &TokenStream2,
) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let rejection = &options.rejection;

    let (_, type_generics, _) = input.generics.split_for_impl();

    let marker_generics = marker_generics(input);

    let (marker_impl_generics, _, marker_where_clause) = marker_generics.split_for_impl();

    let extractor_generics = extractor_generics(input, fields, options, golden_boot);

    let (impl_generics, _, where_clause) = extractor_generics.split_for_impl();

    let has_path = fields
        .iter()
        .any(|field| matches!(field.source, FieldSource::PathVariable));

    let has_query = fields
        .iter()
        .any(|field| matches!(field.source, FieldSource::RequestParam { .. }));

    let path_setup = generate_path_setup(
        has_path,
        quote!(&mut __golden_parts),
        rejection,
        golden_boot,
    );

    let query_setup = generate_query_setup(
        has_query,
        quote!(&mut __golden_parts),
        rejection,
        golden_boot,
    );

    let non_body_extractions = fields
        .iter()
        .filter(|field| !matches!(field.source, FieldSource::RequestBody))
        .map(|field| generate_field_extraction(field, rejection, golden_boot))
        .collect::<syn::Result<Vec<_>>>()?;

    let body_field = fields
        .iter()
        .find(|field| matches!(field.source, FieldSource::RequestBody))
        .expect("body field was already checked");

    let body_extraction = generate_body_extraction(body_field, rejection, golden_boot);

    let field_names = fields.iter().map(|field| &field.ident);

    let validation = generate_validation(options, rejection, golden_boot);

    Ok(quote! {
        #[automatically_derived]
        impl #marker_impl_generics
            #golden_boot::RequestEntity
            for #name #type_generics
            #marker_where_clause
        {
        }

        #[automatically_derived]
        impl #impl_generics
            #golden_boot::__private::axum::extract::FromRequest<
                __GoldenBootState
            >
            for #name #type_generics
            #where_clause
        {
            type Rejection = #rejection;

            async fn from_request(
                __golden_request:
                    #golden_boot::__private::axum::extract::Request,
                __golden_state:
                    &__GoldenBootState,
            ) -> ::std::result::Result<
                Self,
                Self::Rejection,
            > {
                let (
                    mut __golden_parts,
                    __golden_raw_body,
                ) = __golden_request.into_parts();

                #path_setup
                #query_setup

                #(#non_body_extractions)*

                let __golden_request =
                    #golden_boot::__private::axum::http::Request::from_parts(
                        __golden_parts,
                        __golden_raw_body,
                    );

                #body_extraction

                let __golden_entity = Self {
                    #(#field_names),*
                };

                #validation

                Ok(__golden_entity)
            }
        }
    })
}

fn generate_field_extraction(
    field: &RequestField,
    rejection: &Type,
    golden_boot: &TokenStream2,
) -> syn::Result<TokenStream2> {
    match &field.source {
        FieldSource::PathVariable => Ok(generate_path_extraction(field, rejection, golden_boot)),

        FieldSource::RequestParam { default } => Ok(generate_query_extraction(
            field,
            default.as_ref(),
            rejection,
            golden_boot,
        )),

        FieldSource::RequestHeader { name } => Ok(generate_header_extraction(
            field,
            name,
            rejection,
            golden_boot,
        )),

        FieldSource::RequestBody => Err(syn::Error::new_spanned(
            &field.ident,
            "request body must be extracted separately",
        )),
    }
}

fn generate_path_setup(
    enabled: bool,
    parts: TokenStream2,
    rejection: &Type,
    golden_boot: &TokenStream2,
) -> TokenStream2 {
    if !enabled {
        return TokenStream2::new();
    }

    quote! {
        let __golden_path =
            <
                #golden_boot::__private::axum::extract::Path<
                    ::std::collections::HashMap<
                        ::std::string::String,
                        ::std::string::String,
                    >
                >
                as
                #golden_boot::__private::axum::extract::FromRequestParts<
                    __GoldenBootState
                >
            >::from_request_parts(
                #parts,
                __golden_state,
            )
            .await
            .map(
                |#golden_boot::__private::axum::extract::Path(values)| {
                    values
                }
            )
            .map_err(|error| {
                <#rejection as ::std::convert::From<
                    #golden_boot::RequestEntityError
                >>::from(
                    #golden_boot::RequestEntityError::InvalidPath {
                        name: "<path>",
                        message: error.to_string(),
                    }
                )
            })?;
    }
}

fn generate_query_setup(
    enabled: bool,
    parts: TokenStream2,
    rejection: &Type,
    golden_boot: &TokenStream2,
) -> TokenStream2 {
    if !enabled {
        return TokenStream2::new();
    }

    quote! {
        let __golden_query =
            <
                #golden_boot::__private::axum::extract::Query<
                    ::std::collections::HashMap<
                        ::std::string::String,
                        ::std::string::String,
                    >
                >
                as
                #golden_boot::__private::axum::extract::FromRequestParts<
                    __GoldenBootState
                >
            >::from_request_parts(
                #parts,
                __golden_state,
            )
            .await
            .map(
                |#golden_boot::__private::axum::extract::Query(values)| {
                    values
                }
            )
            .map_err(|error| {
                <#rejection as ::std::convert::From<
                    #golden_boot::RequestEntityError
                >>::from(
                    #golden_boot::RequestEntityError::InvalidQuery {
                        name: "<query>",
                        message: error.to_string(),
                    }
                )
            })?;
    }
}

fn generate_path_extraction(
    field: &RequestField,
    rejection: &Type,
    golden_boot: &TokenStream2,
) -> TokenStream2 {
    let ident = &field.ident;
    let ty = &field.inner_ty;

    let parameter_name = LitStr::new(&ident.to_string(), ident.span());

    quote! {
        let #ident = {
            let __golden_value =
                __golden_path
                    .get(#parameter_name)
                    .ok_or_else(|| {
                        <#rejection as ::std::convert::From<
                            #golden_boot::RequestEntityError
                        >>::from(
                            #golden_boot::RequestEntityError::MissingPath {
                                name: #parameter_name,
                            }
                        )
                    })?;

            __golden_value
                .parse::<#ty>()
                .map_err(|error| {
                    <#rejection as ::std::convert::From<
                        #golden_boot::RequestEntityError
                    >>::from(
                        #golden_boot::RequestEntityError::InvalidPath {
                            name: #parameter_name,
                            message: error.to_string(),
                        }
                    )
                })?
        };
    }
}

fn generate_query_extraction(
    field: &RequestField,
    default: Option<&DefaultValue>,
    rejection: &Type,
    golden_boot: &TokenStream2,
) -> TokenStream2 {
    let ident = &field.ident;
    let ty = &field.inner_ty;

    let parameter_name = LitStr::new(&ident.to_string(), ident.span());

    let parse_value = quote! {
        __golden_value
            .parse::<#ty>()
            .map_err(|error| {
                <#rejection as ::std::convert::From<
                    #golden_boot::RequestEntityError
                >>::from(
                    #golden_boot::RequestEntityError::InvalidQuery {
                        name: #parameter_name,
                        message: error.to_string(),
                    }
                )
            })?
    };

    if field.optional {
        return quote! {
            let #ident =
                match __golden_query.get(#parameter_name) {
                    Some(__golden_value) => {
                        Some(#parse_value)
                    }

                    None => None,
                };
        };
    }

    match default {
        None => quote! {
            let #ident = {
                let __golden_value =
                    __golden_query
                        .get(#parameter_name)
                        .ok_or_else(|| {
                            <#rejection as ::std::convert::From<
                                #golden_boot::RequestEntityError
                            >>::from(
                                #golden_boot::RequestEntityError::MissingQuery {
                                    name: #parameter_name,
                                }
                            )
                        })?;

                #parse_value
            };
        },

        Some(DefaultValue::DefaultTrait) => quote! {
            let #ident =
                match __golden_query.get(#parameter_name) {
                    Some(__golden_value) => {
                        #parse_value
                    }

                    None => {
                        <#ty as ::std::default::Default>::default()
                    }
                };
        },

        Some(DefaultValue::Expression(expression)) => {
            quote! {
                let #ident =
                    match __golden_query.get(#parameter_name) {
                        Some(__golden_value) => {
                            #parse_value
                        }

                        None => {
                            #expression
                        }
                    };
            }
        }
    }
}

fn generate_header_extraction(
    field: &RequestField,
    header_name: &syn::Expr,
    rejection: &Type,
    golden_boot: &TokenStream2,
) -> TokenStream2 {
    let ident = &field.ident;
    let ty = &field.inner_ty;

    let field_name = LitStr::new(&ident.to_string(), ident.span());

    let parse_value = quote! {
        {
            let __golden_text =
                __golden_value
                    .to_str()
                    .map_err(|error| {
                        <#rejection as ::std::convert::From<
                            #golden_boot::RequestEntityError
                        >>::from(
                            #golden_boot::RequestEntityError::InvalidHeader {
                                name: #field_name,
                                message: error.to_string(),
                            }
                        )
                    })?;

            __golden_text
                .parse::<#ty>()
                .map_err(|error| {
                    <#rejection as ::std::convert::From<
                        #golden_boot::RequestEntityError
                    >>::from(
                        #golden_boot::RequestEntityError::InvalidHeader {
                            name: #field_name,
                            message: error.to_string(),
                        }
                    )
                })?
        }
    };

    if field.optional {
        quote! {
            let #ident =
                match __golden_parts
                    .headers
                    .get(#header_name)
                {
                    Some(__golden_value) => {
                        Some(#parse_value)
                    }

                    None => None,
                };
        }
    } else {
        quote! {
            let #ident = {
                let __golden_value =
                    __golden_parts
                        .headers
                        .get(#header_name)
                        .ok_or_else(|| {
                            <#rejection as ::std::convert::From<
                                #golden_boot::RequestEntityError
                            >>::from(
                                #golden_boot::RequestEntityError::MissingHeader {
                                    name: #field_name,
                                }
                            )
                        })?;

                #parse_value
            };
        }
    }
}

fn generate_body_extraction(
    field: &RequestField,
    rejection: &Type,
    golden_boot: &TokenStream2,
) -> TokenStream2 {
    let ident = &field.ident;
    let ty = &field.ty;

    quote! {
        let #golden_boot::__private::axum::Json(
            #ident
        ) =
            <
                #golden_boot::__private::axum::Json<#ty>
                as
                #golden_boot::__private::axum::extract::FromRequest<
                    __GoldenBootState
                >
            >::from_request(
                __golden_request,
                __golden_state,
            )
            .await
            .map_err(|error| {
                <#rejection as ::std::convert::From<
                    #golden_boot::RequestEntityError
                >>::from(
                    #golden_boot::RequestEntityError::InvalidBody {
                        message: error.to_string(),
                    }
                )
            })?;
    }
}

fn generate_validation(
    options: &RequestOptions,
    rejection: &Type,
    golden_boot: &TokenStream2,
) -> TokenStream2 {
    if !options.validate {
        return TokenStream2::new();
    }

    quote! {
        #golden_boot::__private::validator::Validate::validate(
            &__golden_entity
        )
        .map_err(|error| {
            <#rejection as ::std::convert::From<
                #golden_boot::RequestEntityError
            >>::from(
                #golden_boot::RequestEntityError::Validation {
                    message: error.to_string(),
                }
            )
        })?;
    }
}

fn marker_generics(input: &DeriveInput) -> Generics {
    let mut generics = input.generics.clone();

    let name = &input.ident;
    let (_, type_generics, _) = input.generics.split_for_impl();

    generics.make_where_clause().predicates.push(parse_quote! {
        #name #type_generics:
            ::std::marker::Send
            + 'static
    });

    generics
}

fn extractor_generics(
    input: &DeriveInput,
    fields: &[RequestField],
    options: &RequestOptions,
    golden_boot: &TokenStream2,
) -> Generics {
    let mut generics = input.generics.clone();

    generics.params.push(parse_quote!(__GoldenBootState));

    let name = &input.ident;
    let rejection = &options.rejection;

    let (_, type_generics, _) = input.generics.split_for_impl();

    let where_clause = generics.make_where_clause();

    where_clause.predicates.push(parse_quote! {
        __GoldenBootState:
            ::std::marker::Send
            + ::std::marker::Sync
    });

    where_clause.predicates.push(parse_quote! {
        #name #type_generics:
            ::std::marker::Send
            + 'static
    });

    where_clause.predicates.push(parse_quote! {
        #rejection:
            ::std::convert::From<
                #golden_boot::RequestEntityError
            >
            + #golden_boot::IntoResponse
    });

    for field in fields {
        match &field.source {
            FieldSource::PathVariable
            | FieldSource::RequestParam { .. }
            | FieldSource::RequestHeader { .. } => {
                let ty = &field.inner_ty;

                where_clause.predicates.push(parse_quote! {
                    #ty:
                        ::std::str::FromStr
                        + ::std::marker::Send
                        + 'static
                });

                where_clause.predicates.push(parse_quote! {
                    <#ty as ::std::str::FromStr>::Err:
                        ::std::fmt::Display
                });
            }

            FieldSource::RequestBody => {
                let ty = &field.ty;

                where_clause.predicates.push(parse_quote! {
                    #ty:
                        #golden_boot::__private::serde::de::DeserializeOwned
                        + ::std::marker::Send
                        + 'static
                });
            }
        }

        if matches!(
            field.source,
            FieldSource::RequestParam {
                default: Some(DefaultValue::DefaultTrait)
            }
        ) {
            let ty = &field.inner_ty;

            where_clause.predicates.push(parse_quote! {
                #ty: ::std::default::Default
            });
        }
    }

    if options.validate {
        where_clause.predicates.push(parse_quote! {
            #name #type_generics:
                #golden_boot::__private::validator::Validate
        });
    }

    generics
}
