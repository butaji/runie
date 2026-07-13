//! Tests for the streaming buffer logic (Layer 1 tests).

use crate::streaming_buffer::StreamingBuffer;

#[test]
fn buffer_flushes_complete_paragraph() {
    let mut buf = StreamingBuffer::new();
    buf.push_delta("Hello, world!\n\n");
    let flushed = buf.force_flush();
    // Split on \n gives ["Hello, world!", "", ""]; stable pieces keep their
    // trailing newline (the final "" artifact had none).
    assert_eq!(flushed, vec!["Hello, world!\n", "\n", ""]);
    assert!(buf.tail().is_empty());
    assert!(buf.is_stable());
}

#[test]
fn buffer_holds_incomplete_code_fence() {
    let mut buf = StreamingBuffer::new();
    buf.push_delta("Some text.\n```python\nprint('hello')");
    let flushed = buf.force_flush();
    // force_flush heals the tail before returning it; tail is now empty
    assert_eq!(flushed, vec!["Some text.\n", "```python\nprint('hello')"]);
    assert!(buf.tail().is_empty());
    // Fence is still open (never closed), so buffer is not fully stable
    assert!(!buf.is_stable());
}

#[test]
fn buffer_completes_code_fence() {
    let mut buf = StreamingBuffer::new();
    buf.push_delta("Some text.\n```python\nprint('hello')\n```");
    let flushed = buf.force_flush();
    assert_eq!(
        flushed,
        vec!["Some text.\n", "```python\n", "print('hello')\n", "```"]
    );
    assert!(buf.tail().is_empty());
    assert!(buf.is_stable());
}

#[test]
fn buffer_batches_deltas() {
    let mut buf = StreamingBuffer::new();
    for i in 0..10 {
        buf.push_delta(&format!("word{} ", i));
    }
    buf.push_delta("\n\n");

    // After \n\n, content should be in stable buffer (not tail)
    // The tail should be empty after complete paragraph
    assert!(buf.is_stable(), "Buffer should be stable after \n\n");
    assert!(buf.tail().is_empty(), "Tail should be empty");

    // force_flush returns the stable content
    let flushed = buf.force_flush();
    assert!(!flushed.is_empty());
    let content = flushed.join("");
    assert!(content.contains("word0"));
    assert!(content.contains("word9"));
}

#[test]
fn buffer_reset_clears_all() {
    let mut buf = StreamingBuffer::new();
    buf.push_delta("hello\n");
    buf.reset();
    assert!(buf.tail().is_empty());
    assert!(buf.stable_len() == 0);
    assert!(buf.is_stable());
}
