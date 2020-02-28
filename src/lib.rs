/*!
Derive macro for automatically adding an implementation of
pandora_api::PandoraApiRequest to a struct.

The name of the Pandora API method that will be called defaults to the
result of converting the struct name to mixed case (GetFoo -> getFoo).
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
// SPDX-License-Identifier: MIT

#![deny(missing_docs)]
extern crate proc_macro;

use darling::FromDeriveInput;
use heck::MixedCase;
use proc_macro::TokenStream;
use proc_macro2;
use quote::{format_ident, quote, ToTokens};
use syn::{Generics, Ident};

/// Derive macro for adding implementation of pandora_api::PandoraApiRequest
/// trait to a struct.
#[derive(FromDeriveInput)]
#[darling(attributes(pandora_request))]
struct PandoraRequest {
    ident: Ident,
    generics: Generics,
    // Default is <StructName>Response
    #[darling(default = "std::option::Option::default")]
    response_type: Option<String>,
    // Default is "Error"
    #[darling(default = "std::option::Option::default")]
    error_type: Option<String>,
    // Default is the output of format!("{}.{}", "<ModuleName>", "<StructName>".to_mixed_case())
    #[darling(default = "std::option::Option::default")]
    method_name: Option<String>,
    #[darling(default = "std::option::Option::default")]
    encrypted: Option<bool>,
}

impl ToTokens for PandoraRequest {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let PandoraRequest {
            ref ident,
            ref generics,
            ref response_type,
            ref error_type,
            ref method_name,
            ref encrypted,
        } = *self;

        // if no response_type was specified, we default
        // to the Self type + "Response".
        let final_response_type = format_ident!(
            "{}",
            response_type
                .as_ref()
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("{}Response", ident))
        );
        let final_error_type = format_ident!(
            "{}",
            error_type
                .as_ref()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "Error".to_string())
        );

        let get_method_decl = if let Some(method_name) = method_name {
            quote! {
                fn get_method(&self) -> String {
                    stringify!(#method_name).to_string()
                }
            }
        } else {
            let mixed_case_method = ident.to_string().to_mixed_case();
            quote! {
                fn get_method(&self) -> String {
                    let module_name = std::module_path!();
                    let class_name = module_name.rsplitn(2, "::").next().expect("Could not infer a valid method name since there is no current module. Must pass #[pandora_request(method_name = \"<value>\")] as part of the derive.");
                    format!("{}.{}", class_name, #mixed_case_method)
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
            .unwrap_or_else(|| quote! {});

        // If the type is generic, we need to pass that
        // through to the impl
        let (imp, ty, wher) = generics.split_for_impl();
        tokens.extend(quote! {
            impl #imp PandoraApiRequest for #ident #ty #wher {
                type Response = #final_response_type;
                type Error = #final_error_type;
                #encrypt_expr
                #get_method_decl
            }
        });
    }
}

/// Derive macro for adding implementation of pandora_api::PandoraApiRequest
/// trait to a struct.
#[proc_macro_derive(PandoraRequest, attributes(pandora_request))]
pub fn derive_pandora_request(input: TokenStream) -> TokenStream {
    let request = PandoraRequest::from_derive_input(&syn::parse(input).unwrap())
        .expect("Failed parsing macro input");
    let pm2_tokens: proc_macro2::TokenStream = quote! {#request};
    //panic!("Generated tokens: {}", pm2_tokens.to_string());
    pm2_tokens.into()
}
