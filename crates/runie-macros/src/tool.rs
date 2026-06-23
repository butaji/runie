//! `define_tool!` implementation.

use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{punctuated::Punctuated, Ident, LitStr, Token, Type};

pub fn expand(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let def = syn::parse_macro_input!(input as ToolDef);
    build_tool(&def).into()
}

fn build_tool(def: &ToolDef) -> proc_macro2::TokenStream {
    let name = &def.name;
    let tool_name = &def.tool_name;
    let description = &def.description;
    let fields = field_tokens(&def.fields);

    quote! {
        #[derive(Debug, Clone, Default)]
        pub struct #name {
            #(#fields),*
        }

        #[::async_trait::async_trait]
        impl ::runie_core::tool::runtime::ToolRuntime for #name {
            fn name(&self) -> &str {
                #tool_name
            }

            fn exec_approval_requirement(&self) -> ::runie_core::tool::runtime::ExecApprovalRequirement {
                ::runie_core::tool::runtime::ExecApprovalRequirement::None
            }

            async fn run(
                &self,
                _ctx: &::runie_core::tool::ToolContext,
            ) -> ::std::result::Result<::runie_core::tool::ToolOutput, ::runie_core::tool::runtime::ToolError> {
                Ok(::runie_core::tool::ToolOutput {
                    tool_name: #tool_name.to_string(),
                    tool_args: ::serde_json::Value::Null,
                    content: #description.to_string(),
                    bytes_transferred: None,
                    duration: ::std::time::Duration::default(),
                    status: ::runie_core::tool::ToolStatus::Success,
                })
            }
        }
    }
}

fn field_tokens(fields: &Punctuated<Field, Token![,]>) -> impl Iterator<Item = proc_macro2::TokenStream> + '_ {
    fields.iter().map(|f| {
        let field_name = &f.name;
        let field_ty = &f.ty;
        quote! { pub #field_name: #field_ty }
    })
}

struct ToolDef {
    name: Ident,
    tool_name: LitStr,
    description: LitStr,
    fields: Punctuated<Field, Token![,]>,
}

impl Parse for ToolDef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name = input.parse()?;
        input.parse::<Token![,]>()?;
        let tool_name = input.parse()?;
        input.parse::<Token![,]>()?;
        let description = input.parse()?;
        input.parse::<Token![,]>()?;
        let content;
        syn::braced!(content in input);
        let fields = content.parse_terminated(Field::parse, Token![,])?;
        Ok(Self {
            name,
            tool_name,
            description,
            fields,
        })
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
