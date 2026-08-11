use crate::types::{ThinkingLevel, ThinkingLevelMap};

impl ThinkingLevel {
    pub const ALL: [Self; 7] = [
        Self::Off,
        Self::Minimal,
        Self::Low,
        Self::Medium,
        Self::High,
        Self::XHigh,
        Self::Max,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Minimal => "minimal",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::XHigh => "xhigh",
            Self::Max => "max",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|level| level.name() == name || (*level == Self::XHigh && name == "x-high"))
    }
}

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

    pub fn declared(&self) -> impl Iterator<Item = (ThinkingLevel, Option<&str>)> {
        ThinkingLevel::ALL
            .into_iter()
            .map(|level| (level, self.value(level)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vocabulary_is_ordered_and_accepts_wire_alias() {
        let names: Vec<_> = ThinkingLevel::ALL
            .into_iter()
            .map(ThinkingLevel::name)
            .collect();
        assert_eq!(
            names,
            ["off", "minimal", "low", "medium", "high", "xhigh", "max"]
        );
        assert_eq!(
            ThinkingLevel::from_name("x-high"),
            Some(ThinkingLevel::XHigh)
        );
        assert_eq!(ThinkingLevel::from_name("unknown"), None);
    }

    #[test]
    fn declared_projection_keeps_one_slot_per_level() {
        let map = ThinkingLevelMap {
            off: Some("off".into()),
            high: Some("high".into()),
            ..Default::default()
        };
        let declared: Vec<_> = map.declared().collect();
        assert_eq!(declared.len(), ThinkingLevel::ALL.len());
        assert_eq!(declared[0], (ThinkingLevel::Off, Some("off")));
        assert_eq!(declared[4], (ThinkingLevel::High, Some("high")));
    }
}
