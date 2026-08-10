impl FeedState {
    fn set_tool_name(&mut self, id: String, name: String) {
        self.navigation
            .facts
            .tools
            .entry(id)
            .and_modify(|record| record.name = Some(name.clone()))
            .or_insert_with(|| ToolRecord::named(name));
    }

    fn reduce_content(&mut self, message: ScrollbackMsg) -> Result<(), ScrollbackMsg> {
        match message {
            ScrollbackMsg::Append(line) => self.append(line),
            ScrollbackMsg::AppendTurnSummary(text) => {
                self.append(Line::new(LineKind::TurnSummary, text))
            }
            ScrollbackMsg::TurnStart => self.navigation.facts.turn_started = true,
            ScrollbackMsg::TurnEnd => self.navigation.facts.turn_started = false,
            ScrollbackMsg::AssistantStreamStart => self.navigation.facts.assistant_stream_open = true,
            ScrollbackMsg::AssistantStreamEnd => self.navigation.facts.assistant_stream_open = false,
            ScrollbackMsg::Clear => self.clear(),
            ScrollbackMsg::SetTheme(theme) => self.navigation.theme = theme,
            ScrollbackMsg::SetToolArgs(id, args) => self.set_tool_args(id, args),
            ScrollbackMsg::RemoveToolArgs(id) => self.remove_tool_args(&id),
            ScrollbackMsg::ActivityReset => self.reset_activity(),
            ScrollbackMsg::ActivityToolStart(name) => self.start_activity_tool(&name),
            ScrollbackMsg::ActivityToolEnd { is_error } => self.finish_activity_tool(is_error),
            ScrollbackMsg::AdvanceAnimation => self.navigation.advance_animation(),
            ScrollbackMsg::RemoveKind(kind) => self.lines.retain(|line| line.kind != kind),
            ScrollbackMsg::NormalizeLiveCompletedAssistants => self.normalize_assistants(),
            ScrollbackMsg::AddLiveAssistantTimestamp(_) => {}
            ScrollbackMsg::RemoveEmptyAfter(kind) => self.remove_empty_after(kind),
            ScrollbackMsg::NormalizeActivitySpacing => self.normalize_activity_spacing(),
            ScrollbackMsg::SetReasoningExpanded(value) => {
                self.navigation.reasoning_expanded = value
            }
            ScrollbackMsg::SetActivityExpanded(value) => self.navigation.activity_expanded = value,
            ScrollbackMsg::ToggleActivityExpanded => {
                self.navigation.activity_expanded = !self.navigation.activity_expanded
            }
            ScrollbackMsg::SetPromptTimestamp(value) => self.navigation.prompt_timestamp = value,
            ScrollbackMsg::SetFollowLatestUser(value) => self.navigation.follow_latest_user = value,
            message => return Err(message),
        }
        Ok(())
    }
}

impl FeedState {
    fn set_tool_args(&mut self, id: String, args: serde_json::Value) {
        self.navigation.facts.tools.entry(id).or_default().set_args(args);
    }

    fn remove_tool_args(&mut self, id: &str) {
        if let Some(record) = self.navigation.facts.tools.get_mut(id) {
            record.clear_args();
        }
    }
}
