//! Protocol version constant and type.

/// Current wire-protocol version.
pub const PROTOCOL_VERSION: &str = "0.1.0";

/// Protocol version carried in every message.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct Version(pub String);

impl Version {
    /// The current protocol version.
    pub fn current() -> Self {
        Self(PROTOCOL_VERSION.to_string())
    }
}

impl Default for Version {
    fn default() -> Self {
        Self::current()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_current_matches_const() {
        assert_eq!(Version::current().0, PROTOCOL_VERSION);
    }
}
