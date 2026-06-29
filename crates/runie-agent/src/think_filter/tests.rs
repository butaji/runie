//! Tests for ThinkFilter.

use super::*;

fn ev(delta: &str) -> ProviderEvent {
    ProviderEvent::TextDelta(delta.into())
}
fn td(delta: &str) -> ProviderEvent {
    ProviderEvent::ThinkingDelta(delta.into())
}

fn has_ts(events: &[ProviderEvent]) -> bool {
    events
        .iter()
        .any(|e| matches!(e, ProviderEvent::ThinkingStart { .. }))
}
fn has_te(events: &[ProviderEvent]) -> bool {
    events
        .iter()
        .any(|e| matches!(e, ProviderEvent::ThinkingEnd { .. }))
}
fn has_td(events: &[ProviderEvent]) -> bool {
    events
        .iter()
        .any(|e| matches!(e, ProviderEvent::ThinkingDelta(_)))
}
fn td_content(events: &[ProviderEvent]) -> String {
    events
        .iter()
        .filter_map(|e| match e {
            ProviderEvent::ThinkingDelta(s) => Some(s.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("")
}
fn text_content(events: &[ProviderEvent]) -> String {
    events
        .iter()
        .filter_map(|e| match e {
            ProviderEvent::TextDelta(s) => Some(s.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("")
}

/// Layer 1: ThinkFilter passes plain text through unchanged.
#[test]
fn passes_plain_text() {
    let mut f = ThinkFilter::new();
    let out: Vec<_> = f.feed(ev("hello world")).into_iter().collect();
    assert_eq!(out, vec![ev("hello world")]);
}

/// Layer 1: Closed thinking block becomes TS + content + TE.
#[test]
fn strips_closed_block() {
    let mut f = ThinkFilter::new();
    let out: Vec<_> = f
        .feed(ev("<tool_call>planning</tool_call>"))
        .into_iter()
        .collect();
    assert!(has_ts(&out));
    assert!(has_td(&out));
    assert!(has_te(&out));
    assert!(td_content(&out).contains("planning"));
}

/// Layer 1: Partial opening tag is buffered until complete.
#[test]
fn partial_open_tag() {
    let mut f = ThinkFilter::new();
    let out1: Vec<_> = f.feed(ev("<tool")).into_iter().collect();
    assert!(out1.is_empty());
    let out2: Vec<_> = f
        .feed(ev("_call>analysis</thinking>"))
        .into_iter()
        .collect();
    assert!(has_ts(&out2));
    assert!(has_td(&out2));
    assert!(has_te(&out2));
}

/// Layer 1: Partial closing tag is buffered until complete.
#[test]
fn partial_close_tag() {
    let mut f = ThinkFilter::new();
    let _ = f.feed(ev("<tool_call>")).into_iter().collect::<Vec<_>>();
    let out1: Vec<_> = f.feed(ev("some thought</th")).into_iter().collect();
    assert!(td_content(&out1).contains("some thought"));
    let out2: Vec<_> = f.feed(ev("inking>")).into_iter().collect();
    assert!(has_te(&out2));
}

/// Layer 1: ThinkingDelta from provider passes through unchanged.
#[test]
fn thinking_delta_passthrough() {
    let mut f = ThinkFilter::new();
    let out: Vec<_> = f.feed(td("already structured")).into_iter().collect();
    assert_eq!(out, vec![td("already structured")]);
}

/// Layer 1: flush() emits ThinkingEnd for unclosed blocks.
#[test]
fn flush_emits_te() {
    let mut f = ThinkFilter::new();
    let _ = f.feed(ev("<thinking>unclosed"))
        .into_iter()
        .collect::<Vec<_>>();
    let flushed: Vec<_> = f.flush().into_iter().collect();
    assert!(has_te(&flushed));
}

/// Layer 1: flush() with no open block emits nothing.
#[test]
fn flush_empty() {
    let mut f = ThinkFilter::new();
    let _ = f.feed(ev("plain")).into_iter().collect::<Vec<_>>();
    let flushed: Vec<_> = f.flush().into_iter().collect();
    assert!(flushed.is_empty());
}

/// Layer 1: Nested opening tags emit ThinkingEnd before re-opening.
#[test]
fn nested_opens() {
    let mut f = ThinkFilter::new();
    let out: Vec<_> = f
        .feed(ev("<thinking>first</thinking><tool_call>second</thinking>"))
        .into_iter()
        .collect();
    assert_eq!(
        out.iter()
            .filter(|e| matches!(e, ProviderEvent::ThinkingStart { .. }))
            .count(),
        2
    );
    assert_eq!(
        out.iter()
            .filter(|e| matches!(e, ProviderEvent::ThinkingEnd { .. }))
            .count(),
        2
    );
}

/// Layer 1: Empty thinking block emits TS + TE, no delta.
#[test]
fn empty_block() {
    let mut f = ThinkFilter::new();
    let out: Vec<_> = f.feed(ev("<thinking></thinking>")).into_iter().collect();
    assert!(has_ts(&out));
    assert!(has_te(&out));
    assert!(!has_td(&out));
}

/// Layer 1: TextDelta after flush resumes as plain text.
#[test]
fn after_flush() {
    let mut f = ThinkFilter::new();
    let _ = f.feed(ev("<thinking>done</thinking>"))
        .into_iter()
        .collect::<Vec<_>>();
    let _ = f.flush().into_iter().collect::<Vec<_>>();
    let out: Vec<_> = f.feed(ev("more text")).into_iter().collect();
    assert_eq!(out, vec![ev("more text")]);
}

/// Layer 1: Multiple chunks in same block accumulate correctly.
#[test]
fn multiple_chunks() {
    let mut f = ThinkFilter::new();
    let _ = f.feed(ev("<tool_call>first"))
        .into_iter()
        .collect::<Vec<_>>();
    let out2: Vec<_> = f.feed(ev(" second")).into_iter().collect();
    let out3: Vec<_> = f.feed(ev(" third</thinking>")).into_iter().collect();
    let all = td_content(&[out2, out3].concat());
    assert!(all.contains("first"));
    assert!(all.contains("second"));
    assert!(all.contains("third"));
}

/// Layer 1: Two back-to-back thinking blocks.
#[test]
fn adjacent_blocks() {
    let mut f = ThinkFilter::new();
    let out: Vec<_> = f
        .feed(ev("<thinking>a</thinking><thinking>b</thinking>"))
        .into_iter()
        .collect();
    assert_eq!(
        out.iter()
            .filter(|e| matches!(e, ProviderEvent::ThinkingStart { .. }))
            .count(),
        2
    );
    assert_eq!(
        out.iter()
            .filter(|e| matches!(e, ProviderEvent::ThinkingEnd { .. }))
            .count(),
        2
    );
}

/// Layer 1: Text before and after thinking block preserved.
#[test]
fn text_before_and_after() {
    let mut f = ThinkFilter::new();
    let out: Vec<_> = f
        .feed(ev("before<thinking>think</thinking>after"))
        .into_iter()
        .collect();
    let texts = text_content(&out);
    assert!(texts.contains("before"));
    assert!(texts.contains("after"));
}

/// Layer 1: flush() emits ThinkingStart + ThinkingDelta + ThinkingEnd for partial open tag.
#[test]
fn flush_with_open() {
    let mut f = ThinkFilter::new();
    let _ = f.feed(ev("<thinking>")).into_iter().collect::<Vec<_>>();
    let flushed: Vec<_> = f.flush().into_iter().collect();
    assert_eq!(flushed.len(), 3);
    assert!(matches!(&flushed[0], ProviderEvent::ThinkingStart { .. }));
    assert!(matches!(&flushed[1], ProviderEvent::ThinkingDelta(_)));
    assert!(matches!(&flushed[2], ProviderEvent::ThinkingEnd { .. }));
}

/// Layer 1: Duplicate </thinking> after whitespace is skipped (only 1 block).
#[test]
fn whitespace_between_closes() {
    let mut f = ThinkFilter::new();
    let out: Vec<_> = f
        .feed(ev("<thinking>a</thinking>   </thinking>b"))
        .into_iter()
        .collect();
    assert_eq!(
        out.iter()
            .filter(|e| matches!(e, ProviderEvent::ThinkingStart { .. }))
            .count(),
        1
    );
    assert_eq!(
        out.iter()
            .filter(|e| matches!(e, ProviderEvent::ThinkingEnd { .. }))
            .count(),
        1
    );
    assert!(text_content(&out).contains("b"));
}

/// Layer 1: emit helpers produce correct events.
#[test]
fn emit_helpers() {
    use super::{emit_text, emit_thinking, emit_thinking_end, emit_thinking_start};
    let mut out = Vec::new();
    emit_thinking_start(&mut out);
    emit_thinking(&mut out, "test".into());
    emit_thinking_end(&mut out);
    emit_text(&mut out, "plain".into());
    assert_eq!(out.len(), 4);
}
