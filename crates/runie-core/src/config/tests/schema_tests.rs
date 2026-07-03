//! Tests for JSON schema generation and verification.

use std::path::Path;

#[test]
fn generated_schema_matches_checked_in() {
    // This test verifies that config.schema.json is kept in sync with Config type.
    // The schema is generated from the canonical Config type via schemars.
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));

    // Walk up from: runie-core/ (where CARGO_MANIFEST_DIR points)
    // to workspace root: /Users/admin/Code/GitHub/runie-dev
    let workspace_root = manifest_dir.parent().unwrap().parent().unwrap();
    let checked_in = workspace_root.join("config.schema.json");

    // Generate schema from Config type
    let generated = runie_core::config::schema::schema_json();

    // Read checked-in schema
    let checked_in_content = std::fs::read_to_string(&checked_in)
        .unwrap_or_else(|e| panic!("failed to read {}: {}", checked_in.display(), e));

    assert_eq!(
        generated, checked_in_content,
        "config.schema.json is out of date with Config type.\n\
         Run: cargo run -p runie-core --example write_config_schema\n\
         to regenerate the schema."
    );
}
