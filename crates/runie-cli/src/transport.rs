//! Shared JSON-RPC transport for CLI interfaces.
//!
//! This module provides common helpers for reading JSON-RPC messages from
//! stdin/IO and writing responses back.

use anyhow::Result;
use serde_json::Value;
use tokio::io::AsyncWriteExt;

use runie_core::proto::{Error, Message, Request, Response};

/// Write a JSON-RPC message to a writer.
pub async fn write_message<W>(writer: &mut W, msg: &Message) -> Result<()>
where
    W: tokio::io::AsyncWrite + Unpin,
{
    let json_str = serde_json::to_string(msg)?;
    writer.write_all(json_str.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    writer.flush().await?;
    Ok(())
}

/// Parse a JSON line into a Request, returning an error message on failure.
pub fn parse_request(line: &str) -> Result<Request, (Option<Value>, Error)> {
    let line = line.trim();
    if line.is_empty() {
        return Err((None, Error::parse("Empty line"))); // converts to anyhow::Error via From
    }

    match serde_json::from_str::<Request>(line) {
        Ok(r) => Ok(r),
        Err(e) => Err((
            serde_json::from_str(line).ok(),
            Error::parse(format!("Parse error: {e}")),
        )),
    }
}

/// Build a JSON-RPC response from a result.
pub fn build_response(id: Option<Value>, result: Result<Value>) -> Message {
    match result {
        Ok(value) => Message::Response(Response::ok(id, value)),
        Err(e) => Message::error(id, Error::internal(format!("{e}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_request_parses_valid_json() {
        let json_str = r#"{"kind":"request","id":1,"method":"initialize","params":{}}"#;
        let req = parse_request(json_str).unwrap();
        assert_eq!(req.method, "initialize");
    }

    #[test]
    fn parse_request_returns_error_on_invalid() {
        let json_str = "not json";
        let result = parse_request(json_str);
        assert!(result.is_err());
    }

    #[test]
    fn build_response_ok() {
        let msg = build_response(Some(serde_json::json!(1)), Ok(serde_json::json!({"result": "ok"})));
        match msg {
            Message::Response(resp) => {
                assert!(resp.error.is_none());
            }
            _ => panic!("expected response"),
        }
    }

    #[test]
    fn build_response_error() {
        let msg = build_response(Some(serde_json::json!(1)), Err(anyhow::anyhow!("test error")));
        match msg {
            Message::Response(resp) => {
                assert!(resp.error.is_some());
            }
            _ => panic!("expected response"),
        }
    }
}
