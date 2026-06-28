//! Procedural macros for declarative Runie domain DSLs.

use proc_macro::TokenStream;

mod actor;
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

/// Defines a ractor-based actor with declarative message handling.
///
/// See `crates/runie-macros/src/actor.rs` for full documentation.
#[proc_macro]
pub fn define_actor(input: TokenStream) -> TokenStream {
    actor::expand(input)
}
