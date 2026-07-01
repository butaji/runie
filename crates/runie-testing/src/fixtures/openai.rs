//! OpenAI SSE fixture strings — shared by runie-agent and runie-provider tests.
//!
//! Uses `include_dir!` to scan the fixture directory at compile time.

use include_dir::Dir;
use std::collections::HashMap;
use std::sync::LazyLock;

/// The fixture directory.
static FIXTURE_DIR: Dir<'_> = include_dir::include_dir!("$CARGO_MANIFEST_DIR/src/fixtures/openai");

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

/// All OpenAI fixture names.
pub const ALL_FIXTURES: &[&str] = &[
    "simple_text_delta.sse",
    "reasoning_content.sse",
    "parallel_tool_calls.sse",
    "rate_limit_error.sse",
];

/// Load a fixture by name. Panics if the name is unknown.
pub fn fixture(name: &str) -> String {
    FIXTURES
        .get(name)
        .map(|s| (*s).to_string())
        .unwrap_or_else(|| panic!("unknown openai fixture: {}", name))
}
