use crate::crate_path;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    Error, Expr, ExprLit, ItemFn, Lit, MetaNameValue, PathArguments, ReturnType, Token, Type,
    parse::Parser, punctuated::Punctuated,
};

const DEFAULT_PORT: u16 = 8080;

pub fn expand(arguments: TokenStream, item: TokenStream) -> syn::Result<TokenStream2> {
    let function = syn::parse::<ItemFn>(item)?;

    validate_application_function(&function)?;

    let port = parse_port(arguments)?;
    let attributes = &function.attrs;
    let visibility = &function.vis;
    let function_name = &function.sig.ident;
    let block = &function.block;
    let runner = if returns_result(&function) {
        quote!(run_fallible)
    } else {
        quote!(run)
    };
    let initializer = match &function.sig.output {
        ReturnType::Default => quote!(async move #block),
        ReturnType::Type(_, output) => quote! {
            async move {
                let __golden_application_output: #output =
                    (async move #block).await;

                __golden_application_output
            }
        },
    };

    let golden_boot = crate_path::golden_boot()?;
    Ok(quote! {
        #(#attributes)*
        #visibility fn #function_name()
            -> ::std::result::Result<
                (),
                #golden_boot::ApplicationError
            >
        {
            #golden_boot::__private::#runner(
                #port,
                #initializer,
            )
        }
    })
}

fn returns_result(function: &ItemFn) -> bool {
    let ReturnType::Type(_, output) = &function.sig.output else {
        return false;
    };

    let Type::Path(type_path) = output.as_ref() else {
        return false;
    };

    let Some(segment) = type_path.path.segments.last() else {
        return false;
    };

    segment.ident == "Result"
        && matches!(
            &segment.arguments,
            PathArguments::AngleBracketed(arguments) if !arguments.args.is_empty()
        )
}

fn validate_application_function(function: &ItemFn) -> syn::Result<()> {
    if function.sig.ident != "main" {
        return Err(Error::new_spanned(
            &function.sig.ident,
            "`#[golden_boot_application]` can only be applied to `main`",
        ));
    }

    if function.sig.asyncness.is_none() {
        return Err(Error::new_spanned(
            function.sig.fn_token,
            "GoldenBoot application entry point must be async",
        ));
    }

    if !function.sig.inputs.is_empty() {
        return Err(Error::new_spanned(
            &function.sig.inputs,
            "GoldenBoot application entry point cannot accept parameters",
        ));
    }

    if !function.sig.generics.params.is_empty() {
        return Err(Error::new_spanned(
            &function.sig.generics,
            "GoldenBoot application entry point cannot have generic parameters",
        ));
    }

    if function.sig.constness.is_some() {
        return Err(Error::new_spanned(
            function.sig.constness,
            "GoldenBoot application entry point cannot be const",
        ));
    }

    if function.sig.unsafety.is_some() {
        return Err(Error::new_spanned(
            function.sig.unsafety,
            "GoldenBoot application entry point cannot be unsafe",
        ));
    }

    Ok(())
}

fn parse_port(arguments: TokenStream) -> syn::Result<u16> {
    if arguments.is_empty() {
        return Ok(DEFAULT_PORT);
    }

    let parser = Punctuated::<MetaNameValue, Token![,]>::parse_terminated;

    let arguments = parser.parse(arguments)?;

    if arguments.len() != 1 {
        return Err(Error::new_spanned(
            arguments,
            "expected one argument: `port = 8080`",
        ));
    }

    let argument = &arguments[0];

    if !argument.path.is_ident("port") {
        return Err(Error::new_spanned(
            &argument.path,
            "unsupported argument; expected `port`",
        ));
    }

    parse_port_value(&argument.value)
}

fn parse_port_value(expression: &Expr) -> syn::Result<u16> {
    let Expr::Lit(ExprLit {
        lit: Lit::Int(integer),
        ..
    }) = expression
    else {
        return Err(Error::new_spanned(
            expression,
            "`port` must be an integer literal",
        ));
    };

    let port = integer.base10_parse::<u16>().map_err(|_| {
        Error::new_spanned(integer, "`port` must be an integer between 1 and 65535")
    })?;

    if port == 0 {
        return Err(Error::new_spanned(
            integer,
            "`port` must be between 1 and 65535",
        ));
    }

    Ok(port)
}
