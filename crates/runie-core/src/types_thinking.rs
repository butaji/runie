use crate::types::{ThinkingLevel, ThinkingLevelMap};

impl ThinkingLevelMap {
    pub fn value(&self, level: ThinkingLevel) -> Option<&str> {
        match level {
            ThinkingLevel::Off => self.off.as_deref(),
            ThinkingLevel::Minimal => self.minimal.as_deref(),
            ThinkingLevel::Low => self.low.as_deref(),
            ThinkingLevel::Medium => self.medium.as_deref(),
            ThinkingLevel::High => self.high.as_deref(),
            ThinkingLevel::XHigh => self.xhigh.as_deref(),
            ThinkingLevel::Max => self.max.as_deref(),
        }
    }
}
