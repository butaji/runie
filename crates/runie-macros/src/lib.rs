//! Procedural macros for declarative Runie domain DSLs.

use proc_macro::TokenStream;

mod command;
mod event;
mod hook;
mod policy;
mod tool;

/// Defines a serializable domain event struct.
#[proc_macro]
pub fn define_event(input: TokenStream) -> TokenStream {
    event::expand(input)
}

/// Defines a tool struct and a `ToolRuntime` implementation.
#[proc_macro]
pub fn define_tool(input: TokenStream) -> TokenStream {
    tool::expand(input)
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
