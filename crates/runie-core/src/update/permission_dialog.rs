//! Hosted permission dialog builder.
//!
//! The 4-option dialog matches Grok's behavior:
//! - Always (1): persists across sessions (stored in config)
//! - This session (2): persists for the current session only
//! - Once (3): single use, will ask again next time
//! - Deny (4): reject

use crate::commands::{DialogKind, DialogState};
use crate::dialog::{ItemAction, Panel, PanelStack};
use crate::model::PermissionRequestState;
use crate::permissions::format::format_tool_input;
use crate::Event;

/// Build a hosted form panel for a pending permission request.
/// Shows 4 options: Always (1), This session (2), Once (3), Deny (4)
pub fn build_permission_dialog(req: &PermissionRequestState) -> PanelStack {
    let request_id = req.request_id.clone();
    let tool = req.tool.clone();
    let summary = format_tool_input(&req.tool, &req.input);

    let panel = Panel::new("permission", "Permission Required")
        .form()
        .non_closable() // Permission dialog is non-closable until user makes a choice
        .header(format!("Tool: {}", tool))
        .header(format!("Details: {}", summary))
        // Always (1) — persists across sessions
        .item(
            "_1. Always",
            ItemAction::Emit(Event::PermissionAlwaysAllow { request_id: request_id.clone(), tool: tool.clone() }),
        )
        // This session (2) — persists for current session only
        .item(
            "_2. This session",
            ItemAction::Emit(Event::PermissionSessionAllow { request_id: request_id.clone(), tool: tool.clone() }),
        )
        // Once (3) — single use
        .item(
            "_3. Once",
            ItemAction::Emit(Event::PermissionOnce { request_id: request_id.clone() }),
        )
        // Deny (4) — reject
        .item(
            "_4. Deny",
            ItemAction::Emit(Event::PermissionDeny { request_id }),
        );

    PanelStack::new(panel)
}

/// Build and wrap a hosted permission dialog as an open `DialogState`.
pub fn open_permission_dialog(req: &PermissionRequestState) -> DialogState {
    DialogState::Active { kind: DialogKind::Generic, panels: build_permission_dialog(req) }
}

#[cfg(test)]
mod tests {
    use crate::dialog::PanelItem;
    use crate::model::PermissionRequestState;
    use crate::update::permission_dialog::build_permission_dialog;

    #[test]
    fn permission_dialog_has_four_options() {
        let req = PermissionRequestState {
            request_id: "req-1".into(),
            tool: "list_dir".into(),
            input: serde_json::json!({"path": "."}),
        };
        let stack = build_permission_dialog(&req);
        let panel = stack.current().expect("panel exists");
        assert!(panel.is_form());
        assert_eq!(panel.title, " Permission Required ");
        let labels: Vec<_> = panel
            .items
            .iter()
            .filter_map(|i| match i {
                PanelItem::Action { label, .. } => Some(label.clone()),
                _ => None,
            })
            .collect();
        // Check all 4 options are present
        assert!(
            labels.iter().any(|l| l == "_1. Always"),
            "Missing Always option: {labels:?}"
        );
        assert!(
            labels.iter().any(|l| l == "_2. This session"),
            "Missing This session option: {labels:?}"
        );
        assert!(
            labels.iter().any(|l| l == "_3. Once"),
            "Missing Once option: {labels:?}"
        );
        assert!(
            labels.iter().any(|l| l == "_4. Deny"),
            "Missing Deny option: {labels:?}"
        );
    }

    #[test]
    fn permission_dialog_first_option_selected() {
        let req = PermissionRequestState {
            request_id: "req-1".into(),
            tool: "list_dir".into(),
            input: serde_json::json!({"path": "."}),
        };
        let stack = build_permission_dialog(&req);
        let panel = stack.current().expect("panel exists");
        // First option (Always) should be selected
        assert_eq!(panel.selected, 0);
    }
}
