//! Proc macros for runie-ext

use proc_macro::TokenStream;
use quote::quote;
use syn;

/// `#[plugin]` macro - registers a plugin with the global registry
///
/// Usage:
/// ```ignore
/// #[runie_ext::plugin]
/// static MY_PLUGIN: MyPlugin = MyPlugin;
/// ```
#[proc_macro]
pub fn plugin(input: TokenStream) -> TokenStream {
    let item = syn::parse_macro_input!(input as syn::ItemStatic);

    let name = &item.ident;
    let ty = &item.ty;

    quote! {
        #item

        // Register with global registry
        const _: () = {
            use runie_ext::{ExtensionRegistry, Plugin};
            use std::sync::Arc;

            static __PLUGIN: Arc<dyn Plugin> = Arc::new(#name);

            #[ctor::ctor]
            fn __register_plugin() {
                let registry = ExtensionRegistry::default();
                let _ = registry.register(__PLUGIN.clone());
            }
        };
    }.into()
}
