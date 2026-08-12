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
        map.declared()
            .filter_map(|(level, wire)| wire.map(|_| level.name().to_owned()))
            .collect()
    }

    pub(super) fn model_has_declared_effort(model: &runie_core::types::Model) -> bool {
        model
            .thinking_level_map
            .as_ref()
            .is_some_and(|map| map.declared().any(|(_, wire)| wire.is_some()))
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
        let Some(level) = runie_core::types::ThinkingLevel::from_name(effort) else {
            return false;
        };
        map.value(level).is_some()
    }

    pub(super) fn default_effort_for_model(
        model: &runie_core::types::Model,
    ) -> runie_core::types::ThinkingLevel {
        let Some(map) = model.thinking_level_map.as_ref() else {
            return runie_core::types::ThinkingLevel::Off;
        };
        if let Some(level) = model
            .default_thinking_level
            .filter(|level| map.value(*level).is_some())
        {
            return level;
        }
        let declared = map
            .declared()
            .filter_map(|(level, wire)| wire.map(|_| level))
            .collect::<Vec<_>>();
        declared
            .get(declared.len() / 2)
            .copied()
            .unwrap_or(runie_core::types::ThinkingLevel::Off)
    }

    pub(super) async fn reset_effort_for_model(&self, model: &runie_core::types::Model) {
        self.loop_actor
            .set_thinking_level(Self::default_effort_for_model(model))
            .await;
    }

    pub(super) async fn set_model_with_declared_effort(&self, model: runie_core::types::Model) {
        self.loop_actor.set_model(model.clone()).await;
        self.reset_effort_for_model(&model).await;
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct EffortTrace {
        model: runie_core::types::Model,
        selected: String,
        unsupported: String,
    }

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

    #[test]
    fn default_effort_is_owned_by_the_model_declaration() {
        let model = runie_core::types::Model {
            thinking_level_map: Some(runie_core::types::ThinkingLevelMap {
                high: Some("high-wire".into()),
                max: Some("max-wire".into()),
                ..Default::default()
            }),
            ..Default::default()
        };
        assert_eq!(
            super::App::default_effort_for_model(&model),
            runie_core::types::ThinkingLevel::Max
        );
        assert_eq!(
            super::App::default_effort_for_model(&runie_core::types::Model::default()),
            runie_core::types::ThinkingLevel::Off
        );
    }

    #[test]
    fn valid_model_default_effort_overrides_first_declared_level() {
        let model = runie_core::types::Model {
            thinking_level_map: Some(runie_core::types::ThinkingLevelMap {
                low: Some("low-wire".into()),
                high: Some("high-wire".into()),
                ..Default::default()
            }),
            default_thinking_level: Some(runie_core::types::ThinkingLevel::High),
            ..Default::default()
        };
        assert_eq!(
            super::App::default_effort_for_model(&model),
            runie_core::types::ThinkingLevel::High
        );
    }

    #[test]
    fn unsupported_model_default_falls_back_to_only_declared_level() {
        let model = runie_core::types::Model {
            thinking_level_map: Some(runie_core::types::ThinkingLevelMap {
                low: Some("low-wire".into()),
                ..Default::default()
            }),
            default_thinking_level: Some(runie_core::types::ThinkingLevel::Max),
            ..Default::default()
        };
        assert_eq!(
            super::App::default_effort_for_model(&model),
            runie_core::types::ThinkingLevel::Low
        );
    }

    #[test]
    fn missing_model_default_uses_middle_declared_effort() {
        let model = runie_core::types::Model {
            thinking_level_map: Some(runie_core::types::ThinkingLevelMap {
                low: Some("low-wire".into()),
                high: Some("high-wire".into()),
                max: Some("max-wire".into()),
                ..Default::default()
            }),
            ..Default::default()
        };
        assert_eq!(
            super::App::default_effort_for_model(&model),
            runie_core::types::ThinkingLevel::High
        );
    }

    #[test]
    fn yaml_effort_trace_uses_only_the_model_declared_levels() {
        let trace: EffortTrace =
            serde_yaml::from_str(include_str!("fixtures/effort-selection.yaml"))
                .expect("effort fixture");
        assert_eq!(
            super::App::declared_effort_options(&trace.model),
            vec!["low", "high"]
        );
        assert!(trace
            .model
            .thinking_level_map
            .as_ref()
            .unwrap()
            .high
            .is_some());
        assert!(!super::App::declared_effort_options(&trace.model).contains(&trace.unsupported));
        assert!(super::App::declared_effort_options(&trace.model).contains(&trace.selected));
    }
}
