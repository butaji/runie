//! ACP protocol framing and serialization.
//!
//! Handles message framing with 4-byte big-endian length prefix.

use anyhow::{Context, Result};
use bytes::{BufMut, BytesMut};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use super::MAX_MESSAGE_SIZE;

/// ACP protocol handler for reading/writing framed messages.
#[derive(Debug)]
pub struct AcpProtocol;

impl AcpProtocol {
    /// Read a framed message from a reader.
    ///
    /// Returns the raw JSON bytes (deserialization is caller's responsibility).
    pub async fn read_frame<R: AsyncRead + Unpin>(reader: &mut R) -> Result<BytesMut> {
        // Read 4-byte length header
        let mut header = [0u8; 4];
        reader.read_exact(&mut header).await.context("failed to read frame header")?;

        let len = u32::from_be_bytes(header) as usize;
        if len > MAX_MESSAGE_SIZE {
            anyhow::bail!("message too large: {} bytes (max: {})", len, MAX_MESSAGE_SIZE);
        }

        // Read the message body
        let mut buf = vec![0u8; len];
        reader.read_exact(&mut buf).await.context("failed to read frame body")?;
        let mut bytes = BytesMut::with_capacity(len);
        bytes.extend_from_slice(&buf);
        Ok(bytes)
    }

    /// Write a framed message to a writer.
    ///
    /// Takes raw JSON bytes and writes them with a 4-byte length prefix.
    pub async fn write_frame<W: AsyncWrite + Unpin>(writer: &mut W, data: &[u8]) -> Result<()> {
        let len = data.len() as u32;
        if len as usize > MAX_MESSAGE_SIZE {
            anyhow::bail!("message too large: {} bytes (max: {})", data.len(), MAX_MESSAGE_SIZE);
        }

        let mut buf = BytesMut::with_capacity(4 + data.len());
        buf.put_u32(len);
        buf.put_slice(data);

        writer.write_all(&buf).await.context("failed to write frame")?;
        writer.flush().await.context("failed to flush writer")?;

        Ok(())
    }

    /// Read and deserialize a message.
    pub async fn read_message<R: AsyncRead + Unpin, T: serde::de::DeserializeOwned>(
        reader: &mut R,
    ) -> Result<T> {
        let buf = Self::read_frame(reader).await?;
        let msg = serde_json::from_slice(&buf).context("failed to deserialize message")?;
        Ok(msg)
    }

    /// Serialize and write a message.
    pub async fn write_message<W: AsyncWrite + Unpin, T: serde::Serialize>(
        writer: &mut W,
        msg: &T,
    ) -> Result<()> {
        let data = serde_json::to_vec(msg).context("failed to serialize message")?;
        Self::write_frame(writer, &data).await
    }
}

/// Extension trait for framed I/O on streams.
#[allow(async_fn_in_trait)]
pub trait FramedRead {
    async fn read_message<T: serde::de::DeserializeOwned>(&mut self) -> Result<T>;
}

impl<R: AsyncRead + Unpin> FramedRead for R {
    async fn read_message<T: serde::de::DeserializeOwned>(&mut self) -> Result<T> {
        AcpProtocol::read_message(self).await
    }
}

/// Extension trait for framed I/O on writers.
#[allow(async_fn_in_trait)]
pub trait FramedWrite {
    async fn write_message<T: serde::Serialize>(&mut self, msg: &T) -> Result<()>;
}

impl<W: AsyncWrite + Unpin> FramedWrite for W {
    async fn write_message<T: serde::Serialize>(&mut self, msg: &T) -> Result<()> {
        AcpProtocol::write_message(self, msg).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::AcpMessage;

    #[tokio::test]
    async fn roundtrip_message() {
        let msg = AcpMessage::new("test", serde_json::json!({"key": "value"}));

        let mut buf = Vec::new();
        AcpProtocol::write_message(&mut buf, &msg).await.unwrap();

        let result: AcpMessage = AcpProtocol::read_message(&mut buf.as_slice()).await.unwrap();
        assert_eq!(result.method, "test");
    }

    #[tokio::test]
    async fn frame_includes_length_prefix() {
        let data = b"hello world";
        let mut buf = Vec::new();
        AcpProtocol::write_frame(&mut buf, data).await.unwrap();

        assert_eq!(buf.len(), 4 + data.len());
        let len = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]);
        assert_eq!(len as usize, data.len());
    }
}
