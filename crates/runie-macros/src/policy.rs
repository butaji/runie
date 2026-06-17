//! `define_policy!` implementation.

use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, LitStr, Token};

pub fn expand(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let def = syn::parse_macro_input!(input as PolicyDef);
    let name = &def.name;
    let tool = &def.tool;
    let action = &def.action;
    let fn_name = Ident::new(&to_snake_case(&name.to_string()), name.span());

    let expanded = quote! {
        #[allow(non_snake_case)]
        pub fn #fn_name() -> ::runie_core::permissions::PermissionRule {
            ::runie_core::permissions::PermissionRule {
                tool_pattern: #tool.to_string(),
                path_pattern: None,
                action: ::runie_core::permissions::PermissionAction::#action,
            }
        }
    };
    expanded.into()
}

struct PolicyDef {
    name: Ident,
    tool: LitStr,
    action: Ident,
}

impl Parse for PolicyDef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name = input.parse()?;
        input.parse::<Token![,]>()?;
        input.parse::<Ident>()?;
        input.parse::<Token![:]>()?;
        let tool = input.parse()?;
        input.parse::<Token![,]>()?;
        input.parse::<Ident>()?;
        input.parse::<Token![:]>()?;
        let action = input.parse()?;
        Ok(Self { name, tool, action })
    }
}

fn to_snake_case(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            out.push('_');
        }
        out.extend(c.to_lowercase());
    }
    out
}
