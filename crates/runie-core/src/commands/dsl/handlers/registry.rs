//! Command Handler Registry
//!
//! Maps command names to handler functions. This allows command metadata
//! to be stored in YAML while keeping execution logic in Rust.

use std::collections::HashMap;

use crate::commands::dsl::spec::FormHandler;
use crate::model::AppState;

use crate::commands::dsl::CommandKind;

/// A named handler function.
#[derive(Clone)]
pub enum NamedHandler {
    /// Simple handler function.
    Handler(fn(&mut AppState, &str) -> crate::commands::CommandResult),
    /// Form with custom handler.
    FormWithHandler {
        title: &'static str,
        fields: &'static [(&'static str, &'static str, &'static str)],
        handler: FormHandler,
    },
}

/// Global registry of command handlers.
/// Maps command name -> handler function.
pub struct HandlerRegistry {
    handlers: HashMap<&'static str, NamedHandler>,
}

impl HandlerRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    /// Register a handler.
    pub fn register(&mut self, name: &'static str, handler: NamedHandler) {
        self.handlers.insert(name, handler);
    }

    /// Look up a handler by name.
    pub fn get(&self, name: &str) -> Option<&NamedHandler> {
        self.handlers.get(name)
    }

    /// Iterate over all handlers.
    pub fn handlers(&self) -> impl Iterator<Item = (&str, &NamedHandler)> {
        self.handlers.iter().map(|(k, v)| (*k, v))
    }

    /// Convert a named handler to a `CommandKind`.
    pub fn to_command_kind(&self, name: &str) -> Option<CommandKind> {
        self.get(name).map(|h| match h {
            NamedHandler::Handler(f) => CommandKind::Handler(*f),
            NamedHandler::FormWithHandler {
                title,
                fields,
                handler,
            } => CommandKind::FormWithHandler {
                title,
                fields,
                handler: *handler,
            },
        })
    }
}

impl Default for HandlerRegistry {
    fn default() -> Self {
        Self::new()
    }
}
