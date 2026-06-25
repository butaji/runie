//! Generate `config.schema.json` from the canonical `Config` type.
//!
//! Run with: cargo run --example write_config_schema --features schema

use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path = args
        .get(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("config.schema.json"));
    runie_core::config::schema::write_schema(&path).expect("write schema");
    println!("wrote {}", path.display());
}
