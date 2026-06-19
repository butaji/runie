//! Blocking permission approval modal.

use ratatui::{
    text::Line,
    widgets::{Paragraph, Wrap},
    Frame,
};
use runie_core::Snapshot;

use crate::theme::style_hint;

/// Render a blocking modal asking the user to allow or deny a tool call.
pub fn permission_dialog(f: &mut Frame, snap: &Snapshot) {
    let request = match &snap.permission_request {
        Some(r) => r,
        None => return,
    };

    let inner = super::panel::setup_popup(f, " Permission Required ");
    let input_text = serde_json::to_string_pretty(&request.input).unwrap_or_default();
    let lines = vec![
        Line::from(format!("Tool: {}", request.tool)),
        Line::from(""),
        Line::from(format!("Input: {}", input_text)),
        Line::from(""),
        Line::from("[y] Allow   [n] Deny   [a] Always allow").style(style_hint()),
    ];

    f.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: true })
            .style(style_hint()),
        inner,
    );
}
