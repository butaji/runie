//! JSON Schema generation and writing for `Config`.
//!
//! This module requires `schemars` as a dependency.

use std::path::Path;

use serde_json::Value;

use crate::config::Config;

/// Generate the JSON schema for `Config` as a JSON value.
pub fn schema_value() -> Value {
    serde_json::to_value(schemars::schema_for!(Config)).expect("schema serializes")
}

/// Generate the JSON schema for `Config` as a pretty-printed string.
pub fn schema_json() -> String {
    serde_json::to_string_pretty(&schemars::schema_for!(Config)).expect("schema serializes")
}

/// Write the JSON schema to a file.
pub fn write_schema(path: impl AsRef<Path>) -> std::io::Result<()> {
    std::fs::write(path, schema_json())
}
