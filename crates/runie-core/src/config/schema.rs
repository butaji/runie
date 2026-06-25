//! JSON Schema generation and writing for `Config`.
//!
//! This module is only available when the `schema` feature is enabled
//! or during test compilation. It requires `schemars` as a dev-dependency.

use std::path::Path;

use serde_json::Value;

#[cfg(feature = "schema")]
use crate::config::Config;

/// Generate the JSON schema for `Config` as a JSON value.
/// Only available with `schema` feature or during test compilation.
#[cfg(feature = "schema")]
pub fn schema_value() -> Value {
    serde_json::to_value(schemars::schema_for!(Config)).expect("schema serializes")
}

/// Generate the JSON schema for `Config` as a pretty-printed string.
/// Only available with `schema` feature or during test compilation.
#[cfg(feature = "schema")]
pub fn schema_json() -> String {
    serde_json::to_string_pretty(&schemars::schema_for!(Config)).expect("schema serializes")
}

/// Write the JSON schema to a file.
/// Only available with `schema` feature or during test compilation.
#[cfg(feature = "schema")]
pub fn write_schema(path: impl AsRef<Path>) -> std::io::Result<()> {
    std::fs::write(path, schema_json())
}



