//! Replay captured MiniMax SSE streams and snapshot the resulting events.

use runie_provider::openai::stream::replay_sse;

fn fixture(name: &str) -> String {
    std::fs::read_to_string(format!(
        "{}/tests/fixtures/minimax/{}",
        env!("CARGO_MANIFEST_DIR"),
        name
    ))
    .unwrap()
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
