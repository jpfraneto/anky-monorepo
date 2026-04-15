#[derive(Default)]
pub(crate) struct StreamRenderBuffer {
    pending: String,
}

impl StreamRenderBuffer {
    pub(crate) fn push(&mut self, chunk: &str) -> Option<String> {
        if chunk.is_empty() {
            return None;
        }
        self.pending.push_str(chunk);
        self.take_stable_prefix()
    }

    pub(crate) fn finish(&mut self) -> Option<String> {
        if self.pending.is_empty() {
            None
        } else {
            Some(std::mem::take(&mut self.pending))
        }
    }

    fn take_stable_prefix(&mut self) -> Option<String> {
        let flush_at = self
            .pending
            .char_indices()
            .filter_map(|(idx, ch)| is_stream_boundary(ch).then_some(idx + ch.len_utf8()))
            .last()
            .or_else(|| (self.pending.len() >= 48).then_some(self.pending.len()))?;

        Some(self.pending.drain(..flush_at).collect())
    }
}

fn is_stream_boundary(ch: char) -> bool {
    ch.is_whitespace()
        || matches!(
            ch,
            '.' | ','
                | '!'
                | '?'
                | ';'
                | ':'
                | ')'
                | ']'
                | '}'
                | '…'
                | '，'
                | '。'
                | '！'
                | '？'
                | '；'
                | '：'
                | '、'
        )
        || is_cjk_char(ch)
}

fn is_cjk_char(ch: char) -> bool {
    matches!(
        ch,
        '\u{3040}'..='\u{30ff}'
            | '\u{3400}'..='\u{4dbf}'
            | '\u{4e00}'..='\u{9fff}'
            | '\u{ac00}'..='\u{d7af}'
    )
}

#[cfg(test)]
mod tests {
    use super::StreamRenderBuffer;

    #[test]
    fn buffers_partial_words_until_boundary() {
        let mut buffer = StreamRenderBuffer::default();
        assert_eq!(buffer.push("hel"), None);
        assert_eq!(buffer.push("lo "), Some("hello ".to_string()));
    }

    #[test]
    fn flushes_punctuation_immediately() {
        let mut buffer = StreamRenderBuffer::default();
        assert_eq!(buffer.push("hello."), Some("hello.".to_string()));
    }

    #[test]
    fn flushes_cjk_without_waiting_for_spaces() {
        let mut buffer = StreamRenderBuffer::default();
        assert_eq!(buffer.push("你"), Some("你".to_string()));
        assert_eq!(buffer.push("好"), Some("好".to_string()));
    }

    #[test]
    fn flushes_tail_when_stream_finishes() {
        let mut buffer = StreamRenderBuffer::default();
        assert_eq!(buffer.push("unfinished"), None);
        assert_eq!(buffer.finish(), Some("unfinished".to_string()));
    }

    #[test]
    fn force_flushes_long_unbroken_segments() {
        let mut buffer = StreamRenderBuffer::default();
        let long = "a".repeat(48);
        assert_eq!(buffer.push(&long), Some(long));
    }
}
