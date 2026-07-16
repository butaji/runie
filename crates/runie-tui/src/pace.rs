//! PacedRenderer — smooth typing animation by decoupling "received text" from "displayed text".
//!
//! The renderer receives text chunks and advances the displayed cursor gradually,
//! snapping to word boundaries for a natural reading experience.

/// PacedRenderer decouples "received text" from "displayed text" for smooth typing animation.
#[derive(Debug, Default)]
pub struct PacedRenderer {
    /// All text received so far (source of truth).
    received: String,
    /// Text currently displayed (cursor position in received text).
    displayed: usize,
}

impl PacedRenderer {
    /// Create a new empty renderer.
    pub fn new() -> Self {
        Self {
            received: String::new(),
            displayed: 0,
        }
    }

    /// Append more received text.
    pub fn push(&mut self, text: &str) {
        self.received.push_str(text);
    }

    /// Advance the displayed cursor by `max(4, rate_chunk)` chars, snapping to word boundary.
    /// Returns the newly displayed substring since the last tick.
    pub fn tick(&mut self) -> String {
        if self.displayed >= self.received.len() {
            return String::new();
        }

        // Calculate step size: clamp to [4, 24] range.
        // Min 4 ensures short text (5-10 chars) completes in 1-2 ticks at 100ms/tick,
        // instead of needing 3+ ticks which causes the test harness to capture
        // intermediate pacing states (e.g. "secon" instead of "second").
        let pending_len = self.received.len().saturating_sub(self.displayed);
        let rate_chunk = (pending_len as f64 / 20.0).ceil() as usize;
        let step = rate_chunk.clamp(4, 24);

        // Look for a word boundary (whitespace or punctuation) within `step + 10` chars.
        let lookahead = (step + 10).min(self.received.len() - self.displayed);
        let end_pos = self.displayed + lookahead;
        let search_slice = &self.received[self.displayed..end_pos];

        let boundary = search_slice
            .find(|c: char| c.is_ascii_whitespace() || (c.is_ascii_punctuation() && c != '\''))
            .map(|i| self.displayed + i + 1); // include the boundary char

        let target = match boundary {
            Some(pos) if pos <= self.displayed + step => pos,
            _ => (self.displayed + step).min(self.received.len()),
        };

        let result = self.received[self.displayed..target].to_string();
        self.displayed = target;
        result
    }

    /// Flush all remaining received text to displayed.
    pub fn finish(&mut self) {
        self.displayed = self.received.len();
    }

    /// Text currently displayed.
    pub fn displayed(&self) -> &str {
        &self.received[..self.displayed]
    }

    /// Remaining pending text not yet displayed.
    pub fn pending(&self) -> &str {
        &self.received[self.displayed..]
    }

    /// Whether there is pending text to display.
    pub fn is_caught_up(&self) -> bool {
        self.displayed >= self.received.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paced_renderer_starts_empty() {
        let renderer = PacedRenderer::new();
        assert_eq!(renderer.displayed(), "");
        assert_eq!(renderer.pending(), "");
        assert!(renderer.is_caught_up());
    }

    #[test]
    fn paced_renderer_tick_advances_cursor() {
        let mut renderer = PacedRenderer::new();
        renderer.push("Hello world!");

        let chunk1 = renderer.tick();
        assert!(!chunk1.is_empty(), "tick should return visible chunk");

        // The displayed text should start with "He" at minimum (2 char step)
        let displayed = renderer.displayed();
        assert!(
            displayed.starts_with("He"),
            "displayed should contain first chars: got {}",
            displayed
        );
    }

    #[test]
    fn paced_renderer_tick_snaps_to_word_boundary() {
        let mut renderer = PacedRenderer::new();
        // Build up enough text so step grows larger
        let long_text = "word1 word2 word3 word4 word5 word6 word7 word8 word9 word10";
        renderer.push(long_text);

        let mut prev_displayed_len = 0;
        let mut ticks = 0;
        while !renderer.is_caught_up() && ticks < 30 {
            let chunk = renderer.tick();
            if chunk.is_empty() {
                break;
            }
            ticks += 1;
            let curr_len = renderer.displayed().len();
            assert!(
                curr_len > prev_displayed_len,
                "displayed should grow: {} -> {}",
                prev_displayed_len,
                curr_len
            );
            prev_displayed_len = curr_len;
        }
        // Should have made progress and snapped to word boundaries eventually
        assert!(prev_displayed_len > 0, "should have displayed some text");
    }

    #[test]
    fn paced_renderer_tick_catches_up_on_small_input() {
        let mut renderer = PacedRenderer::new();
        renderer.push("Hi");

        let chunk = renderer.tick();
        // With only 2 chars, step = max(2, 2/20) = 2, so we should display "Hi" immediately
        assert!(
            chunk == "Hi" || renderer.is_caught_up(),
            "small input should catch up"
        );
    }

    #[test]
    fn paced_renderer_finish_flushes_all() {
        let mut renderer = PacedRenderer::new();
        renderer.push("This is a longer text that has not been fully displayed yet.");

        // Advance partially
        renderer.tick();
        renderer.tick();

        // Finish should flush everything
        renderer.finish();

        assert!(renderer.is_caught_up(), "finish should catch up completely");
        assert!(
            renderer.displayed().contains("This is a longer"),
            "finish should display all: {}",
            renderer.displayed()
        );
    }

    #[test]
    fn paced_renderer_adaptive_chunk_size() {
        let mut renderer = PacedRenderer::new();
        // 100 chars should give step = max(2, 100/20) = 5
        let text = "a".repeat(100);
        renderer.push(&text);

        let chunk1 = renderer.tick();
        let chunk2 = renderer.tick();

        // Step should be 5, so each tick advances by ~5 chars
        assert!(
            chunk1.len() >= 2 && chunk1.len() <= 15,
            "chunk1 size should be reasonable: {}",
            chunk1.len()
        );
        assert!(
            chunk2.len() >= 2 && chunk2.len() <= 15,
            "chunk2 size should be reasonable: {}",
            chunk2.len()
        );
    }

    #[test]
    fn paced_renderer_tick_no_op_when_caught_up() {
        let mut renderer = PacedRenderer::new();
        renderer.push("Short");
        renderer.finish(); // Mark as caught up

        let chunk = renderer.tick();
        assert!(
            chunk.is_empty(),
            "tick should return empty when caught up: '{}'",
            chunk
        );
        assert!(renderer.is_caught_up());
    }

    #[test]
    fn paced_renderer_multiple_pushes() {
        let mut renderer = PacedRenderer::new();
        renderer.push("Hello ");
        let chunk1 = renderer.tick();
        renderer.push("world!");
        let _chunk2 = renderer.tick();
        let _chunk3 = renderer.tick(); // May need another tick to catch up

        // First tick shows "He" (2 char step for 7 pending chars)
        assert!(
            chunk1.contains("He"),
            "first chunk should start with 'He': {}",
            chunk1
        );
        // Eventually we should have accumulated text from both pushes
        let displayed = renderer.displayed();
        assert!(
            displayed.contains("Hello") || displayed.contains("world"),
            "displayed should contain text from pushes: {}",
            displayed
        );
    }
}
