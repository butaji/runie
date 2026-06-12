//! Scoped model entry for Ctrl+P cycling.

/// A model entry in the scoped cycling list.
#[derive(Clone, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct ScopedModel {
    pub name: String,
    pub provider: String,
    pub enabled: bool,
}

impl ScopedModel {
    pub fn full(&self) -> String {
        format!("{}/{}", self.provider, self.name)
    }
}
