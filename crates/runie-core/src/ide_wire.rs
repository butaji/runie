use super::IDE_MAX_FRAME_BYTES;

/// Bounded newline-delimited JSON-RPC framing state. Socket adapters own the
/// stream; this value owns only incomplete bytes and complete messages.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct IdeWireBuffer {
    pending: String,
}

impl IdeWireBuffer {
    pub fn push(&mut self, bytes: &str) -> Result<Vec<String>, String> {
        self.pending.push_str(bytes);
        if self.pending.len() > IDE_MAX_FRAME_BYTES && !self.pending.contains('\n') {
            return Err("IDE JSON-RPC frame exceeds the bounded byte limit".into());
        }
        let mut frames = Vec::new();
        while let Some(newline) = self.pending.find('\n') {
            let frame = self.pending[..newline].trim_end_matches('\r').to_owned();
            self.pending.drain(..=newline);
            if frame.len() > IDE_MAX_FRAME_BYTES {
                return Err("IDE JSON-RPC frame exceeds the bounded byte limit".into());
            }
            if !frame.trim().is_empty() {
                frames.push(frame);
            }
        }
        Ok(frames)
    }

    pub fn pending_bytes(&self) -> usize {
        self.pending.len()
    }
}
