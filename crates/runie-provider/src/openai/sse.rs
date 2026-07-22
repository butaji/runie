//! SSE (Server-Sent Events) byte-buffer parser.
//!
//! Handles SSE frame boundaries: `\n` (LF) and `\r\n\r\n` (CRLF double-newline).
//! Accumulates incoming bytes into a buffer and emits complete frames.

use bytes::Buf;
use bytes::BytesMut;

/// SSE event types emitted by the parser.
#[derive(Debug, Clone, PartialEq)]
pub enum SseEvent {
    /// A data field value from the SSE frame.
    Data(String),
    /// An error field value from the SSE frame.
    Error(String),
    /// Terminal `[DONE]` frame.
    Done,
}

/// Errors that can occur while parsing SSE data.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum SseError {
    #[error("invalid UTF-8 in SSE stream")]
    InvalidUtf8,
}

/// SSE frame parser operating on a byte buffer.
///
/// Accumulates bytes into an internal string buffer and splits on SSE frame
/// boundaries (`\n\n` or `\r\n\r\n`). Emits complete frames as `SseEvent`.
#[derive(Debug, Default)]
pub struct SseParser {
    /// Accumulated bytes not yet forming a complete frame.
    buf: String,
}

impl SseParser {
    /// Create a new SSE parser.
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse the next complete SSE event from the buffer.
    ///
    /// Appends incoming bytes to the internal buffer, then extracts and emits
    /// the first complete frame found. SSE frames are separated by `\n\n` or
    /// `\r\n\r\n` (double-newline boundary).
    ///
    /// Returns `None` if no complete frame is available yet.
    pub fn next_event(&mut self, buf: &mut BytesMut) -> Option<Result<SseEvent, SseError>> {
        // Append incoming bytes to the accumulator.
        match std::str::from_utf8(buf.as_ref()) {
            Ok(s) => self.buf.push_str(s),
            Err(e) => {
                buf.advance(e.valid_up_to());
                return Some(Err(SseError::InvalidUtf8));
            }
        }
        buf.clear();

        // SSE frames are separated by \r\n\r\n or \n\n (double-newline boundary).
        // Check for CRLF double-newline first.
        if let Some(pos) = self.buf.find("\r\n\r\n") {
            let frame = self.buf[..pos].to_string();
            self.buf = self.buf[pos + 4..].to_string();
            return Some(self.parse_frame(&frame));
        }

        // Check for LF double-newline boundary.
        if let Some(pos) = self.buf.find("\n\n") {
            let frame = self.buf[..pos].to_string();
            self.buf = self.buf[pos + 2..].to_string();
            return Some(self.parse_frame(&frame));
        }

        // No complete frame yet (waiting for the terminating double-newline).
        None
    }

    /// Parse a single SSE frame string into an event.
    ///
    /// A frame may contain multiple field lines (e.g. several `data:` lines).
    /// All `data:` lines in the frame are joined with `\n` into a single
    /// `SseEvent::Data`. A joined value of exactly `[DONE]` is emitted as
    /// `SseEvent::Done`.
    fn parse_frame(&self, frame: &str) -> Result<SseEvent, SseError> {
        let mut data_lines: Vec<String> = Vec::new();
        let mut error: Option<String> = None;

        for line in frame.lines() {
            // SSE comments start with ':'; ignore empty lines too.
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with(':') {
                continue;
            }

            // Handle "error: ..." prefix first (fixture error format).
            if let Some(err) = line.strip_prefix("error: ") {
                error.get_or_insert(err.trim().to_string());
                continue;
            }
            if line.starts_with("error:") {
                let err = line.strip_prefix("error:").unwrap().trim_start();
                error.get_or_insert(err.to_string());
                continue;
            }

            // Handle "data: ..." with an optional space after the colon.
            if let Some(data) = line.strip_prefix("data: ") {
                data_lines.push(data.trim().to_string());
                continue;
            }
            if line.starts_with("data:") {
                let data = line.strip_prefix("data:").unwrap().trim_start();
                data_lines.push(data.to_string());
                continue;
            }
        }

        if let Some(err) = error {
            return Ok(SseEvent::Error(err));
        }

        if data_lines.is_empty() {
            // Empty frame or frame with only comments/empty lines.
            return Ok(SseEvent::Data(String::new()));
        }

        let value = data_lines.join("\n");
        if value == "[DONE]" {
            Ok(SseEvent::Done)
        } else {
            Ok(SseEvent::Data(value))
        }
    }

    /// Returns any remaining buffered data not yet emitted as a frame.
    pub fn remaining(&self) -> &str {
        &self.buf
    }

    /// Clear the internal buffer.
    pub fn clear(&mut self) {
        self.buf.clear();
    }
}

