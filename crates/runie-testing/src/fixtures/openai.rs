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
    "opencode_go_deepseek_v4_flash_multi_tool.sse",
    "opencode_go_deepseek_v4_flash_reasoning.sse",
    "opencode_go_deepseek_v4_flash_simple.sse",
    "opencode_go_deepseek_v4_flash_tool.sse",
    "opencode_go_deepseek_v4_pro_multi_tool.sse",
    "opencode_go_deepseek_v4_pro_reasoning.sse",
    "opencode_go_deepseek_v4_pro_simple.sse",
    "opencode_go_deepseek_v4_pro_tool.sse",
    "opencode_go_glm_5_1_simple.sse",
    "opencode_go_glm_5_1_tool.sse",
    "opencode_go_glm_5_2_multi_tool.sse",
    "opencode_go_glm_5_2_reasoning.sse",
    "opencode_go_glm_5_2_simple.sse",
    "opencode_go_glm_5_2_tool.sse",
    "opencode_go_glm_5_simple.sse",
    "opencode_go_glm_5_tool.sse",
    "opencode_go_kimi_k2_5_simple.sse",
    "opencode_go_kimi_k2_5_tool.sse",
    "opencode_go_kimi_k2_6_multi_tool.sse",
    "opencode_go_kimi_k2_6_reasoning.sse",
    "opencode_go_kimi_k2_6_simple.sse",
    "opencode_go_kimi_k2_6_tool.sse",
    "opencode_go_kimi_k2_7_code_simple.sse",
    "opencode_go_kimi_k2_7_code_tool.sse",
    "opencode_go_mimo_v2_5_multi_tool.sse",
    "opencode_go_mimo_v2_5_pro_simple.sse",
    "opencode_go_mimo_v2_5_pro_tool.sse",
    "opencode_go_mimo_v2_5_reasoning.sse",
    "opencode_go_mimo_v2_5_simple.sse",
    "opencode_go_mimo_v2_5_tool.sse",
    "parallel_tool_calls.sse",
    "rate_limit_error.sse",
    "reasoning_content.sse",
    "simple_text_delta.sse",
];

/// Load a fixture by name. Panics if the name is unknown.
pub fn fixture(name: &str) -> String {
    FIXTURES
        .get(name)
        .map(|s| (*s).to_string())
        .unwrap_or_else(|| panic!("unknown openai fixture: {}", name))
}
