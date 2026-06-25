//! Fact types for the declarative DSL.
//!
//! Facts are the authoritative record of state changes broadcast by actors.
//! They are the only thing that reaches `AppState::update()`.

/// A fact broadcast by an actor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fact {
    /// Config has been reloaded from disk.
    ConfigReloaded,
    /// Session state changed (messages, tree, etc.).
    SessionChanged,
    /// Input state changed.
    InputChanged,
    /// View cache is stale — render must rebuild.
    ViewInvalidated,
    /// Completion popup changed.
    CompletionChanged,
    /// Turn progressed.
    TurnProgress,
    /// Permission request arrived (blocking modal).
    PermissionRequest,
    /// Permission was resolved.
    PermissionResolved,
    /// Trust decision changed.
    TrustChanged,
    /// Transient message cleared.
    TransientCleared,
    /// An IO operation completed.
    IoComplete,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fact_clone_is_equal() {
        let fact = Fact::ViewInvalidated;
        assert_eq!(fact, fact.clone());
    }

    #[test]
    fn fact_debug_format() {
        let fact = Fact::SessionChanged;
        assert!(format!("{:?}", fact).contains("SessionChanged"));
    }
}
