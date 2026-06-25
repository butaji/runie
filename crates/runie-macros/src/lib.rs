//! Procedural macros for declarative Runie domain DSLs.

use proc_macro::TokenStream;

mod command;
mod event;
mod hook;
mod policy;

/// Defines a serializable domain event struct.
#[proc_macro]
pub fn define_event(input: TokenStream) -> TokenStream {
    event::expand(input)
}

/// Defines a command and its handler.
#[proc_macro]
pub fn define_command(input: TokenStream) -> TokenStream {
    command::expand(input)
}

/// Defines a hook handler.
#[proc_macro]
pub fn define_hook(input: TokenStream) -> TokenStream {
    hook::expand(input)
}

/// Defines a permission rule.
#[proc_macro]
pub fn define_policy(input: TokenStream) -> TokenStream {
    policy::expand(input)
}
