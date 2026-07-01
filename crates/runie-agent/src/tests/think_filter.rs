//! Tests for ThinkFilter — Layer 1 (State / Logic)

use super::*;
use crate::think_filter::ThinkFilter;
use runie_core::provider_event::ProviderEvent;
use runie_core::provider_event::StopReason;

fn ts() -> ProviderEvent {
    ProviderEvent::ThinkingStart { id: "inline".to_string() }
}
fn te() -> ProviderEvent {
    ProviderEvent::ThinkingEnd { id: "inline".to_string() }
}
fn td(s: &str) -> ProviderEvent {
    ProviderEvent::ThinkingDelta(s.to_string())
}
fn textd(s: &str) -> ProviderEvent {
    ProviderEvent::TextDelta(s.to_string())
}

#[test]
fn think_filter_buffers_partial_open_tag_then_completes_across_chunks() {
    let mut f = ThinkFilter::new();
    // Chunk ends mid-tag: content emitted, partial buffered.
    let out1 = f.feed(ProviderEvent::TextDelta("hi <tool_call>".into()));
    assert_eq!(out1, vec![textd("hi ")]);
    // Next chunk completes the tag and opens thinking.
    let out2 = f.feed(ProviderEvent::TextDelta(">\nreasoning".into()));
    assert_eq!(out2, vec![ts(), td(">\nreasoning")]);
}

#[test]
fn think_filter_passthrough_plain_text() {
    let mut f = ThinkFilter::new();
    let out = f.feed(ProviderEvent::TextDelta("hello world".into()));
    assert_eq!(out, vec![textd("hello world")]);
}

#[test]
fn think_filter_extracts_closed_thinking_block() {
    let mut f = ThinkFilter::new();
    let out = f.feed(ProviderEvent::TextDelta(
        "before <tool_call>\nreasoning\n</thinking>\nafter".into(),
    ));
    assert_eq!(
        out,
        vec![
            textd("before "),
            ts(),
            td("\nreasoning\n"),
            te(),
            textd("\nafter"),
        ]
    );
}

#[test]
fn think_filter_handles_angle_bracket_tags() {
    let mut f = ThinkFilter::new();
    let out = f.feed(ProviderEvent::TextDelta(
        "before <thinking>\nreasoning\n</thinking>\nafter".into(),
    ));
    assert_eq!(
        out,
        vec![
            textd("before "),
            ts(),
            td("\nreasoning\n"),
            te(),
            textd("\nafter"),
        ]
    );
}

#[test]
fn think_filter_buffers_partial_open_tag() {
    let mut f = ThinkFilter::new();
    // First chunk ends mid-tag.
    let out1 = f.feed(ProviderEvent::TextDelta("hi <tool_call>".into()));
    assert_eq!(out1, vec![textd("hi ")]); // partial held back
    // Second chunk completes the tag.
    let out2 = f.feed(ProviderEvent::TextDelta(">\nreasoning".into()));
    assert_eq!(out2, vec![ts(), td(">\nreasoning")]);
}

#[test]
fn think_filter_buffers_partial_close_tag() {
    let mut f = ThinkFilter::new();
    // Enter thinking block.
    f.feed(ProviderEvent::TextDelta("<tool_call>\nreasoning ".into()));
    // Delta ends mid-closing-tag: buffer the partial, emit thinking content.
    let out1 = f.feed(ProviderEvent::TextDelta("</think".into()));
    assert_eq!(out1, vec![td("\nreasoning ")]); // content before partial tag
    // Second chunk completes the closing tag.
    let out2 = f.feed(ProviderEvent::TextDelta("ing>\nafter".into()));
    assert_eq!(out2, vec![te(), textd("\nafter")]);
}

#[test]
fn think_filter_passes_structured_thinking_delta_unchanged() {
    let mut f = ThinkFilter::new();
    let out = f.feed(ProviderEvent::ThinkingDelta("reasoning from provider".into()));
    assert_eq!(out, vec![td("reasoning from provider")]);
}

#[test]
fn think_filter_flush_drains_open_block() {
    let mut f = ThinkFilter::new();
    f.feed(ProviderEvent::TextDelta("<tool_call>\nunfinished".into()));
    let out = f.flush();
    assert_eq!(out, vec![td("\nunfinished"), te()]);
}

