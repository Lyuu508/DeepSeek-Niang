use crate::ChatFailure;

/// Incremental byte-oriented SSE parser: UTF-8 may be split across any HTTP chunks.
#[derive(Default)]
pub struct SseDecoder {
    pending: Vec<u8>,
    data: Vec<String>,
    event_size: usize,
}
impl SseDecoder {
    pub fn push(&mut self, bytes: &[u8]) -> Result<Vec<String>, ChatFailure> {
        if self.pending.len() + bytes.len() > 1_048_576 {
            return Err(ChatFailure::new("STREAM_INVALID"));
        }
        self.pending.extend_from_slice(bytes);
        let mut result = Vec::new();
        while let Some(end) = self.pending.iter().position(|b| *b == b'\n') {
            let mut line: Vec<_> = self.pending.drain(..=end).collect();
            line.pop();
            if line.last() == Some(&b'\r') {
                line.pop();
            }
            let line =
                std::str::from_utf8(&line).map_err(|_| ChatFailure::new("STREAM_INVALID"))?;
            if line.is_empty() {
                if !self.data.is_empty() {
                    result.push(self.data.join("\n"));
                    self.data.clear();
                    self.event_size = 0;
                }
            } else if let Some(value) = line.strip_prefix("data:") {
                let value = value.strip_prefix(' ').unwrap_or(value);
                self.event_size += value.len();
                if self.event_size > 262_144 {
                    return Err(ChatFailure::new("STREAM_INVALID"));
                }
                self.data.push(value.to_owned());
            }
        }
        Ok(result)
    }
}
