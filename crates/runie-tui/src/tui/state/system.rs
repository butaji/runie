use super::ClearInputConfirm;

/// SystemState contains status and app lifecycle fields.
#[derive(Clone)]
pub struct SystemState {
    pub running: bool,
    pub mock_mode: bool,
    pub status_header: Option<String>,
    pub status_details: Option<String>,
    pub status_start_time: Option<std::time::Instant>,
    pub clear_input_confirm: ClearInputConfirm,
}

impl Default for SystemState {
    fn default() -> Self {
        Self {
            running: true,
            mock_mode: false,
            status_header: None,
            status_details: None,
            status_start_time: None,
            clear_input_confirm: ClearInputConfirm::default(),
        }
    }
}