#[test]
fn think_filter_flush_drains_partial_tag_as_text() {
    let mut f = ThinkFilter::new();
    // Partial opening tag never resolved.
    f.feed(ProviderEvent::TextDelta("hi <tool_call>".into()));
    let out = f.flush();
    assert_eq!(out, vec![textd("<tool_call>")]);
}

#[test]
fn think_filter_nested_tags_track_depth() {
    let mut f = ThinkFilter::new();
    // "<tool_call> inner <tool_call> deep </thinking> </thinking> after"
    // Flat model: outer opens, inner opens (closes outer first), deep,
    // first </thinking> closes inner, second </thinking> closes nothing (text).
    let out = f.feed(ProviderEvent::TextDelta(
        "<tool_call> inner <tool_call> deep </thinking> </thinking> after".into(),
    ));
    assert_eq!(
        out,
        vec![
            ts(),          // open outer
            td(" inner "),
            te(),          // close outer (inner opens, flattening)
            ts(),          // open inner
            td(" deep "),
            te(),          // close inner
            textd(" </thinking> after"), // second </thinking> is text
        ]
    );
}

#[test]
fn think_filter_tool_call_start_flushes_buffer() {
    let mut f = ThinkFilter::new();
    f.feed(ProviderEvent::TextDelta("<tool_call>\nunfinished".into()));
    let out = f.feed(ProviderEvent::ToolCallStart {
        id: "c1".into(),
        name: "bash".into(),
    });
    assert_eq!(out.len(), 3);
    assert!(matches!(out[0], ProviderEvent::ThinkingDelta(_)));
    assert!(matches!(out[1], ProviderEvent::ThinkingEnd { .. }));
    assert!(matches!(&out[2], ProviderEvent::ToolCallStart { id, name } if id == "c1" && name == "bash"));
}

#[test]
fn think_filter_finish_flushes_buffer() {
    let mut f = ThinkFilter::new();
    f.feed(ProviderEvent::TextDelta("<tool_call>\nunfinished".into()));
    let out = f.feed(ProviderEvent::Finish {
        reason: StopReason::Stop,
    });
    assert!(matches!(&out[0], ProviderEvent::ThinkingDelta(_)));
    assert!(matches!(&out[1], ProviderEvent::ThinkingEnd { .. }));
    assert!(matches!(&out[2], ProviderEvent::Finish { .. }));
}

#[test]
fn think_filter_usage_flushes_buffer() {
    let mut f = ThinkFilter::new();
    f.feed(ProviderEvent::TextDelta("<thinking>unresolved".into()));
    let out = f.feed(ProviderEvent::Usage {
        input_tokens: 100,
        output_tokens: 50,
    });
    assert!(matches!(&out[0], ProviderEvent::ThinkingDelta(_)));
    assert!(matches!(&out[1], ProviderEvent::ThinkingEnd { .. }));
    assert!(matches!(&out[2], ProviderEvent::Usage { .. }));
}

#[test]
fn think_filter_error_flushes_buffer() {
    let mut f = ThinkFilter::new();
    f.feed(ProviderEvent::TextDelta("<tool_call>open".into()));
    let err = ProviderEvent::Error(runie_core::provider_event::ModelError::Other(anyhow::anyhow!("oops")));
    let out = f.feed(err.clone());
    assert!(matches!(&out[0], ProviderEvent::ThinkingDelta(_)));
    assert!(matches!(&out[1], ProviderEvent::ThinkingEnd { .. }));
    assert_eq!(out[2], err);
}

#[test]
fn think_filter_multiple_open_close_cycles() {
    let mut f = ThinkFilter::new();
    let out = f.feed(ProviderEvent::TextDelta(
        "<tool_call>a</thinking><tool_call>b</thinking>c".into(),
    ));
    assert_eq!(
        out,
        vec![
            ts(),
            td("a"),
            te(),
            ts(),
            td("b"),
            te(),
            textd("c"),
        ]
    );
}

#[test]
fn think_filter_empty_delta_preserves_state() {
    let mut f = ThinkFilter::new();
    f.feed(ProviderEvent::TextDelta("<tool_call>".into()));
    let out = f.feed(ProviderEvent::TextDelta("".into()));
    // Empty delta: buffer still holds "<tool_call>", nothing emitted.
    assert!(out.is_empty());
    // Now complete the block.
    let out2 = f.feed(ProviderEvent::TextDelta("done</thinking>".into()));
    assert_eq!(out2, vec![ts(), td("done"), te()]);
}
