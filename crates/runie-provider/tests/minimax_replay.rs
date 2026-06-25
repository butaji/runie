//! Replay captured MiniMax SSE streams and snapshot the resulting events.

use runie_provider::openai::stream::replay_sse;
use runie_testing::fixtures::minimax as fixtures;

fn fixture(name: &str) -> String {
    // Use the shared fixture constants from runie-testing
    match name {
        "m3_list_files_call.sse" => fixtures::M3_LIST_FILES_CALL.to_string(),
        "m3_list_files_final.sse" => fixtures::M3_LIST_FILES_FINAL.to_string(),
        "m3_read_file_call.sse" => fixtures::M3_READ_FILE_CALL.to_string(),
        "m3_read_file_final.sse" => fixtures::M3_READ_FILE_FINAL.to_string(),
        "m3_multi_tool_list_dir.sse" => fixtures::M3_MULTI_TOOL_LIST_DIR.to_string(),
        "m3_multi_tool_readme.sse" => fixtures::M3_MULTI_TOOL_README.to_string(),
        "m27_multi_tool_readme.sse" => fixtures::M27_MULTI_TOOL_README.to_string(),
        _ => unreachable!(),
    }
}

#[test]
fn m3_list_files_emits_text_and_json_tool_call() {
    let events = replay_sse(&fixture("m3_list_files_call.sse"));
    assert!(events.iter().any(|e| matches!(e, runie_core::provider_event::ProviderEvent::TextDelta(_))));
    assert!(events.iter().any(|e| matches!(
        e,
        runie_core::provider_event::ProviderEvent::Finish { reason }
        if *reason == runie_core::provider_event::StopReason::Stop
    )));
    insta::assert_debug_snapshot!(events);
}

#[test]
fn m3_read_file_emits_text_and_json_tool_call() {
    let events = replay_sse(&fixture("m3_read_file_call.sse"));
    insta::assert_debug_snapshot!(events);
}

#[test]
fn m3_multi_tool_readme_emits_delimited_xml_tool_call() {
    let events = replay_sse(&fixture("m3_multi_tool_readme.sse"));
    insta::assert_debug_snapshot!(events);
}

#[test]
fn m27_multi_tool_readme_emits_standard_xml_tool_call() {
    let events = replay_sse(&fixture("m27_multi_tool_readme.sse"));
    insta::assert_debug_snapshot!(events);
}
