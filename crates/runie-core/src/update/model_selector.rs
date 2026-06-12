use crate::model::AppState;
use crate::model_catalog::{build_model_selector_items, model_catalog};
use crate::Event;
use crate::commands::DialogState;

impl AppState {
    pub(crate) fn update_model_selector(
        &mut self,
        event: Event,
        filter: String,
        selected: usize,
    ) {
        match event {
            Event::Abort | Event::ModelSelectorClose | Event::ToggleModelSelector => {
                self.open_dialog = None;
                self.mark_dirty();
            }
            Event::Input(c) | Event::ModelSelectorFilter(c) => {
                self.model_selector_push_char(filter, c);
            }
            Event::Backspace | Event::ModelSelectorBackspace => {
                self.model_selector_pop_char(filter);
            }
            Event::HistoryPrev | Event::ModelSelectorUp => {
                self.model_selector_move_up(filter, selected);
            }
            Event::HistoryNext | Event::ModelSelectorDown => {
                self.model_selector_move_down(filter, selected);
            }
            Event::Submit | Event::ModelSelectorSelect => {
                self.model_selector_select(filter, selected);
            }
            _ => {
                self.set_model_selector(filter, selected);
            }
        }
    }

    fn model_selector_push_char(&mut self, mut filter: String, c: char) {
        filter.push(c);
        self.set_model_selector(filter, 0);
    }

    fn model_selector_pop_char(&mut self, mut filter: String) {
        filter.pop();
        self.set_model_selector(filter, 0);
    }

    fn model_selector_move_up(&mut self, filter: String, selected: usize) {
        let items = build_model_selector_items(
            &model_catalog(),
            &self.recent_models,
            &filter,
            &self.config.current_provider,
            &self.config.current_model,
        );
        let new_sel = if selected == 0 {
            items.len().saturating_sub(1)
        } else {
            selected - 1
        };
        self.set_model_selector(filter, new_sel);
    }

    fn model_selector_move_down(&mut self, filter: String, selected: usize) {
        let items = build_model_selector_items(
            &model_catalog(),
            &self.recent_models,
            &filter,
            &self.config.current_provider,
            &self.config.current_model,
        );
        let new_sel = if items.is_empty() {
            0
        } else {
            (selected + 1) % items.len()
        };
        self.set_model_selector(filter, new_sel);
    }

    fn model_selector_select(&mut self, filter: String, selected: usize) {
        let items = build_model_selector_items(
            &model_catalog(),
            &self.recent_models,
            &filter,
            &self.config.current_provider,
            &self.config.current_model,
        );
        if let Some(item) = items.get(selected) {
            let parts: Vec<&str> = item.1.split('/').collect();
            if parts.len() == 2 {
                self.switch_model(parts[0].to_string(), parts[1].to_string());
            }
        }
        self.open_dialog = None;
        self.mark_dirty();
    }

    fn set_model_selector(&mut self, filter: String, selected: usize) {
        self.open_dialog = Some(DialogState::ModelSelector {
            filter,
            selected,
        });
        self.mark_dirty();
    }
}
