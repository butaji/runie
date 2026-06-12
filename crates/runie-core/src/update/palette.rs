use crate::model::AppState;
use crate::Event;

impl AppState {
    pub(crate) fn update_palette(&mut self, event: Event, filter: String, selected: usize) {
        match event {
            Event::Abort | Event::PaletteClose | Event::ToggleCommandPalette => {
                self.open_dialog = None;
                self.mark_dirty();
            }
            Event::Input(c) | Event::PaletteFilter(c) => {
                self.palette_push_char(filter, c);
            }
            Event::Backspace | Event::PaletteBackspace => {
                self.palette_pop_char(filter);
            }
            Event::HistoryPrev | Event::PaletteUp => {
                self.palette_move_up(filter, selected);
            }
            Event::HistoryNext | Event::PaletteDown => {
                self.palette_move_down(filter, selected);
            }
            Event::Submit | Event::PaletteSelect => {
                self.palette_select(filter, selected);
            }
            _ => {
                self.set_palette(filter, selected);
            }
        }
    }

    fn palette_push_char(&mut self, mut filter: String, c: char) {
        filter.push(c);
        self.set_palette(filter, 0);
    }

    fn palette_pop_char(&mut self, mut filter: String) {
        filter.pop();
        self.set_palette(filter, 0);
    }

    pub(crate) fn filtered_skills(&self, filter: &str) -> Vec<&crate::skills::Skill> {
        let f = filter.to_lowercase();
        self.skills
            .iter()
            .filter(|s| {
                s.user_invocable
                    && (f.is_empty()
                        || s.name.to_lowercase().contains(&f)
                        || s.description.to_lowercase().contains(&f))
            })
            .collect()
    }

    fn palette_move_up(&mut self, filter: String, selected: usize) {
        let total = crate::commands::filter_commands(&self.registry, &filter).len()
            + self.filtered_skills(&filter).len();
        let new_sel = if selected == 0 {
            total.saturating_sub(1)
        } else {
            selected - 1
        };
        self.set_palette(filter, new_sel);
    }

    fn palette_move_down(&mut self, filter: String, selected: usize) {
        let total = crate::commands::filter_commands(&self.registry, &filter).len()
            + self.filtered_skills(&filter).len();
        let new_sel = if total == 0 {
            0
        } else {
            (selected + 1) % total
        };
        self.set_palette(filter, new_sel);
    }

    fn palette_select(&mut self, filter: String, selected: usize) {
        // Parse filter as "command args" to support typing commands with arguments
        let (cmd_part, args) = filter.split_once(' ').unwrap_or((&filter, ""));

        // Try exact match by name or alias first
        if let Some(cmd) = self.registry.get(cmd_part) {
            let cmd_name = cmd.name.clone();
            let result = cmd.flow.clone().exec(self, &cmd_name, args);
            self.process_command_result(result);
            self.mark_dirty();
            return;
        }

        // Fall back to selection-based execution from filtered list
        let cmd_items = crate::commands::filter_commands(&self.registry, &filter);
        let skill_items = self.filtered_skills(&filter);

        if selected < cmd_items.len() {
            if let Some(cmd) = cmd_items.get(selected) {
                let cmd_name = cmd.name.clone();
                let result = cmd.flow.clone().exec(self, &cmd_name, args);
                self.process_command_result(result);
            }
        } else if let Some(skill) = skill_items.get(selected - cmd_items.len()) {
            self.add_system_msg(format!(
                "Skill: {}\nDescription: {}\nContext: {}",
                skill.name, skill.description, skill.context
            ));
        } else {
            self.add_system_msg(format!("Unknown command: /{}. Try /help.", filter));
        }
        self.mark_dirty();
    }

    fn set_palette(&mut self, filter: String, selected: usize) {
        self.open_dialog = Some(crate::commands::DialogState::CommandPalette {
            filter,
            selected,
        });
        self.mark_dirty();
    }
}
