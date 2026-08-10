//! Cadence-owned viewport flushing for terminal scroll input.

use super::{ScrollDirection, ScrollNormalizer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrollFlush {
    pub lines: i32,
    pub backlog: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrollFinalize {
    pub flushed: i32,
    pub dropped: i32,
    pub backlog: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrollFlushState {
    normalizer: ScrollNormalizer,
    viewport_rows: u16,
    backlog: i32,
    last_flush_ms: Option<u64>,
}

pub const DEFAULT_SCROLL_FLUSH_CADENCE_MS: u64 = 16;
pub const MIN_SCROLL_FLUSH_LINES: i32 = 6;

impl ScrollFlushState {
    pub const fn new(normalizer: ScrollNormalizer, viewport_rows: u16) -> Self {
        Self {
            normalizer,
            viewport_rows,
            backlog: 0,
            last_flush_ms: None,
        }
    }

    pub const fn input_at(mut self, at_ms: u64, direction: ScrollDirection) -> (Self, i32) {
        let (normalizer, delta) = self.normalizer.push_at(at_ms, direction);
        self.normalizer = normalizer;
        self.backlog += delta;
        (self, delta)
    }

    pub const fn with_normalizer(mut self, normalizer: ScrollNormalizer) -> Self {
        self.normalizer = normalizer;
        self
    }

    pub const fn with_viewport_rows(mut self, viewport_rows: u16) -> Self {
        self.viewport_rows = viewport_rows;
        self
    }

    pub const fn flush_cap(self) -> i32 {
        let viewport_cap = (self.viewport_rows / 2) as i32;
        if viewport_cap > MIN_SCROLL_FLUSH_LINES {
            viewport_cap
        } else {
            MIN_SCROLL_FLUSH_LINES
        }
    }

    pub const fn flush_at(mut self, at_ms: u64) -> (Self, ScrollFlush) {
        let cap = self.flush_cap();
        let lines = if self.backlog < -cap {
            -cap
        } else if self.backlog > cap {
            cap
        } else {
            self.backlog
        };
        self.backlog -= lines;
        if lines != 0 {
            self.last_flush_ms = Some(at_ms);
        }
        (
            self,
            ScrollFlush {
                lines,
                backlog: self.backlog,
            },
        )
    }

    pub const fn finalize(mut self) -> (Self, ScrollFinalize) {
        let dropped = self.backlog;
        self.backlog = 0;
        self.last_flush_ms = None;
        (
            self,
            ScrollFinalize {
                flushed: 0,
                dropped,
                backlog: 0,
            },
        )
    }

    pub const fn backlog(self) -> i32 {
        self.backlog
    }
    pub const fn last_flush_ms(self) -> Option<u64> {
        self.last_flush_ms
    }

    pub const fn flush_due(self, at_ms: u64) -> bool {
        match self.last_flush_ms {
            None => true,
            Some(last) => at_ms.saturating_sub(last) >= DEFAULT_SCROLL_FLUSH_CADENCE_MS,
        }
    }

    pub const fn normalizer_for_replay(self) -> ScrollNormalizer {
        self.normalizer
    }
}
