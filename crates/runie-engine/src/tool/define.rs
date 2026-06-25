//! Tool definition macro — generates `name()`, `description()`, `input_schema()`,
//! `is_read_only()`, and `requires_approval()` from a declarative struct.
//!
//! # Example
//!
//! ```ignore
//! tool! {
//!     name: "read_file",
//!     description: "Read a file",
//!     schema: {
//!         "path": ("string", "Path to the file"),
//!         "offset": ("integer", "Starting line"),
//!     },
//!     required: ["path"],
//!     read_only: true,
//!     approval: false,
//! }
//! ```
//!
//! This expands to the five accessor methods on the implementing type.

use serde_json::Value;

/// Helper to build an input schema from field declarations.
pub fn build_schema(
    fields: &[(&str, &str, &str)],
    required: &[&str],
) -> Value {
    let properties = fields
        .iter()
        .map(|(name, typ, desc)| {
            (
                name.to_string(),
                serde_json::json!({
                    "type": typ,
                    "description": desc,
                }),
            )
        })
        .collect::<serde_json::Map<_, _>>();

    serde_json::json!({
        "type": "object",
        "properties": properties,
        "required": required,
    })
}

/// A single field declaration for the schema builder.
pub struct Field {
    pub name: &'static str,
    pub typ: &'static str,
    pub description: &'static str,
}

/// Build a schema from a slice of field descriptors.
pub fn schema_from_fields(fields: &[Field], required: &[&str]) -> Value {
    let mut properties = serde_json::Map::new();
    for field in fields {
        properties.insert(
            field.name.to_string(),
            serde_json::json!({
                "type": field.typ,
                "description": field.description,
            }),
        );
    }
    serde_json::json!({
        "type": "object",
        "properties": properties,
        "required": required,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_includes_all_fields() {
        let fields = [
            Field {
                name: "path",
                typ: "string",
                description: "File path",
            },
            Field {
                name: "lines",
                typ: "integer",
                description: "Max lines",
            },
        ];
        let schema = schema_from_fields(&fields, &["path"]);
        let props = schema["properties"].as_object().unwrap();
        assert!(props.contains_key("path"));
        assert!(props.contains_key("lines"));
        assert_eq!(schema["required"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn schema_with_all_required() {
        let fields = [
            Field {
                name: "a",
                typ: "string",
                description: "A",
            },
        ];
        let schema = schema_from_fields(&fields, &["a"]);
        assert!(schema["required"].as_array().unwrap().contains(&serde_json::json!("a")));
    }
}
