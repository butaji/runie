//! Tool definition helpers — generates `name()`, `description()`, `input_schema()`,
//! `is_read_only()`, and `requires_approval()` from declarative structs.
//!
//! Use the [`define_tool!`] macro to define tools with less boilerplate.
//!
//! # Example
//!
//! ```ignore
//! use crate::tool::define_tool;
//!
//! define_tool! {
//!     name: "read_file",
//!     description: "Read a file from disk",
//!     read_only: true,
//!     approval: false,
//!     fields: {
//!         "path": (string, "Path to the file"),
//!         "offset": (integer, "Starting line (0-based)"),
//!         "limit": (integer, "Maximum lines to read"),
//!     },
//!     required: ["path"]
//! }
//! ```

use serde_json::{json, Value};

/// Helper to build an input schema from field declarations.
pub fn build_schema(fields: &[(&str, &str, &str)], required: &[&str]) -> Value {
    let properties = fields
        .iter()
        .map(|(name, typ, desc)| {
            (
                name.to_string(),
                json!({
                    "type": typ,
                    "description": desc,
                }),
            )
        })
        .collect::<serde_json::Map<_, _>>();

    json!({
        "type": "object",
        "properties": properties,
        "required": required,
    })
}

// =============================================================================
// Declarative macro for tool definitions
// =============================================================================

/// Declare a tool with its metadata.
///
/// # Example
///
/// ```ignore
/// define_tool! {
///     name: "read_file",
///     description: "Read a file from disk",
///     read_only: true,
///     approval: false,
///     fields: {
///         "path": ("string", "Path to the file"),
///     },
///     required: ["path"]
/// }
/// ```
///
/// This expands to:
///
/// ```ignore
/// fn name(&self) -> &str { "read_file" }
/// fn description(&self) -> &str { "Read a file from disk" }
/// fn input_schema(&self) -> Value { /* ... */ }
/// fn is_read_only(&self) -> bool { true }
/// fn requires_approval(&self, _input: &Value) -> bool { false }
/// ```
#[macro_export]
macro_rules! define_tool {
    (
        name: $name:expr_2021,
        description: $desc:expr_2021,
        read_only: $ro:expr_2021,
        approval: $approval:expr_2021,
        fields: {
            $($fname:literal: ($ftype:literal, $fdesc:literal)),* $(,)?
        },
        required: [$($req:literal),* $(,)?]
    ) => {
        fn name(&self) -> &str {
            $name
        }
        fn description(&self) -> &str {
            $desc
        }
        fn input_schema(&self) -> Value {
            let fields = [
                $(( $fname, $ftype, $fdesc )),*
            ];
            let required: &[&str] = &[$($req),*];
            $crate::tool::define::build_schema(&fields, required)
        }
        fn is_read_only(&self) -> bool {
            $ro
        }
        fn requires_approval(&self, _input: &Value) -> bool {
            $approval
        }
    };
}
