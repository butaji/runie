//! JSON Schema generation and writing for `Config`.

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_generation_roundtrips() {
        let value = schema_value();
        assert!(value.get("$schema").is_some());
        assert!(value.get("title").is_some());
    }

    #[test]
    fn schema_includes_config_properties() {
        let value = schema_value();
        let props = value["properties"].as_object().unwrap();
        assert!(props.contains_key("provider"));
        assert!(props.contains_key("model"));
    }
}
