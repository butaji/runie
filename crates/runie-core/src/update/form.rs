//! Form Action Types

/// What a form panel should do in response to an event.
#[derive(Debug, Clone)]
pub enum FormAction {
    /// Keep the form open, persist the panel state.
    KeepOpen,
    /// Close the form (no submit).
    Close,
    /// Close the form and dispatch the submit event.
    Submit(Option<crate::Event>),
    /// Go back one step: if the stack is deeper than the root, pop the
    /// current panel and keep the dialog open; if at the root, close
    /// the dialog. This is the semantic of ESC / back.
    Back,
}
