//! `define_command!` implementation.

use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, LitStr, Token};

pub fn expand(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let def = syn::parse_macro_input!(input as CommandDef);
    let name = &def.name;
    let name_str = name.to_string();
    let desc = &def.description;
    let module_name = Ident::new(&format!("__runie_cmd_{}", name), name.span());
    let handler_name = Ident::new(&format!("__runie_handler_{}", name), name.span());

    let expanded = quote! {
        mod #module_name {
            pub fn #handler_name(
                _state: &mut ::runie_core::model::AppState,
                _args: &str,
            ) -> ::runie_core::commands::CommandResult {
                ::runie_core::commands::CommandResult::None
            }
        }

        pub fn #name() -> ::runie_core::commands::CommandDef {
            ::runie_core::commands::CommandDef::new(#name_str)
                .desc(#desc)
                .handler(#module_name::#handler_name)
        }
    };
    expanded.into()
}

struct CommandDef {
    name: Ident,
    description: LitStr,
}

impl Parse for CommandDef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name = input.parse()?;
        input.parse::<Token![,]>()?;
        let description = input.parse()?;
        Ok(Self { name, description })
    }
}
