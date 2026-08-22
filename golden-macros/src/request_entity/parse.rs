use syn::{
    Attribute, Data, DeriveInput, Error, Expr, Field, Fields, GenericArgument, LitStr,
    PathArguments, Type,
};

use super::model::{DefaultValue, FieldSource, RequestField, RequestOptions};

pub fn option_inner_type(ty: &Type) -> Option<Type> {
    let Type::Path(type_path) = ty else {
        return None;
    };

    let segment = type_path.path.segments.last()?;

    if segment.ident != "Option" {
        return None;
    }

    let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return None;
    };

    let GenericArgument::Type(inner) = arguments.args.first()? else {
        return None;
    };

    Some(inner.clone())
}

pub fn parse_options(input: &DeriveInput) -> syn::Result<RequestOptions> {
    let mut rejection = None;
    let mut validate = false;

    for attribute in &input.attrs {
        if !attribute.path().is_ident("request_entity") {
            continue;
        }

        attribute.parse_nested_meta(|meta| {
            if meta.path.is_ident("rejection") {
                rejection = Some(meta.value()?.parse::<Type>()?);

                return Ok(());
            }

            if meta.path.is_ident("validate") {
                validate = true;
                return Ok(());
            }

            Err(meta.error("unsupported request_entity option"))
        })?;
    }

    let rejection = rejection.ok_or_else(|| {
        Error::new_spanned(
            &input.ident,
            "missing `#[request_entity(rejection = ErrorType)]`",
        )
    })?;

    Ok(RequestOptions {
        rejection,
        validate,
    })
}

pub fn named_fields(
    input: &DeriveInput,
) -> syn::Result<&syn::punctuated::Punctuated<Field, syn::token::Comma>> {
    let Data::Struct(data) = &input.data else {
        return Err(Error::new_spanned(
            input,
            "RequestEntity can only be derived for structs",
        ));
    };

    let Fields::Named(fields) = &data.fields else {
        return Err(Error::new_spanned(
            input,
            "RequestEntity requires named fields",
        ));
    };

    Ok(&fields.named)
}

fn parse_name(meta: &syn::meta::ParseNestedMeta<'_>) -> syn::Result<LitStr> {
    let name = meta.value()?.parse::<LitStr>()?;

    if name.value().is_empty() {
        return Err(Error::new_spanned(name, "parameter name cannot be empty"));
    }

    Ok(name)
}

fn parse_path_variable(attribute: &Attribute) -> syn::Result<Option<LitStr>> {
    if matches!(attribute.meta, syn::Meta::Path(_)) {
        return Ok(None);
    }

    let mut name = None;

    attribute.parse_nested_meta(|meta| {
        if !meta.path.is_ident("name") {
            return Err(meta.error("unsupported path_variable option; expected `name`"));
        }

        if name.is_some() {
            return Err(meta.error("duplicate `name` option"));
        }

        name = Some(parse_name(&meta)?);
        Ok(())
    })?;

    Ok(name)
}

fn parse_request_param(
    attribute: &Attribute,
) -> syn::Result<(Option<LitStr>, Option<DefaultValue>)> {
    if matches!(attribute.meta, syn::Meta::Path(_)) {
        return Ok((None, None));
    }

    let mut name = None;
    let mut default = None;

    attribute.parse_nested_meta(|meta| {
        if meta.path.is_ident("name") {
            if name.is_some() {
                return Err(meta.error("duplicate `name` option"));
            }

            name = Some(parse_name(&meta)?);
            return Ok(());
        }

        if !meta.path.is_ident("default") {
            return Err(
                meta.error("unsupported request_param option; expected `name` or `default`")
            );
        }

        if default.is_some() {
            return Err(meta.error("duplicate `default` option"));
        }

        if meta.input.is_empty() {
            default = Some(DefaultValue::DefaultTrait);

            return Ok(());
        }

        let expression = meta.value()?.parse::<Expr>()?;

        default = Some(DefaultValue::Expression(expression));

        Ok(())
    })?;

    Ok((name, default))
}

fn parse_request_header(attribute: &Attribute) -> syn::Result<Expr> {
    let mut name = None;

    attribute.parse_nested_meta(|meta| {
        if !meta.path.is_ident("name") {
            return Err(meta.error("unsupported request_header option"));
        }

        name = Some(meta.value()?.parse::<Expr>()?);

        Ok(())
    })?;

    name.ok_or_else(|| {
        Error::new_spanned(
            attribute,
            "`#[request_header]` requires `name = HeaderName`",
        )
    })
}

pub fn parse_field(field: &Field) -> syn::Result<RequestField> {
    let ident = field
        .ident
        .clone()
        .ok_or_else(|| Error::new_spanned(field, "RequestEntity requires named fields"))?;

    let mut source = None;

    for attribute in &field.attrs {
        let current = if attribute.path().is_ident("path_variable") {
            Some(FieldSource::PathVariable {
                name: parse_path_variable(attribute)?,
            })
        } else if attribute.path().is_ident("request_param") {
            let (name, default) = parse_request_param(attribute)?;

            Some(FieldSource::RequestParam { name, default })
        } else if attribute.path().is_ident("request_header") {
            Some(FieldSource::RequestHeader {
                name: parse_request_header(attribute)?,
            })
        } else if attribute.path().is_ident("request_body") {
            Some(FieldSource::RequestBody)
        } else {
            None
        };

        if let Some(current) = current {
            if source.is_some() {
                return Err(Error::new_spanned(
                    attribute,
                    "a field can have only one request source",
                ));
            }

            source = Some(current);
        }
    }

    let source = source.ok_or_else(|| {
        Error::new_spanned(
            field,
            "every RequestEntity field must declare its request source",
        )
    })?;

    let optional_inner = option_inner_type(&field.ty);

    let optional = optional_inner.is_some();

    let inner_ty = optional_inner.unwrap_or_else(|| field.ty.clone());

    if matches!(source, FieldSource::PathVariable { .. }) && optional {
        return Err(Error::new_spanned(
            &field.ty,
            "`#[path_variable]` cannot use `Option<T>`",
        ));
    }

    if matches!(source, FieldSource::RequestBody) && optional {
        return Err(Error::new_spanned(
            &field.ty,
            "`#[request_body]` cannot currently use `Option<T>`",
        ));
    }

    if let FieldSource::RequestParam {
        default: Some(_), ..
    } = &source
        && optional
    {
        return Err(Error::new_spanned(
            field,
            "`default` cannot be combined with `Option<T>`",
        ));
    }

    Ok(RequestField {
        ident,
        ty: field.ty.clone(),
        inner_ty,
        optional,
        source,
    })
}
