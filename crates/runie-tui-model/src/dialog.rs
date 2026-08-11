//! Renderer-independent unified dialog domain model.

use std::fmt::Debug;

macro_rules! dialog_kinds {
    ($(($kind:ident, $hint:literal)),+ $(,)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub enum DialogKind { $($kind),+ }

        impl DialogKind {
            pub const fn hint(self) -> &'static str {
                match self { $(Self::$kind => $hint,)+ }
            }
        }
    };
}

dialog_kinds! {
    (List, "search: "),
    (Selector, "Select an item"),
    (Form, "Enter values: "),
    (Confirm, "Confirm action"),
    (TextInput, "Enter text"),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialogAction {
    pub id: &'static str,
    pub label: &'static str,
    pub hotkey: Option<&'static str>,
    pub enabled: DialogPredicate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogPredicate {
    Always,
}

impl DialogPredicate {
    pub fn evaluate(self, _frame: &DialogFrame) -> bool {
        matches!(self, Self::Always)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialogSpec {
    pub id: &'static str,
    pub title: &'static str,
    pub kind: DialogKind,
    pub actions: &'static [DialogAction],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialogResult {
    Selected(usize),
    Text(String),
    Confirmed(bool),
    Action(&'static str),
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialogFrame {
    pub spec: DialogSpec,
    pub query: String,
    pub selected: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DialogStack {
    frames: Vec<DialogFrame>,
}

/// Move through a finite dialog result set, wrapping at both ends.
pub fn wrap_dialog_selection(current: usize, delta: isize, count: usize) -> usize {
    if count == 0 {
        return 0;
    }
    let current = current % count;
    if delta.is_negative() {
        let backward = delta.unsigned_abs() % count;
        if backward > current {
            count - (backward - current)
        } else {
            current - backward
        }
    } else {
        let forward = delta as usize % count;
        if forward >= count - current {
            forward - (count - current)
        } else {
            current + forward
        }
    }
}

impl DialogStack {
    pub fn push(&mut self, spec: DialogSpec) {
        self.frames.push(DialogFrame {
            spec,
            query: String::new(),
            selected: 0,
        });
    }

    pub fn pop(&mut self) -> Option<DialogFrame> {
        self.frames.pop()
    }

    pub fn top(&self) -> Option<&DialogFrame> {
        self.frames.last()
    }

    pub fn top_mut(&mut self) -> Option<&mut DialogFrame> {
        self.frames.last_mut()
    }

    pub fn top_id(&self) -> Option<&'static str> {
        self.top().map(|frame| frame.spec.id)
    }

    pub fn depth(&self) -> usize {
        self.frames.len()
    }

    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    pub fn clear(&mut self) {
        self.frames.clear();
    }

    pub fn complete(&mut self, result: DialogResult) -> Option<(DialogFrame, DialogResult)> {
        self.pop().map(|frame| (frame, result))
    }

    /// Escape always returns to the previous frame, or closes the root frame.
    pub fn escape(&mut self) -> bool {
        self.pop().is_some()
    }
}

/// Declare a typed dialog specification and its static actions.
#[macro_export]
macro_rules! dialog_spec {
    (
        $name:ident => {
            id: $id:literal,
            title: $title:literal,
            kind: $kind:ident,
            actions: [$(
                { id: $action_id:literal, label: $label:literal $(, hotkey: $hotkey:literal)? }
            ),* $(,)?]
        }
    ) => {
        pub const $name: $crate::DialogSpec = $crate::DialogSpec {
            id: $id,
            title: $title,
            kind: $crate::DialogKind::$kind,
            actions: &[
                $(
                    $crate::DialogAction {
                        id: $action_id,
                        label: $label,
                        hotkey: dialog_spec!(@hotkey $($hotkey)?),
                        enabled: $crate::DialogPredicate::Always,
                    }
                ),*
            ],
        };
    };
    (@hotkey $hotkey:literal) => { Some($hotkey) };
    (@hotkey) => { None };
}

#[cfg(test)]
mod tests {
    use super::*;

    dialog_spec!(COMMANDS => {
        id: "commands",
        title: "Commands",
        kind: List,
        actions: [
            { id: "new", label: "New Session", hotkey: "n" },
            { id: "quit", label: "Quit" },
        ]
    });

    #[test]
    fn macro_builds_typed_spec() {
        assert_eq!(COMMANDS.kind, DialogKind::List);
        assert_eq!(COMMANDS.actions[0].hotkey, Some("n"));
        assert_eq!(COMMANDS.actions[1].hotkey, None);
    }

    #[test]
    fn dialog_kinds_replay_from_yaml_data() {
        let kinds = runie_core::replay_yaml_state(
            include_str!("../fixtures/dialog-kinds.yaml"),
            Vec::<DialogKind>::new(),
            |state, kind: &DialogKind| {
                state.push(*kind);
            },
        )
        .expect("dialog kind fixture");
        assert_eq!(
            kinds,
            [
                DialogKind::List,
                DialogKind::Selector,
                DialogKind::Form,
                DialogKind::Confirm,
                DialogKind::TextInput
            ]
        );
    }

    #[test]
    fn escape_pops_the_active_frame() {
        let mut stack = DialogStack::default();
        stack.push(COMMANDS.clone());
        stack.top_mut().expect("frame").query = "new".into();
        assert!(stack.escape());
        assert!(stack.is_empty());
    }

    #[test]
    fn completion_returns_typed_result_and_pops_frame() {
        let mut stack = DialogStack::default();
        stack.push(COMMANDS.clone());
        let (frame, result) = stack
            .complete(DialogResult::Selected(1))
            .expect("completed frame");
        assert_eq!(frame.spec.id, "commands");
        assert_eq!(result, DialogResult::Selected(1));
        assert!(stack.is_empty());
    }

    #[test]
    fn dialog_selection_wraps_in_both_directions() {
        assert_eq!(wrap_dialog_selection(0, -1, 4), 3);
        assert_eq!(wrap_dialog_selection(3, 1, 4), 0);
        assert_eq!(wrap_dialog_selection(1, 9, 4), 2);
        assert_eq!(wrap_dialog_selection(2, -7, 4), 3);
        assert_eq!(wrap_dialog_selection(8, 1, 0), 0);
    }
}
