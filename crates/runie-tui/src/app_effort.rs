use super::App;

impl App {
    pub(super) fn model_has_declared_effort(model: &runie_core::types::Model) -> bool {
        model.thinking_level_map.as_ref().is_some_and(|map| {
            [
                &map.minimal,
                &map.low,
                &map.medium,
                &map.high,
                &map.xhigh,
                &map.max,
            ]
            .into_iter()
            .any(Option::is_some)
        })
    }

    pub(super) fn model_supports_effort(&self, effort: &str) -> bool {
        let Some(map) = self
            .model_catalog
            .snapshot()
            .selected
            .and_then(|model| model.thinking_level_map)
        else {
            return false;
        };
        [
            ("minimal", map.minimal),
            ("low", map.low),
            ("medium", map.medium),
            ("high", map.high),
            ("xhigh", map.xhigh),
            ("max", map.max),
        ]
        .into_iter()
        .any(|(level, wire)| level == effort && wire.is_some())
            || (effort == "off" && map.off.is_some())
    }
}
