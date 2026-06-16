//! Login flow event variants.

use std::fmt;
use strum::IntoStaticStr;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, IntoStaticStr)]
#[strum(serialize_all = "PascalCase")]
pub enum LoginFlowEvent {
    Start,
    SelectProvider { provider: String },
    SubmitKey { provider: String, key: String },
    ValidationDone { provider: String, key: String, models: Vec<String> },
    ValidationFailed { provider: String, key: String, error: String },
    ModelsFetched { provider: String, key: String, models: Vec<String> },
    ToggleModel { model: String },
    Save,
    Cancel,
}

impl fmt::Display for LoginFlowEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoginFlowEvent::Start => write!(f, "Start"),
            LoginFlowEvent::SelectProvider { .. } => write!(f, "SelectProvider"),
            LoginFlowEvent::SubmitKey { .. } => write!(f, "SubmitKey"),
            LoginFlowEvent::ValidationDone { .. } => write!(f, "ValidationDone"),
            LoginFlowEvent::ValidationFailed { .. } => write!(f, "ValidationFailed"),
            LoginFlowEvent::ModelsFetched { .. } => write!(f, "ModelsFetched"),
            LoginFlowEvent::ToggleModel { .. } => write!(f, "ToggleModel"),
            LoginFlowEvent::Save => write!(f, "Save"),
            LoginFlowEvent::Cancel => write!(f, "Cancel"),
        }
    }
}
