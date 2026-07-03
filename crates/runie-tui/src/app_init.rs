//! Application initialization helpers.
//!
//! All startup file I/O is routed through actors so the event loop can never
//! be blocked and state mutations flow through the event system.

use runie_core::model::AppState;

/// Load skills, auth providers, and git info by sending intents to actors.
/// Actors emit facts (`SkillsLoaded`, `AuthLoaded`, `EnvDetected`) that
/// `UiActor` applies to `AppState` via the event dispatch path.
pub async fn bootstrap(state: &mut AppState) {
    let Some(handles) = state.actor_handles() else {
        return;
    };

    // Route all init I/O through IoActor — no direct state mutation here.
    handles.io.detect_env().await;
    handles.io.load_skills().await;
    handles.io.load_auth().await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use runie_core::event::Event;

    /// Layer 1 test: `bootstrap` sends actor messages instead of mutating state directly.
    /// Verifies by checking that the IoActor emits the expected facts on the event bus.
    #[tokio::test]
    async fn bootstrap_emits_load_intents() {
        // Use the test helper that creates a full LeaderHandle with real actors.
        let handle = runie_core::actors::leader::test_leader_handle().await;
        let bus = handle.event_bus().clone();

        let mut state = AppState::default();
        state.set_actor_handles(handle.clone());

        // Subscribe before bootstrap so we don't miss events.
        let mut sub = bus.subscribe();

        bootstrap(&mut state).await;

        // Wait for all three facts to be emitted (up to 5s).
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
        let mut found = 0u8;
        while tokio::time::Instant::now() < deadline {
            let rem = deadline - tokio::time::Instant::now();
            match tokio::time::timeout(rem, sub.recv()).await {
                Ok(Ok(evt)) => {
                    if matches!(
                        evt,
                        Event::SkillsLoaded { .. }
                            | Event::AuthLoaded { .. }
                            | Event::EnvDetected { .. }
                    ) {
                        found += 1;
                        if found >= 3 {
                            break;
                        }
                    }
                }
                Ok(Err(_)) | Err(_) => break,
            }
        }
        assert_eq!(
            found, 3,
            "Expected SkillsLoaded, AuthLoaded, and EnvDetected events"
        );

        handle.shutdown().await;
    }
}
