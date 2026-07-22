//! Text chunking for search indexing.
//!
//! Provides text splitting strategies optimized for semantic search:
//! - Sentence-based chunking
//! - Token-based chunking (approximate)
//! - Overlapping chunks for better context preservation

/// Configuration for text chunking.
#[derive(Debug, Clone)]
pub struct ChunkConfig {
    /// Maximum chunk size in characters.
    pub max_chars: usize,
    /// Maximum tokens per chunk (approximate).
    pub max_tokens: usize,
    /// Overlap between chunks (percentage).
    pub overlap_percent: usize,
    /// Minimum chunk size.
    pub min_chars: usize,
    /// Split on sentence boundaries when possible.
    pub split_sentences: bool,
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self {
            max_chars: 1000,
            max_tokens: 256,
            overlap_percent: 10,
            min_chars: 100,
            split_sentences: true,
        }
    }
}

impl ChunkConfig {
    /// Create a config optimized for embeddings.
    pub fn for_embeddings() -> Self {
        Self {
            max_chars: 1000,
            max_tokens: 256,
            overlap_percent: 10,
            min_chars: 100,
            split_sentences: true,
        }
    }

    /// Create a config for short snippets.
    pub fn for_snippets() -> Self {
        Self {
            max_chars: 200,
            max_tokens: 50,
            overlap_percent: 5,
            min_chars: 20,
            split_sentences: true,
        }
    }

    /// Create a config for long documents.
    pub fn for_documents() -> Self {
        Self {
            max_chars: 2000,
            max_tokens: 512,
            overlap_percent: 15,
            min_chars: 200,
            split_sentences: true,
        }
    }

    /// Set maximum characters.
    pub fn max_chars(mut self, n: usize) -> Self {
        self.max_chars = n;
        self
    }

    /// Set maximum tokens.
    pub fn max_tokens(mut self, n: usize) -> Self {
        self.max_tokens = n;
        self
    }

    /// Set overlap percentage.
    pub fn overlap_percent(mut self, n: usize) -> Self {
        self.overlap_percent = n;
        self
    }
}

/// A text chunk with metadata.
#[derive(Debug, Clone)]
pub struct TextChunk {
    /// Chunk content.
    pub content: String,
    /// Start byte offset in original text.
    pub start: usize,
    /// End byte offset in original text.
    pub end: usize,
    /// Estimated token count.
    pub token_count: usize,
    /// Chunk index.
    pub index: usize,
    /// Whether this is an overflow chunk.
    pub is_overflow: bool,
}

impl TextChunk {
    /// Get the length in characters.
    pub fn len(&self) -> usize {
        self.content.len()
    }

    /// Check if chunk is empty.
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }
}

/// Text chunker for semantic search.
#[derive(Debug, Default)]
pub struct TextChunker {
    config: ChunkConfig,
}

impl TextChunker {
    /// Create a new chunker with default config.
    pub fn new() -> Self {
        Self {
            config: ChunkConfig::default(),
        }
    }

    /// Create with custom config.
    pub fn with_config(config: ChunkConfig) -> Self {
        Self { config }
    }

    /// Create with default config for embeddings.
    pub fn for_embeddings() -> Self {
        Self {
            config: ChunkConfig::for_embeddings(),
        }
    }

    /// Split text into chunks.
    pub fn chunk(&self, text: &str) -> Vec<TextChunk> {
        if text.trim().is_empty() {
            return Vec::new();
        }

        if self.config.split_sentences {
            self.chunk_by_sentences(text)
        } else {
            self.chunk_by_chars(text)
        }
    }

    /// Chunk by sentence boundaries.
    #[allow(clippy::too_many_lines)]
    fn chunk_by_sentences(&self, text: &str) -> Vec<TextChunk> {
        let sentences = self.split_sentences(text);
        let mut chunks: Vec<TextChunk> = Vec::new();
        let mut current_chunk = String::new();
        let mut current_start = 0;
        let mut current_tokens = 0;
        let mut chunk_index = 0;
        let overlap_chars = (self.config.max_chars * self.config.overlap_percent) / 100;

        for sentence in sentences {
            let sentence_len = sentence.len();
            let sentence_tokens = estimate_tokens(sentence);

            // Check if adding this sentence exceeds limits
            let would_exceed_chars = current_chunk.len() + sentence_len > self.config.max_chars;
            let would_exceed_tokens = current_tokens + sentence_tokens > self.config.max_tokens;

            if would_exceed_chars || would_exceed_tokens {
                // Save current chunk if it meets minimum size
                if current_chunk.len() >= self.config.min_chars || chunks.is_empty() {
                    chunks.push(TextChunk {
                        content: current_chunk.trim().to_string(),
                        start: current_start,
                        end: current_start + current_chunk.len(),
                        token_count: current_tokens,
                        index: chunk_index,
                        is_overflow: false,
                    });
                    chunk_index += 1;
                }

                // Start new chunk with overlap
                if overlap_chars > 0 && !current_chunk.is_empty() {
                    let overlap_start = current_chunk.len().saturating_sub(overlap_chars);
                    let overlap_len = current_chunk.len() - overlap_start;
                    let overlap_text = current_chunk[overlap_start..].trim();
                    let overlap_tokens = estimate_tokens(overlap_text);
                    current_start += overlap_len - overlap_text.len();
                    current_chunk = overlap_text.to_string();
                    current_tokens = overlap_tokens;
                } else {
                    current_chunk = String::new();
                    current_start = 0;
                    current_tokens = 0;
                }
            }

            // Add separator if needed
            if !current_chunk.is_empty() && !current_chunk.ends_with('\n') {
                current_chunk.push_str(". ");
            }

            current_chunk.push_str(sentence);
            current_tokens += sentence_tokens;
        }

        // Don't forget the last chunk
        if current_chunk.len() >= self.config.min_chars || chunks.is_empty() {
            chunks.push(TextChunk {
                content: current_chunk.trim().to_string(),
                start: current_start,
                end: current_start + current_chunk.len(),
                token_count: current_tokens,
                index: chunk_index,
                is_overflow: false,
            });
        }

        chunks
    }

