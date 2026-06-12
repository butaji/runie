//! Dialog action handlers.

use crate::model::AppState;
use crate::Event;
use crate::update::FormAction;

impl AppState {
    pub fn form_panel_action(panel: &mut crate::dialog::Panel, event: Event) -> FormAction {
        use Event::*;
        use FormAction as A;
        match event {
            // ESC / close-key: stack nav. `update_form_panel` decides
            // whether to pop (stack deeper) or close (at root).
            SettingsClose | CommandFormClose | Abort => A::Back,
            CommandFormUp | HistoryPrev | SettingsUp | PaletteUp | ModelSelectorUp => {
                let _ = panel.select_up();
                A::KeepOpen
            }
            CommandFormDown | HistoryNext | SettingsDown | PaletteDown | ModelSelectorDown => {
                let _ = panel.select_down();
                A::KeepOpen
            }
            CommandFormInput(c) | Input(c) => {
                // If on a form field, type the character into the field.
                // If on a button, check for accelerator match instead.
                if panel.selected_form_field().is_some() {
                    Self::form_panel_edit_char(panel, c, true);
                    A::KeepOpen
                } else if let Some(crate::dialog::ItemAction::Emit(evt)) =
                    panel.find_button_by_accel(c)
                {
                    A::Submit(Some(evt.clone()))
                } else {
                    A::KeepOpen
                }
            }
            CommandFormBackspace | Backspace => {
                Self::form_panel_edit_char(panel, ' ', false);
                A::KeepOpen
            }
            CommandFormSubmit | Submit | SettingsSelect | PaletteSelect => {
                // If the selection is on a button (Action/FormSubmit), activate it
                // instead of submitting the whole form.
                if let Some(item) = panel.selected_item() {
                    match item {
                        crate::dialog::PanelItem::Action {
                            action: crate::dialog::ItemAction::Emit(evt),
                            ..
                        } => {
                            return A::Submit(Some(evt.clone()));
                        }
                        crate::dialog::PanelItem::Action { .. }
                        | crate::dialog::PanelItem::FormSubmit => {
                            return A::Submit(None);
                        }
                        _ => {}
                    }
                }
                A::Submit(Self::form_build_submit(panel))
            }
            _ => A::KeepOpen,
        }
    }

    /// Append (push=true) or delete one char (push=false) from the selected form field.
    fn form_panel_edit_char(panel: &mut crate::dialog::Panel, c: char, push: bool) {
        let Some(idx) = panel.selected_form_field() else {
            return;
        };
        let crate::dialog::PanelItem::FormField { value, key, .. } = &mut panel.items[idx] else {
            return;
        };
        if push {
            value.push(c);
        } else {
            value.pop();
        }
        panel.form_values.insert(key.clone(), value.clone());
    }

    pub(crate) fn handle_panel_action(
        &mut self,
        action: crate::dialog::ItemAction,
        stack: &mut crate::dialog::PanelStack,
    ) -> bool {
        use crate::dialog::ItemAction;
        match action {
            ItemAction::Push(_) | ItemAction::Pop => {
                stack.pop();
                false
            }
            ItemAction::Close => {
                self.open_dialog = None;
                self.mark_dirty();
                true
            }
            ItemAction::Emit(evt) => {
                let keep_open = stack
                    .current()
                    .map(|p| p.keep_open_on_activate)
                    .unwrap_or(false);
                if !keep_open {
                    self.open_dialog = None;
                }
                self.mark_dirty();
                self.update(evt);
                !keep_open
            }
            ItemAction::Toggle(key) => {
                self.panel_toggle_item(stack, &key);
                false
            }
            ItemAction::Cycle(key) => {
                self.panel_cycle_item(stack, &key);
                false
            }
        }
    }

    fn panel_toggle_item(&mut self, stack: &mut crate::dialog::PanelStack, key: &str) {
        use crate::dialog::PanelItem;
        if let Some(PanelItem::Toggle { value, .. }) =
            stack.current_mut().and_then(|p| p.selected_item_mut())
        {
            *value = !*value;
        }
        self.apply_panel_setting(key);
    }

    fn panel_cycle_item(&mut self, stack: &mut crate::dialog::PanelStack, key: &str) {
        use crate::dialog::PanelItem;
        if let Some(PanelItem::Select {
            current, options, ..
        }) = stack.current_mut().and_then(|p| p.selected_item_mut())
        {
            if let Some(idx) = options.iter().position(|o| o == current) {
                let next = (idx + 1) % options.len();
                *current = options[next].clone();
            }
        }
        self.apply_panel_setting(key);
    }

    fn apply_panel_setting(&mut self, key: &str) {
        match key {
            "read_only" => {
                self.config.read_only = !self.config.read_only;
                let status = if self.config.read_only {
                    "enabled"
                } else {
                    "disabled"
                };
                self.notify(
                    format!("Read-only mode {}", status),
                    crate::event::TransientLevel::Warning,
                );
            }
            "steering_mode" => {
                self.steering_mode = match self.steering_mode {
                    crate::model::DeliveryMode::OneAtATime => crate::model::DeliveryMode::All,
                    crate::model::DeliveryMode::All => crate::model::DeliveryMode::OneAtATime,
                };
            }
            "follow_up_mode" => {
                self.follow_up_mode = match self.follow_up_mode {
                    crate::model::DeliveryMode::OneAtATime => crate::model::DeliveryMode::All,
                    crate::model::DeliveryMode::All => crate::model::DeliveryMode::OneAtATime,
                };
            }
            _ => {}
        }
    }

    pub(crate) fn process_command_result(&mut self, result: crate::commands::CommandResult) {
        match result {
            crate::commands::CommandResult::Message(msg) => self.add_system_msg(msg),
            crate::commands::CommandResult::Event(evt) => self.update(evt),
            crate::commands::CommandResult::OpenDialog(d) => match d {
                crate::commands::DialogType::CommandPalette => self.open_command_palette(),
                crate::commands::DialogType::ModelSelector => self.open_model_selector(),
                crate::commands::DialogType::Settings => self.open_settings_dialog(),
                crate::commands::DialogType::ScopedModels => self.open_scoped_models_dialog(),
            },
            crate::commands::CommandResult::OpenPanelStack(stack) => {
                self.open_dialog = Some(crate::commands::DialogState::PanelStack(stack));
                self.mark_dirty();
            }
            crate::commands::CommandResult::None => {}
        }
    }

    /// Open a filterable @-file picker as a PanelStack dialog.
    pub(super) fn insert_at_ref(&mut self, path: &str) {
        self.input.input = path.to_string();
        self.input.cursor_pos = self.input.input.len();
        self.open_dialog = None;
        self.mark_dirty();
    }

}
