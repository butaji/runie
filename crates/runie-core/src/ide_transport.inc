impl IdeActor {
    /// Own one bounded async reader until EOF, reducing each complete JSON-RPC
    /// notification through the actor and publishing disconnect as an event.
    pub async fn ingest_reader<R>(&self, reader: R) -> Result<usize, String>
    where
        R: tokio::io::AsyncRead + Unpin,
    {
        use tokio::io::AsyncBufReadExt;
        let mut reader = tokio::io::BufReader::new(reader);
        let mut buffer = IdeWireBuffer::default();
        let mut line = Vec::new();
        let mut frames = 0;
        loop {
            line.clear();
            let read = reader
                .read_until(b'\n', &mut line)
                .await
                .map_err(|error| format!("IDE reader failed: {error}"))?;
            if read == 0 {
                break;
            }
            if line.len() > IDE_MAX_FRAME_BYTES {
                return Err("IDE JSON-RPC frame exceeds the bounded byte limit".into());
            }
            let text = std::str::from_utf8(&line)
                .map_err(|error| format!("IDE frame is not UTF-8: {error}"))?;
            frames += self.apply_wire_frames(&mut buffer, text).await?;
        }
        self.apply(IdeEvent::ConnectionLost).await?;
        Ok(frames)
    }
}
