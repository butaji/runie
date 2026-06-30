//! MiniMax SSE fixture strings — shared by runie-agent and runie-provider tests.
//!
//! Uses `include_dir!` to scan the fixture directory at compile time.

use include_dir::Dir;
use std::collections::HashMap;
use std::sync::LazyLock;

/// The fixture directory.
static FIXTURE_DIR: Dir<'_> = include_dir::include_dir!("$CARGO_MANIFEST_DIR/src/fixtures/minimax");

/// Lazy-loaded fixture contents.
static FIXTURES: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    FIXTURE_DIR
        .files()
        .map(|file| {
            let name = file.path().file_name().unwrap().to_str().unwrap();
            let contents = file.contents_utf8().unwrap_or_default();
            (name, contents)
        })
        .collect()
});

/// All MiniMax fixture names.
pub const ALL_FIXTURES: &[&str] = &[
    "m3_read_file_final.sse",
    "m3_read_file_call.sse",
    "m3_multi_tool_readme.sse",
    "m3_multi_tool_list_dir.sse",
    "m3_list_files_final.sse",
    "m3_list_files_call.sse",
    "m27_multi_tool_readme.sse",
];

/// Load a fixture by name. Panics if the name is unknown.
pub fn fixture(name: &str) -> String {
    FIXTURES
        .get(name)
        .map(|s| (*s).to_string())
        .unwrap_or_else(|| panic!("unknown fixture: {}", name))
}
