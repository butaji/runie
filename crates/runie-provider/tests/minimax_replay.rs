//! Replay captured MiniMax SSE streams and snapshot the resulting events.

use runie_core::provider_event::ProviderEvent;
use runie_provider::openai::stream::replay_sse;
use runie_testing::fixtures::minimax::fixture;

/// ISSUE D: MiniMax puts reasoning and tool calls inside `delta.content`.
/// No `TextDelta` may ever leak the raw `<think>` / `<minimax:tool_call>` /
/// `<tool_call>` / `</invoke>` tags or an inline `{"name","arguments"}` blob —
/// those must be routed to the thinking / structured-tool paths instead.
fn assert_no_raw_markup_in_text(events: &[ProviderEvent]) {
    let joined: String = events
        .iter()
        .filter_map(|e| match e {
            ProviderEvent::TextDelta(t) => Some(t.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("");
    for forbidden in [
        "<think",
        "</think>",
        "<minimax:tool_call",
        "</minimax:tool_call>",
        "<tool_call>",
        "</tool_call>",
        "</invoke>",
        "\"name\"",
        "\"arguments\"",
    ] {
        assert!(
            !joined.contains(forbidden),
            "TextDelta leaked raw MiniMax markup {forbidden:?}; joined text = {joined:?}"
        );
    }
}

#[test]
fn m3_list_files_emits_text_and_json_tool_call() {
    let events = replay_sse(&fixture("m3_list_files_call.sse"));
    assert!(events
        .iter()
        .any(|e| matches!(e, ProviderEvent::TextDelta(_))));
    assert!(events.iter().any(|e| matches!(
        e,
        ProviderEvent::Finish { reason }
        if *reason == runie_core::provider_event::StopReason::Stop
    )));
    assert_no_raw_markup_in_text(&events);
    insta::assert_debug_snapshot!(events);
}

#[test]
fn m3_read_file_emits_text_and_json_tool_call() {
    let events = replay_sse(&fixture("m3_read_file_call.sse"));
    assert_no_raw_markup_in_text(&events);
    insta::assert_debug_snapshot!(events);
}

#[test]
fn m3_multi_tool_readme_emits_delimited_xml_tool_call() {
    let events = replay_sse(&fixture("m3_multi_tool_readme.sse"));
    assert_no_raw_markup_in_text(&events);
    insta::assert_debug_snapshot!(events);
}

#[test]
fn m27_multi_tool_readme_emits_standard_xml_tool_call() {
    let events = replay_sse(&fixture("m27_multi_tool_readme.sse"));
    assert_no_raw_markup_in_text(&events);
    insta::assert_debug_snapshot!(events);
}
