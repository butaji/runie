use super::App;

impl App {
    pub(super) async fn open_effort_picker(&self, model: &runie_core::types::Model) {
        self.ui
            .send(crate::app::UiMsg::SetPaletteParameterOptions(
                Self::declared_effort_options(model),
            ))
            .await;
        self.ui
            .send(crate::app::UiMsg::OpenPaletteParameters(
                crate::app::PaletteAction::SetEffort,
            ))
            .await;
    }

    pub(super) fn declared_effort_options(model: &runie_core::types::Model) -> Vec<String> {
        let Some(map) = model.thinking_level_map.as_ref() else {
            return Vec::new();
        };
        [
            ("off", &map.off),
            ("minimal", &map.minimal),
            ("low", &map.low),
            ("medium", &map.medium),
            ("high", &map.high),
            ("xhigh", &map.xhigh),
            ("max", &map.max),
        ]
        .into_iter()
        .filter_map(|(name, wire)| wire.as_ref().map(|_| name.to_owned()))
        .collect()
    }

    pub(super) fn model_has_declared_effort(model: &runie_core::types::Model) -> bool {
        model.thinking_level_map.as_ref().is_some_and(|map| {
            [
                &map.off,
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

#[cfg(test)]
mod tests {
    #[test]
    fn declared_effort_options_preserve_model_order() {
        let model = runie_core::types::Model {
            thinking_level_map: Some(runie_core::types::ThinkingLevelMap {
                low: Some("low-wire".into()),
                high: Some("high-wire".into()),
                ..Default::default()
            }),
            ..Default::default()
        };
        assert_eq!(
            super::App::declared_effort_options(&model),
            vec!["low".to_owned(), "high".to_owned()]
        );
    }

    #[test]
    fn an_off_only_model_still_requires_effort_selection() {
        let model = runie_core::types::Model {
            thinking_level_map: Some(runie_core::types::ThinkingLevelMap {
                off: Some("disabled".into()),
                ..Default::default()
            }),
            ..Default::default()
        };
        assert_eq!(super::App::declared_effort_options(&model), vec!["off"]);
    }
}
