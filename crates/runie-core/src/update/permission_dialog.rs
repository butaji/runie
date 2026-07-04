//! Hosted permission dialog builder.

use crate::commands::{DialogKind, DialogState};
use crate::dialog::{ItemAction, Panel, PanelStack};
use crate::model::PermissionRequestState;
use crate::permissions::format::format_tool_input;
use crate::Event;

/// Build a hosted form panel for a pending permission request.
pub fn build_permission_dialog(req: &PermissionRequestState) -> PanelStack {
    let request_id = req.request_id.clone();
    let tool = req.tool.clone();
    let summary = format_tool_input(&req.tool, &req.input);

    let panel = Panel::new("permission", "Permission Required")
        .form()
        .header(format!("Tool: {}", tool))
        .header(format!("Details: {}", summary))
        .item(
            "_Allow",
            ItemAction::Emit(Event::PermissionAllow {
                request_id: request_id.clone(),
            }),
        )
        .item(
            "Den_y",
            ItemAction::Emit(Event::PermissionDeny {
                request_id: request_id.clone(),
            }),
        )
        .item(
            "Always _Allow",
            ItemAction::Emit(Event::PermissionAlwaysAllow { request_id, tool }),
        );

    PanelStack::new(panel)
}

/// Build and wrap a hosted permission dialog as an open `DialogState`.
pub fn open_permission_dialog(req: &PermissionRequestState) -> DialogState {
    DialogState::Active {
        kind: DialogKind::Generic,
        panels: build_permission_dialog(req),
    }
}

#[cfg(test)]
mod tests {
    use crate::dialog::PanelItem;
    use crate::model::PermissionRequestState;
    use crate::update::permission_dialog::build_permission_dialog;

    #[test]
    fn permission_dialog_has_allow_deny_always() {
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
        assert!(labels.iter().any(|l| l == "_Allow"));
        assert!(labels.iter().any(|l| l == "Den_y"));
        assert!(labels.iter().any(|l| l == "Always _Allow"));
    }
}
