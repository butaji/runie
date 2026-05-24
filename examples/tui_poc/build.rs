//! Build script that compiles app.tsx to Rust.
//!
//! Reads the TSX source, transpiles it to Rust using the runie compiler,
//! and writes the output to src/generated_main.rs.

use std::env;
use std::fs;
use std::path::Path;
use runie::compile_tsx;

fn main() {
    // Get the manifest directory (examples/tui_poc)
    let manifest_dir = env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR not set");

    let tsx_path = Path::new(&manifest_dir).join("app.tsx");
    let output_path = Path::new(&manifest_dir).join("src").join("generated_main.rs");

    println!("cargo:rerun-if-changed={}", tsx_path.display());

    // Compile TSX to Rust
    let rust_code = match compile_tsx(&tsx_path) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("TSX compile error: {}", e);
            // Read the TSX to see what we're parsing
            let tsx_content = fs::read_to_string(&tsx_path).unwrap_or_default();
            eprintln!("TSX content:\n{}", tsx_content);
            panic!("Failed to compile TSX: {}", e);
        }
    };

    // Write generated code
    fs::write(&output_path, &rust_code)
        .expect("Failed to write generated_main.rs");

    println!("cargo:warning=Generated Rust code written to {}", output_path.display());

    // Tell Cargo to rerun this build script if the TSX file changes
    println!("cargo:rerun-if-changed={}", tsx_path.display());
}
