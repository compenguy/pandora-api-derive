// SPDX-License-Identifier: MIT
/*!
Derive macros for automatically adding an implementation of
pandora_api::Pandora<Json|Rest>ApiRequest to a struct.

The name of the Pandora API method that will be called defaults to the
result of converting the struct name to lower camel case (GetFoo -> getFoo).
This may be overridden using the #[pandora_request(method_name = getFOOBar)
struct attribute.

The default error type is Error.  If a different type name is required,
this may be overridden using the #[pandora_request(error_type = FooError)]
struct attribute.

The default return type of the request is <struct name>Response. This may be
overridden using the #[pandora_request(response_type = FooResponse)] struct
attribute.

The default for a request is to send it unencrypted.  If the request must be
encrypted, this may be overridden using the #[pandora_request(encrypted = true)]
struct attribute.

*/

#![deny(missing_docs)]
#![allow(clippy::manual_unwrap_or_default)] // darling's #[darling(default)] triggers false positive
extern crate proc_macro;

use darling::FromDeriveInput;
use heck::ToLowerCamelCase;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::{Generics, Ident, LitStr};

/// Shared attributes parsed from `#[pandora_request(...)]` for both Json and Rest derives.
#[derive(FromDeriveInput)]
#[darling(attributes(pandora_request))]
struct PandoraRequest {
    ident: Ident,
    generics: Generics,
    /// Default is <StructName>Response
    #[darling(default)]
    response_type: Option<String>,
    /// Default is "Error"
    #[darling(default)]
    error_type: Option<String>,
    /// Default is derived from module path and struct name in lower camel case.
    #[darling(default)]
    method_name: Option<String>,
    #[darling(default)]
    encrypted: Option<bool>,
}

/// Distinguishes the two API styles for code generation.
enum RequestKind {
    /// JSON API: trait PandoraJsonApiRequest, method format "{}.{}"
    Json,
    /// REST API: trait PandoraRestApiRequest, method format "/api/v1/{}/{}"
    Rest,
}

impl RequestKind {
    fn trait_ident(&self) -> Ident {
        match self {
            RequestKind::Json => format_ident!("PandoraJsonApiRequest"),
            RequestKind::Rest => format_ident!("PandoraRestApiRequest"),
        }
    }

    fn method_format(&self) -> &'static str {
        match self {
            RequestKind::Json => "{}.{}",
            RequestKind::Rest => "/api/v1/{}/{}",
        }
    }
}

/// Generates the `impl Trait for Struct` block for the given request and kind.
fn emit_impl(request: &PandoraRequest, kind: RequestKind) -> proc_macro2::TokenStream {
    let PandoraRequest {
        ref ident,
        ref generics,
        ref response_type,
        ref error_type,
        ref method_name,
        ref encrypted,
    } = *request;

    let final_response_type = format_ident!(
        "{}",
        response_type
            .as_ref()
            .map(String::to_string)
            .unwrap_or_else(|| format!("{ident}Response"))
    );
    let final_error_type = format_ident!(
        "{}",
        error_type
            .as_ref()
            .map(String::to_string)
            .unwrap_or_else(|| "Error".to_string())
    );

    let get_method_decl = if let Some(ref method_name) = method_name {
        let method_name_lit = LitStr::new(method_name, Span::call_site());
        quote! {
            fn get_method(&self) -> String {
                #method_name_lit.to_string()
            }
        }
    } else {
        let lower_camel_case_method = ident.to_string().to_lower_camel_case();
        let method_format_lit = LitStr::new(kind.method_format(), Span::call_site());
        quote! {
            fn get_method(&self) -> String {
                let module_name = std::module_path!();
                let class_name = module_name.rsplitn(2, "::").next().expect("Could not infer a valid method name since there is no current module. Must pass #[pandora_request(method_name = \"<value>\")] as part of the derive.");
                format!(#method_format_lit, class_name, #lower_camel_case_method)
            }
        }
    };

    let encrypt_expr = encrypted
        .map(|b| {
            quote! {
                fn encrypt_request(&self) -> bool {
                    #b
                }
            }
        })
        .unwrap_or_else(|| {
            quote! {
                fn encrypt_request(&self) -> bool {
                    false
                }
            }
        });

    let trait_ident = kind.trait_ident();
    let (imp, ty, wher) = generics.split_for_impl();

    quote! {
        impl #imp #trait_ident for #ident #ty #wher {
            type Response = #final_response_type;
            type Error = #final_error_type;
            #encrypt_expr
            #get_method_decl
        }
    }
}

/// Parses macro input and returns a compile error TokenStream on failure.
fn parse_input(input: TokenStream) -> Result<syn::DeriveInput, proc_macro2::TokenStream> {
    syn::parse(input).map_err(|e| e.to_compile_error())
}

/// Parses derive input into `PandoraRequest` and returns a compile error TokenStream on failure.
fn parse_request(ast: &syn::DeriveInput) -> Result<PandoraRequest, proc_macro2::TokenStream> {
    PandoraRequest::from_derive_input(ast).map_err(|e| e.write_errors())
}

/// Derive macro for adding implementation of pandora_api::PandoraJsonApiRequest
/// trait to a struct.
#[proc_macro_derive(PandoraJsonRequest, attributes(pandora_request))]
pub fn derive_pandora_json_request(input: TokenStream) -> TokenStream {
    let ast = match parse_input(input) {
        Ok(a) => a,
        Err(e) => return e.into(),
    };
    let request = match parse_request(&ast) {
        Ok(r) => r,
        Err(e) => return e.into(),
    };
    emit_impl(&request, RequestKind::Json).into()
}

/// Derive macro for adding implementation of pandora_api::PandoraRestApiRequest
/// trait to a struct.
#[proc_macro_derive(PandoraRestRequest, attributes(pandora_request))]
pub fn derive_pandora_rest_request(input: TokenStream) -> TokenStream {
    let ast = match parse_input(input) {
        Ok(a) => a,
        Err(e) => return e.into(),
    };
    let request = match parse_request(&ast) {
        Ok(r) => r,
        Err(e) => return e.into(),
    };
    emit_impl(&request, RequestKind::Rest).into()
}
