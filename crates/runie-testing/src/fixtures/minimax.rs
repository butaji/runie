//! MiniMax SSE fixture strings — shared by runie-agent and runie-provider tests.
//!
//! All 7 fixtures are byte-identical copies. This module is the single source of
//! truth; crates that previously duplicated them must load from here instead.

macro_rules! define_fixtures {
    ($($name:ident),* $(,)?) => {
        $(
            #[doc = concat!("Contents of `minimax/", stringify!($name), ".sse`.")]
            pub const $name: &str = include_str!(concat!("minimax/", stringify!($name), ".sse"));
        )*
    };
}

define_fixtures! {
    M3_READ_FILE_FINAL,
    M3_READ_FILE_CALL,
    M3_MULTI_TOOL_README,
    M3_MULTI_TOOL_LIST_DIR,
    M3_LIST_FILES_FINAL,
    M3_LIST_FILES_CALL,
    M27_MULTI_TOOL_README,
}

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
