use std::time::SystemTime;

use chrono::{DateTime, Utc};
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    Attribute, Data, DeriveInput, Error, Expr, Fields, Ident, Lit, Meta, parse_macro_input,
    spanned::Spanned,
};

use api_core::APIEndpointError;

///
/// ```rust
/// #[derive(APIEndpointError)]
/// #[api(endpoint(method = "POST", path = "example/path/to/endpoint"))]
/// ExampleError {
///     SomeError,
///     OtherError,
/// }
/// ```
///

#[proc_macro_derive(APIEndpointError, attributes(api))]
pub fn derive_api_error(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand(input) {
        Ok(ts) => ts,
        Err(e) => e.to_compile_error().into(),
    }
}

#[derive(Clone)]
struct EndpointAttr {
    method: String,
    path: String,
}
impl EndpointAttr {
    fn as_conv(self) -> proc_macro2::TokenStream {
        let method = self.method;
        let path = self.path;
        quote! {
            ::api_core::EndpointMeta {
                method: #method,
                path: #path,
            }
        }
    }
}

struct VariantCfg {
    pat: proc_macro2::TokenStream,
    code_expr: Expr,
    status: u16,
    msg: Option<String>,
}

fn expand(input: DeriveInput) -> syn::Result<TokenStream> {
    let enum_ident = input.ident.clone();

    // Parse enum-level attributes: #[api(endpoint(method="...", path="..."))]
    let enum_endpoint = parse_enum_endpoint(&input.attrs, &enum_ident)?;

    // Must be an enum
    let Data::Enum(data_enum) = &input.data else {
        return Err(syn::Error::new(
            input.span(),
            "#[derive(ApiError)] only supports enums",
        ));
    };

    // For each variant, parse attributes
    let mut variants_cfg = Vec::<VariantCfg>::new();
    for v in &data_enum.variants {
        let pat = pattern_from_variant(&enum_ident, v);

        // Find #[api(...)] on variant and extract code/status/msg/endpoint
        let mut code_expr: Option<Expr> = None;
        let mut status: Option<u16> = None;
        let mut msg: Option<String> = None;

        for attr in &v.attrs {
            if !attr.path().is_ident("api") {
                continue;
            }
            match attr.parse_args_with(|input: syn::parse::ParseStream| {
                let mut found_any = false;
                while !input.is_empty() {
                    let ident: syn::Ident = input.parse()?;
                    let _eq: syn::Token![=] = input.parse()?;
                    if ident == "code" {
                        let expr: Expr = input.parse()?;
                        code_expr = Some(expr);
                    } else if ident == "status" {
                        let lit: Lit = input.parse()?;
                        let Lit::Int(i) = lit else {
                            return Err(input.error("status must be an integer literal"));
                        };
                        status = Some(i.base10_parse()?);
                    } else if ident == "msg" {
                        let lit: Lit = input.parse()?;
                        let Lit::Str(s) = lit else {
                            return Err(input.error("msg must be a string literal"));
                        };
                        msg = Some(s.value());
                    } else if ident == "endpoint" {
                        // Already processed
                        // TODO: Make this implementation a little more intuative
                        return Err(input.error("Testing"));
                    } else {
                        return Err(input.error("unknown key in #[api(...)] on variant"));
                    }
                    found_any = true;
                    // Optional comma
                    let _ = input.parse::<syn::Token![,]>();
                }
                if !found_any {
                    return Err(input.error("empty #[api(...)] on variant"));
                }
                Ok(())
            }) {
                Ok(_) => {}
                Err(e) => return Err(e),
            }
        }

        let Some(code_expr) = code_expr else {
            return Err(syn::Error::new(
                v.span(),
                "variant missing #[api(code = APIErrorCode::... , ...)]",
            ));
        };

        let Some(status) = status else {
            return Err(syn::Error::new(
                v.span(),
                "variant missing #[api(status = ...)]",
            ));
        };

        variants_cfg.push(VariantCfg {
            pat,
            code_expr,
            status,
            msg,
        });
    }

    // Generate match arms
    let arms = variants_cfg.iter().map(|c| {
        let pat = &c.pat;
        let code = &c.code_expr;
        let st = c.status;
        let msg = if let Some(m) = c.msg.clone() {
            quote! { Some(#m) }
        } else {
            quote! { None }
        };
        let ep = enum_endpoint.clone().as_conv();
        quote! { #pat => ::api_core::APIError {
            code: #code,
            message: #msg,
            http_status: #st,
            timestamp: ::api_core::APIError::timestamp(),
            endpoint: #ep,
        },
        }
    });

    // Implementations
    let expanded = quote! {
        impl ::std::convert::Into<::api_core::APIError> for #enum_ident {
            fn into(self) -> ::api_core::APIError {
                match self { #(#arms)* }
            }
        }

        impl ::api_core::APIEndpointError for #enum_ident {}
    };

    Ok(expanded.into())
}

fn parse_enum_endpoint(attrs: &[Attribute], ident: &Ident) -> syn::Result<EndpointAttr> {
    for attr in attrs {
        if !attr.path().is_ident("api") {
            continue;
        }
        // We only accept container-level: endpoint(method="...", path="...")
        if let Meta::List(list) = &attr.meta {
            // Look for "endpoint(...)" form
            for nested in &list.parse_args_with(parse_nested_list).unwrap_or_default() {
                if let Some(ep) = nested.clone()? {
                    return Ok(ep);
                }
            }
        } else {
            // Also accept #[api(endpoint(method="..", path=".."))] with parse_args directly
            if let Ok(ep) = attr.parse_args_with(|input: syn::parse::ParseStream| {
                let ident: syn::Ident = input.parse()?;
                if ident != "endpoint" {
                    return Err(input.error("expected endpoint(method=\"...\", path=\"...\")"));
                }
                let content;
                syn::parenthesized!(content in input);
                let (m, p) = parse_endpoint_args(&content)?;
                Ok(EndpointAttr { method: m, path: p })
            }) {
                return Ok(ep);
            }
        }
    }
    Err(syn::Error::new(
        ident.span(),
        "No endpoint specified. Add #[api(endpoint(method=\"...\", path=\"...\"))] on enum or variant.",
    ))
}

// Helper to parse nested entries like endpoint(...)
fn parse_nested_list(
    input: syn::parse::ParseStream,
) -> syn::Result<Vec<syn::Result<Option<EndpointAttr>>>> {
    let mut out = Vec::new();
    while !input.is_empty() {
        let ident: syn::Ident = input.parse()?;
        if ident == "endpoint" {
            let content;
            syn::parenthesized!(content in input);
            let (m, p) = parse_endpoint_args(&content)?;
            out.push(Ok(Some(EndpointAttr { method: m, path: p })));
        } else {
            // Skip unknown, but note error
            out.push(Err(syn::Error::new(
                ident.span(),
                "unknown container key in #[api(...)]",
            )));
        }
        let _ = input.parse::<syn::Token![,]>();
    }
    Ok(out)
}

fn parse_endpoint_args(input: &syn::parse::ParseBuffer) -> syn::Result<(String, String)> {
    // method="...", path="..."
    let mut method: Option<String> = None;
    let mut path: Option<String> = None;
    while !input.is_empty() {
        let key: syn::Ident = input.parse()?;
        let _eq: syn::Token![=] = input.parse()?;
        let val = input.parse::<Lit>()?;
        let Lit::Str(s) = val else {
            return Err(syn::Error::new(
                val.span(),
                "endpoint values must be string literals",
            ));
        };
        if key == "method" {
            method = Some(s.value());
        } else if key == "path" {
            path = Some(s.value());
        } else {
            return Err(syn::Error::new(
                key.span(),
                "unknown key in endpoint(...), expected method or path",
            ));
        }
        let _ = input.parse::<syn::Token![,]>();
    }
    let method =
        method.ok_or_else(|| syn::Error::new(input.span(), "endpoint(...) missing method"))?;
    let path = path.ok_or_else(|| syn::Error::new(input.span(), "endpoint(...) missing path"))?;
    Ok((method, path))
}

fn pattern_from_variant(enum_ident: &syn::Ident, v: &syn::Variant) -> proc_macro2::TokenStream {
    let vid = &v.ident;
    match &v.fields {
        Fields::Unit => quote! { #enum_ident::#vid },
        Fields::Unnamed(_) => quote! { #enum_ident::#vid ( .. ) },
        Fields::Named(_) => quote! { #enum_ident::#vid { .. } },
    }
}
