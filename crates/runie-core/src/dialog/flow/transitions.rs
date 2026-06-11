//! Common Transitions

use crate::dialog::dsl::ItemAction;
use crate::Event;

/// Create a transition to a named panel
pub fn push(id: impl Into<String>) -> ItemAction {
    ItemAction::Push(id.into())
}

/// Create a back transition
pub fn pop() -> ItemAction {
    ItemAction::Pop
}

/// Create a close transition
pub fn close() -> ItemAction {
    ItemAction::Close
}

/// Create an emit transition
pub fn emit(event: Event) -> ItemAction {
    ItemAction::Emit(event)
}

/// Navigate to a flow step
pub fn goto_step(flow_id: &str, step_index: usize) -> ItemAction {
    let target = format!("{}:{}", flow_id, step_index);
    ItemAction::Push(target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dialog::ItemAction;

    #[test]
    fn test_push_pop_close() {
        let push_action = push("next");
        let pop_action = pop();
        let close_action = close();

        assert!(matches!(push_action, ItemAction::Push(id) if id == "next"));
        assert!(matches!(pop_action, ItemAction::Pop));
        assert!(matches!(close_action, ItemAction::Close));
    }

    #[test]
    fn test_emit() {
        let action = emit(Event::RunSaveCommand { name: String::new() });
        assert!(matches!(action, ItemAction::Emit(Event::RunSaveCommand { .. })));
    }

    #[test]
    fn test_goto_step() {
        let action = goto_step("wizard", 2);
        assert!(matches!(action, ItemAction::Push(id) if id == "wizard:2"));
    }
}
