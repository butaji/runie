use crate::{dialog_spec, DialogSpec};

dialog_spec!(COMMAND_DIALOG_SPEC => {
    id: "commands", title: "Commands", kind: List,
    actions: [
        { id: "navigate", label: "nav", hotkey: "↑/↓" },
        { id: "select", label: "select", hotkey: "Enter" },
        { id: "back", label: "close", hotkey: "Esc" },
    ]
});
dialog_spec!(FILE_SELECTOR_DIALOG_SPEC => {
    id: "files", title: "Files", kind: Selector,
    actions: [{ id: "select", label: "Select", hotkey: "Enter" }, { id: "back", label: "Back", hotkey: "Esc" }]
});
dialog_spec!(MODEL_SELECTOR_DIALOG_SPEC => {
    id: "model", title: "Models", kind: Selector,
    actions: [{ id: "select", label: "Select", hotkey: "Enter" }, { id: "back", label: "Back", hotkey: "Esc" }]
});
dialog_spec!(SHORTCUTS_DIALOG_SPEC => {
    id: "shortcuts", title: "Shortcuts", kind: List,
    actions: [{ id: "back", label: "Back", hotkey: "Esc" }]
});
dialog_spec!(SESSION_DIALOG_SPEC => {
    id: "session", title: "Session Info", kind: Form,
    actions: [{ id: "back", label: "Back", hotkey: "Esc" }]
});
dialog_spec!(CHANGELOG_DIALOG_SPEC => {
    id: "changelog", title: "Changelog", kind: List,
    actions: [{ id: "back", label: "Back", hotkey: "Esc" }]
});
dialog_spec!(PALETTE_PARAMETERS_DIALOG_SPEC => {
    id: "palette-parameters", title: "Command Parameters", kind: Form,
    actions: [
        { id: "submit", label: "Run", hotkey: "Enter" },
        { id: "back", label: "Back", hotkey: "Esc" }
    ]
});
dialog_spec!(THEME_SELECTOR_DIALOG_SPEC => {
    id: "theme-selector", title: "Themes", kind: Selector,
    actions: [
        { id: "select", label: "Select", hotkey: "Enter" },
        { id: "preview", label: "Preview", hotkey: "Space" },
        { id: "back", label: "Back", hotkey: "Esc" }
    ]
});
dialog_spec!(COMMAND_RESULT_DIALOG_SPEC => {
    id: "command-result", title: "Command Result", kind: Form,
    actions: [{ id: "back", label: "Close", hotkey: "Esc" }]
});
dialog_spec!(USER_QUESTION_DIALOG_SPEC => {
    id: "user-question", title: "Question", kind: Selector,
    actions: [
        { id: "select", label: "Answer", hotkey: "Enter" },
        { id: "back", label: "Cancel", hotkey: "Esc" }
    ]
});

pub const COMMAND_DIALOG: DialogSpec = COMMAND_DIALOG_SPEC;
pub const FILE_SELECTOR_DIALOG: DialogSpec = FILE_SELECTOR_DIALOG_SPEC;
pub const MODEL_SELECTOR_DIALOG: DialogSpec = MODEL_SELECTOR_DIALOG_SPEC;
pub const SHORTCUTS_DIALOG: DialogSpec = SHORTCUTS_DIALOG_SPEC;
pub const SESSION_DIALOG: DialogSpec = SESSION_DIALOG_SPEC;
pub const CHANGELOG_DIALOG: DialogSpec = CHANGELOG_DIALOG_SPEC;
pub const PALETTE_PARAMETERS_DIALOG: DialogSpec = PALETTE_PARAMETERS_DIALOG_SPEC;
pub const THEME_SELECTOR_DIALOG: DialogSpec = THEME_SELECTOR_DIALOG_SPEC;
pub const COMMAND_RESULT_DIALOG: DialogSpec = COMMAND_RESULT_DIALOG_SPEC;
pub const USER_QUESTION_DIALOG: DialogSpec = USER_QUESTION_DIALOG_SPEC;
