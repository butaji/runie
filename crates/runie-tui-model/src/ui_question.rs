impl UiState {
    fn update_user_question(mut self, msg: UiMsg) -> Option<Self> {
        let question = self.user_question.as_ref()?;
        match msg {
            UiMsg::UserQuestionMove(delta) => {
                let frame = self.dialog_stack.top_mut()?;
                frame.selected = crate::wrap_dialog_selection(
                    frame.selected,
                    delta,
                    question.request.options.len(),
                );
            }
            UiMsg::ToggleUserQuestionSelection => {
                if !question.request.allow_multiple { return Some(self); }
                let index = self.dialog_stack.top()?.selected;
                if let Some(position) = self.user_question_selected.iter().position(|value| *value == index) {
                    self.user_question_selected.remove(position);
                } else {
                    self.user_question_selected.push(index);
                }
            }
            UiMsg::SubmitUserQuestion => { self.dialog_stack.pop(); }
            _ => return None,
        }
        Some(self)
    }
}
