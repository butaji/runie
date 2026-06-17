//! `define_hook!` implementation.

use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, LitStr, Token};

pub fn expand(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let def = syn::parse_macro_input!(input as HookDef);
    let name = &def.name;
    let _ = &def.event_name;

    let expanded = quote! {
        #[derive(Debug, Clone, Copy, Default)]
        pub struct #name;

        impl ::runie_core::hooks::HookHandler for #name {
            fn handle(
                &self,
                _payload: &::serde_json::Value,
            ) -> ::runie_core::hooks::HookDecision {
                ::runie_core::hooks::HookDecision::Allow
            }
        }
    };
    expanded.into()
}

struct HookDef {
    name: Ident,
    event_name: LitStr,
}

impl Parse for HookDef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name = input.parse()?;
        input.parse::<Token![,]>()?;
        let event_name = input.parse()?;
        Ok(Self { name, event_name })
    }
}