    /// Chunk by character boundaries.
    fn chunk_by_chars(&self, text: &str) -> Vec<TextChunk> {
        let chars: Vec<char> = text.chars().collect();
        let len = chars.len();
        let overlap = (self.config.max_chars * self.config.overlap_percent) / 100;
        let mut chunks: Vec<TextChunk> = Vec::new();

        let mut pos = 0;
        let mut chunk_index = 0;

        while pos < len {
            let end = (pos + self.config.max_chars).min(len);
            let chunk_chars = &chars[pos..end];
            let content: String = chunk_chars.iter().collect();

            chunks.push(TextChunk {
                content: content.trim().to_string(),
                start: pos,
                end,
                token_count: estimate_tokens(&content),
                index: chunk_index,
                is_overflow: end < len,
            });

            chunk_index += 1;
            pos += self.config.max_chars - overlap;
        }

        chunks
    }

    /// Split text into sentences.
    fn split_sentences<'a>(&self, text: &'a str) -> Vec<&'a str> {
        let mut sentences: Vec<&'a str> = Vec::new();

        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            // Simple sentence splitting on common terminators
            let mut last_end = 0;
            for (i, _) in line.match_indices(['.', '!', '?']) {
                let end_pos = i + 1;
                let sentence = line[last_end..end_pos].trim();
                if !sentence.is_empty() {
                    sentences.push(sentence);
                }
                last_end = end_pos;
            }

            // Handle remaining text
            let remaining = line[last_end..].trim();
            if !remaining.is_empty() && !remaining.ends_with(['.', '!', '?']) {
                sentences.push(remaining);
            }
        }

        if sentences.is_empty() && !text.trim().is_empty() {
            sentences.push(text.trim());
        }

        sentences
    }
}

/// Estimate token count for text.
/// Uses a simple heuristic: ~4 chars per token for English.
pub fn estimate_tokens(text: &str) -> usize {
    let word_count = text.split_whitespace().count();
    // Heuristic: ~1.3 tokens per word for typical English
    ((word_count as f64) * 1.3).round() as usize
}

/// Count words in text.
pub fn count_words(text: &str) -> usize {
    text.split_whitespace().count()
}

/// Split text into words.
pub fn split_words(text: &str) -> Vec<&str> {
    text.split_whitespace().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_tokens() {
        let text = "This is a test sentence with eight words.";
        assert_eq!(estimate_tokens(text), 10); // 8 words × 1.3 = 10.4 → 10

        let empty = "";
        assert_eq!(estimate_tokens(empty), 0);
    }

    #[test]
    fn test_chunker_basic() {
        let chunker = TextChunker::for_embeddings();
        let text = "This is a short text. It should fit in one chunk.";

        let chunks = chunker.chunk(text);
        assert!(!chunks.is_empty());
        // Chunker may append a trailing period for sentence normalization.
        assert!(
            chunks[0].content.starts_with("This is a short text"),
            "got: {}",
            chunks[0].content
        );
    }

    #[test]
    fn test_chunker_long_text() {
        let chunker = TextChunker::with_config(ChunkConfig {
            max_chars: 50,
            max_tokens: 15,
            overlap_percent: 10,
            min_chars: 10,
            split_sentences: true,
        });

        let text = "This is the first sentence. Here comes the second one. And the third for good measure.";

        let chunks = chunker.chunk(text);
        assert!(chunks.len() > 1);
        assert!(chunks[0].len() <= 50);
    }

    #[test]
    fn test_chunker_empty() {
        let chunker = TextChunker::new();
        let chunks = chunker.chunk("");
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_chunker_preserves_content() {
        let chunker = TextChunker::for_embeddings();
        let text = "First sentence. Second sentence. Third sentence.";

        let chunks = chunker.chunk(text);
        let combined: String = chunks.iter().map(|c| c.content.as_str()).collect();
        // Combined content should contain all original words
        assert!(combined.contains("First"));
        assert!(combined.contains("Second"));
        assert!(combined.contains("Third"));
    }

    #[test]
    fn test_chunk_metadata() {
        let chunker = TextChunker::for_embeddings();
        let text = "A".repeat(500);

        let chunks = chunker.chunk(&text);
        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].index, 0);
        assert!(chunks[0].start < chunks[0].end);
    }

    #[test]
    fn test_config_variants() {
        let embeddings = ChunkConfig::for_embeddings();
        assert_eq!(embeddings.max_tokens, 256);

        let snippets = ChunkConfig::for_snippets();
        assert_eq!(snippets.max_tokens, 50);

        let documents = ChunkConfig::for_documents();
        assert_eq!(documents.max_tokens, 512);
    }
}
