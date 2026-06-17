//! `define_event!` implementation.

use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{punctuated::Punctuated, Ident, Token, Type};

pub fn expand(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let def = syn::parse_macro_input!(input as EventDef);
    let vis = &def.vis;
    let name = &def.name;
    let fields = def.fields.iter().map(|f| {
        let field_name = &f.name;
        let field_ty = &f.ty;
        quote! { #field_name: #field_ty }
    });
    let field_names = def.fields.iter().map(|f| &f.name);

    let expanded = quote! {
        #[derive(Debug, Clone, PartialEq, ::serde::Serialize, ::serde::Deserialize)]
        #vis struct #name {
            #(#fields),*
        }

        impl #name {
            /// Returns the event type name.
            pub fn event_name() -> &'static str {
                stringify!(#name)
            }
        }

        impl ::std::convert::From<#name> for ::serde_json::Value {
            fn from(event: #name) -> Self {
                match ::serde_json::to_value(&event) {
                    Ok(value) => value,
                    Err(_) => ::serde_json::json!({
                        "event": stringify!(#name),
                        "fields": [#(stringify!(#field_names)),*]
                    }),
                }
            }
        }
    };
    expanded.into()
}

struct EventDef {
    vis: syn::Visibility,
    name: Ident,
    fields: Punctuated<Field, Token![,]>,
}

impl Parse for EventDef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let vis = input.parse()?;
        let name = input.parse()?;
        let content;
        syn::braced!(content in input);
        let fields = content.parse_terminated(Field::parse, Token![,])?;
        Ok(Self { vis, name, fields })
    }
}

struct Field {
    name: Ident,
    ty: Type,
}

impl Parse for Field {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name = input.parse()?;
        input.parse::<Token![:]>()?;
        let ty = input.parse()?;
        Ok(Self { name, ty })
    }
}
