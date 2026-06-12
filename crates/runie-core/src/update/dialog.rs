//! Dialog handling.

use crate::model::AppState;
use crate::update::FormAction;

impl AppState {





















    /// Apply a `FormAction` to the current dialog. Mirrors the KeepOpen /
    /// Close / Submit paths in `update_form_panel`. `Back` is handled
    /// in `update_form_panel` itself (stack-level) and never reaches here;
    /// we include it to keep the match exhaustive.
    pub(crate) fn apply_form_action(&mut self, action: FormAction) {
        match action {
            FormAction::Close => {
                self.open_dialog = None;
                self.mark_dirty();
            }
            FormAction::Submit(evt) => {
                self.open_dialog = None;
                self.mark_dirty();
                if let Some(e) = evt {
                    self.update(e);
                }
            }
            FormAction::KeepOpen => {
                self.mark_dirty();
            }
            FormAction::Back => {
                // Handled in `update_form_panel` (pop or close based on
                // stack depth). This branch is defensive in case future
                // code paths route a Back action through here.
            }
        }
    }
    /// Build the submit event for a form panel by reading form values and
    /// dispatching via the form-command table. The panel's `id` selects which
    /// command to run. Returns `None` for unknown command ids.
    pub(crate) fn form_build_submit(panel: &mut crate::dialog::Panel) -> Option<crate::Event> {
        let values = panel.get_form_values().clone();
        let cmd = panel.id.clone();
        match cmd.as_str() {
            "save" => Some(crate::Event::RunSaveCommand {
                name: values.get("name").cloned().unwrap_or_default(),
            }),
            "load" => Some(crate::Event::RunLoadCommand {
                name: values.get("name").cloned().unwrap_or_default(),
            }),
            "delete" => Some(crate::Event::RunDeleteCommand {
                name: values.get("name").cloned().unwrap_or_default(),
            }),
            "import" => Some(crate::Event::RunImportCommand {
                path: values.get("path").cloned().unwrap_or_default(),
            }),
            "export" => Some(crate::Event::RunExportCommand {
                path: values.get("path").cloned().unwrap_or_default(),
            }),
            "skill" => Some(crate::Event::RunSkillCommand {
                name: values.get("name").cloned().unwrap_or_default(),
            }),
            "login" => Some(crate::Event::RunLoginCommand {
                provider: values.get("provider").cloned().unwrap_or_default(),
                token: values.get("token").cloned().unwrap_or_default(),
            }),
            "logout" => Some(crate::Event::RunLogoutCommand {
                provider: values.get("provider").cloned().unwrap_or_default(),
            }),
            "name" => Some(crate::Event::RunNameCommand {
                name: values.get("name").cloned().unwrap_or_default(),
            }),
            "fork" => {
                let index = values.get("index").cloned().unwrap_or_default();
                Some(crate::Event::RunForkCommand {
                    message_index: index,
                })
            }
            "compact" => {
                let keep = values.get("keep").cloned().unwrap_or_default();
                let focus = values.get("focus").cloned().unwrap_or_default();
                Some(crate::Event::RunCompactCommand { keep, focus })
            }
            "prompt" => Some(crate::Event::RunPromptCommand {
                name: values.get("name").cloned().unwrap_or_default(),
            }),
            "login-key" => Some(crate::Event::LoginFlowSubmitKey {
                provider: String::new(),
                key: values.get("key").cloned().unwrap_or_default(),
            }),
            _ => None,
        }
    }
}