/// Append more raw bytes to the parser and extract any complete events.
pub fn parse_events(parser: &mut SseParser, input: &mut BytesMut) -> Vec<Result<SseEvent, SseError>> {
    let mut events = Vec::new();
    while let Some(event) = parser.next_event(input) {
        events.push(event);
    }
    events
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_single_data_frame_lf() {
        let mut parser = SseParser::new();
        let mut buf = BytesMut::from("data: hello\n\n");

        let event = parser.next_event(&mut buf);
        assert!(matches!(event, Some(Ok(SseEvent::Data(s))) if s == "hello"));
        assert!(parser.remaining().is_empty());
    }

    #[test]
    fn parses_single_data_frame_crlf() {
        let mut parser = SseParser::new();
        let mut buf = BytesMut::from("data: hello\r\n\r\n");

        let event = parser.next_event(&mut buf);
        assert!(matches!(event, Some(Ok(SseEvent::Data(s))) if s == "hello"));
    }

    #[test]
    fn parses_done_frame() {
        let mut parser = SseParser::new();
        let mut buf = BytesMut::from("data: [DONE]\n\n");

        let event = parser.next_event(&mut buf);
        assert!(matches!(event, Some(Ok(SseEvent::Done))));
    }

    #[test]
    fn parses_error_frame() {
        let mut parser = SseParser::new();
        let mut buf = BytesMut::from("error: rate limit\n\n");

        let event = parser.next_event(&mut buf);
        assert!(matches!(event, Some(Ok(SseEvent::Error(s))) if s == "rate limit"));
    }

    #[test]
    fn accumulates_partial_frames() {
        let mut parser = SseParser::new();
        let mut buf = BytesMut::from("data: first");

        // No complete frame yet (missing trailing \n\n).
        assert!(parser.next_event(&mut buf).is_none());
        assert_eq!(parser.remaining(), "data: first");

        // Complete the first frame and add a second frame.
        buf.extend_from_slice(b"\n\ndata: second\n\n");
        let events: Vec<_> = parse_events(&mut parser, &mut buf);
        assert_eq!(events.len(), 2);
        assert!(matches!(&events[0], Ok(SseEvent::Data(s)) if s == "first"));
        assert!(matches!(&events[1], Ok(SseEvent::Data(s)) if s == "second"));
    }

    #[test]
    fn parses_multiple_frames_in_one_buffer() {
        let mut parser = SseParser::new();
        // SSE format: each event line ends with \n, separated by \n\n.
        let mut buf = BytesMut::from("data: one\n\ndata: two\n\n");

        let events: Vec<_> = parse_events(&mut parser, &mut buf);
        assert_eq!(events.len(), 2);
        assert!(matches!(&events[0], Ok(SseEvent::Data(s)) if s == "one"));
        assert!(matches!(&events[1], Ok(SseEvent::Data(s)) if s == "two"));
    }

    #[test]
    fn parses_multiple_data_lines_in_one_frame() {
        let mut parser = SseParser::new();
        let mut buf = BytesMut::from("data: one\ndata: two\n\n");

        let event = parser.next_event(&mut buf);
        assert!(matches!(event, Some(Ok(SseEvent::Data(s))) if s == "one\ntwo"));
    }

    #[test]
    fn handles_json_data_frames() {
        let mut parser = SseParser::new();
        let json = r#"data: {"content":"hello"}"#;
        let mut buf = BytesMut::from(json);
        buf.extend_from_slice(b"\n\n");

        let event = parser.next_event(&mut buf);
        assert!(matches!(
            event,
            Some(Ok(SseEvent::Data(s))) if s.contains("hello")
        ));
    }

    #[test]
    fn handles_empty_data_line() {
        let mut parser = SseParser::new();
        let mut buf = BytesMut::from("\n\n");

        let event = parser.next_event(&mut buf);
        // Empty lines emit Data("") for SSE compatibility.
        assert!(matches!(event, Some(Ok(SseEvent::Data(s))) if s.is_empty()));
    }

    #[test]
    fn parser_clear_resets_buffer() {
        let mut parser = SseParser::new();
        let mut buf = BytesMut::from("data: partial");
        parser.next_event(&mut buf); // Will be None, data accumulated
        assert_eq!(parser.remaining(), "data: partial");

        parser.clear();
        assert!(parser.remaining().is_empty());
    }

    #[test]
    fn remaining_returns_accumulated_data() {
        let mut parser = SseParser::new();
        let mut buf = BytesMut::from("incomplete");

        assert!(parser.next_event(&mut buf).is_none());
        assert_eq!(parser.remaining(), "incomplete");
    }

    #[test]
    fn parse_events_returns_all_complete_frames() {
        let mut parser = SseParser::new();
        let mut buf = BytesMut::from("data: a\n\ndata: b\n\ndata: [DONE]\n\n");

        let events = parse_events(&mut parser, &mut buf);
        assert_eq!(events.len(), 3);
        assert!(matches!(&events[0], Ok(SseEvent::Data(s)) if s == "a"));
        assert!(matches!(&events[1], Ok(SseEvent::Data(s)) if s == "b"));
        assert!(matches!(&events[2], Ok(SseEvent::Done)));
    }

    #[test]
    fn data_prefix_without_space() {
        let mut parser = SseParser::new();
        let mut buf = BytesMut::from("data:hello\n\n");

        let event = parser.next_event(&mut buf);
        assert!(matches!(event, Some(Ok(SseEvent::Data(s))) if s == "hello"));
    }

    #[test]
    fn error_prefix_without_space() {
        let mut parser = SseParser::new();
        let mut buf = BytesMut::from("error:rate_limit\n\n");

        let event = parser.next_event(&mut buf);
        assert!(matches!(event, Some(Ok(SseEvent::Error(s))) if s == "rate_limit"));
    }

    #[test]
    fn mixed_crlf_and_lf_boundaries() {
        let mut parser = SseParser::new();
        // First frame uses CRLF, second uses LF.
        let mut buf = BytesMut::from("data: first\r\n\r\ndata: second\n\n");

        let events: Vec<_> = parse_events(&mut parser, &mut buf);
        assert_eq!(events.len(), 2);
        assert!(matches!(&events[0], Ok(SseEvent::Data(s)) if s == "first"));
        assert!(matches!(&events[1], Ok(SseEvent::Data(s)) if s == "second"));
    }
}
