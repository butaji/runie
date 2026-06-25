//! SSE (Server-Sent Events) framing utilities.
//!
//! Splits a byte stream on newlines, strips `data: ` prefixes,
//! drops `[DONE]` markers, and yields JSON strings.

use bytes::Bytes;
use futures::Stream;
use std::pin::Pin;

/// Wraps a byte stream and yields SSE-formatted JSON strings.
pub fn sse_framing<E>(
    bytes: impl Stream<Item = Result<Bytes, E>> + Send + 'static,
) -> impl Stream<Item = Result<String, String>> + Send
where
    E: std::fmt::Display + std::marker::Send,
{
    let stream = Box::pin(bytes);
    FramingStream {
        inner: stream,
        buffer: String::new(),
    }
}

struct FramingStream<E> {
    inner: Pin<Box<dyn Stream<Item = Result<Bytes, E>> + Send>>,
    buffer: String,
}

impl<E: std::fmt::Display> Stream for FramingStream<E> {
    type Item = Result<String, String>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        use std::task::Poll;

        loop {
            match Pin::as_mut(&mut self.inner).poll_next(cx) {
                Poll::Ready(Some(Ok(chunk))) => {
                    self.buffer.push_str(&String::from_utf8_lossy(&chunk));
                }
                Poll::Ready(Some(Err(e))) => {
                    return Poll::Ready(Some(Err(e.to_string())));
                }
                Poll::Ready(None) => {
                    if let Some(line) = drain_buffer(&mut self.buffer) {
                        return Poll::Ready(Some(Ok(line)));
                    }
                    return Poll::Ready(None);
                }
                Poll::Pending => return Poll::Pending,
            }

            if let Some(line) = drain_buffer(&mut self.buffer) {
                return Poll::Ready(Some(Ok(line)));
            }
        }
    }
}

fn drain_buffer(buffer: &mut String) -> Option<String> {
    while let Some(pos) = buffer.find('\n') {
        let line = buffer[..pos].trim().to_string();
        *buffer = buffer[pos + 1..].to_string();

        let data = line.strip_prefix("data: ")?;
        if data == "[DONE]" {
            continue;
        }

        return Some(data.to_string());
    }
    None
}

/// Parse a raw SSE data line into a JSON string.
pub fn parse_sse_line(line: &str) -> Option<String> {
    let data = line.strip_prefix("data: ")?;
    if data == "[DONE]" {
        return None;
    }
    Some(data.to_string())
}

/// Parse an SSE event line (data: {...} or data: [DONE]).
#[derive(Debug, Clone, PartialEq)]
pub enum SseLine {
    Data(String),
    Done,
}

impl SseLine {
    pub fn parse(line: &str) -> Option<Self> {
        let data = line.strip_prefix("data: ")?;
        if data == "[DONE]" {
            Some(SseLine::Done)
        } else {
            Some(SseLine::Data(data.to_string()))
        }
    }

    pub fn is_data(&self) -> bool {
        matches!(self, SseLine::Data(_))
    }
    pub fn is_done(&self) -> bool {
        matches!(self, SseLine::Done)
    }
    pub fn into_data(self) -> Option<String> {
        match self {
            SseLine::Data(s) => Some(s),
            SseLine::Done => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::stream;
    use futures::StreamExt;

    fn bytes_stream<E: std::fmt::Display + std::marker::Send>(
        data: &[&[u8]],
    ) -> impl Stream<Item = Result<Bytes, E>> + Send {
        let stream_data: Vec<Result<Bytes, E>> =
            data.iter().map(|b| Ok(Bytes::copy_from_slice(b))).collect();
        stream::iter(stream_data)
    }

    #[tokio::test]
    async fn sse_framing_splits_on_newline() {
        let input = bytes_stream::<std::io::Error>(&[b"data: {\"a\":1}\ndata: [DONE]\n"]);
        let framing = sse_framing(input);
        let items: Vec<String> = framing.filter_map(|r| async { r.ok() }).collect().await;
        assert_eq!(items, vec![r#"{"a":1}"#]);
    }

    #[tokio::test]
    async fn sse_framing_handles_partial_chunks() {
        let input = bytes_stream::<std::io::Error>(&[b"data: {\"a\":", b"1}\n"]);
        let framing = sse_framing(input);
        let items: Vec<String> = framing.filter_map(|r| async { r.ok() }).collect().await;
        assert_eq!(items, vec![r#"{"a":1}"#]);
    }

    #[tokio::test]
    async fn sse_framing_handles_multiple_events() {
        let input =
            bytes_stream::<std::io::Error>(&[b"data: {\"x\":1}\ndata: {\"y\":2}\ndata: [DONE]\n"]);
        let framing = sse_framing(input);
        let items: Vec<String> = framing.filter_map(|r| async { r.ok() }).collect().await;
        assert_eq!(items, vec![r#"{"x":1}"#, r#"{"y":2}"#]);
    }

    #[tokio::test]
    async fn sse_framing_skips_empty_lines() {
        let input = bytes_stream::<std::io::Error>(&[b"\ndata: {\"a\":1}\n\ndata: [DONE]\n"]);
        let framing = sse_framing(input);
        let items: Vec<String> = framing.filter_map(|r| async { r.ok() }).collect().await;
        assert_eq!(items, vec![r#"{"a":1}"#]);
    }

    #[test]
    fn parse_sse_line_text() {
        assert_eq!(
            parse_sse_line("data: {\"x\":1}"),
            Some(r#"{"x":1}"#.to_string())
        );
    }

    #[test]
    fn parse_sse_line_done() {
        assert_eq!(parse_sse_line("data: [DONE]"), None);
    }

    #[test]
    fn parse_sse_line_no_prefix() {
        assert_eq!(parse_sse_line("not data: {\"x\":1}"), None);
    }

    #[test]
    fn sse_line_parse_data() {
        assert_eq!(
            SseLine::parse("data: {\"x\":1}"),
            Some(SseLine::Data(r#"{"x":1}"#.to_string()))
        );
    }

    #[test]
    fn sse_line_parse_done() {
        assert_eq!(SseLine::parse("data: [DONE]"), Some(SseLine::Done));
    }

    #[test]
    fn sse_line_into_data() {
        assert_eq!(
            SseLine::Data(r#"{"x":1}"#.to_string()).into_data(),
            Some(r#"{"x":1}"#.to_string())
        );
        assert_eq!(SseLine::Done.into_data(), None);
    }
}
