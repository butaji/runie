use crate::components::{DiffViewer, ModelPicker, SessionTreeNavigator};
use super::{PermissionModalState, CommandPaletteState};

/// OverlayState contains modal, palette, and picker states.
#[derive(Clone)]
pub struct OverlayState {
    pub permission_modal: PermissionModalState,
    pub command_palette: CommandPaletteState,
    pub model_picker: Option<ModelPicker>,
    pub diff_viewer: Option<DiffViewer>,
    pub session_tree: SessionTreeNavigator,
}

impl Default for OverlayState {
    fn default() -> Self {
        Self {
            permission_modal: PermissionModalState::default(),
            command_palette: CommandPaletteState::default(),
            model_picker: None,
            diff_viewer: None,
            session_tree: SessionTreeNavigator::new(),
        }
    }
}
