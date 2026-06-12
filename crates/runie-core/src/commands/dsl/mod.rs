//! Command DSL Module
//!
//! Provides a fluent API for defining commands and their flows.

mod builder;
mod category;
mod flow;

pub use builder::{cmd, CommandDef, FormBuilder, FormField};
pub use category::CommandCategory;
pub use flow::{CommandFlow, CommandResult, DialogType};

#[macro_export]
macro_rules! cmd {
    ($name:literal) => {
        $crate::commands::CommandDef::new($name)
    };
    ($name:literal, $msg:literal) => {
        $crate::commands::CommandDef::new($name).msg($msg)
    };
}
