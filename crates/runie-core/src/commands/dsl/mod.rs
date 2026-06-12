//! Command DSL Module
//!
//! Provides a fluent API for defining commands and their flows.

mod flow;
mod builder;
mod category;

pub use flow::{CommandFlow, CommandResult, DialogType};
pub use builder::{CommandDef, FormBuilder, FormField, cmd};
pub use category::CommandCategory;

#[macro_export]
macro_rules! cmd {
    ($name:literal) => { $crate::commands::CommandDef::new($name) };
    ($name:literal, $msg:literal) => { 
        $crate::commands::CommandDef::new($name).msg($msg)
    };
}
