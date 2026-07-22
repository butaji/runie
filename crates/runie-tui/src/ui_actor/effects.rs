//! Effects dispatch — routes side effects to IoActor.

use runie_core::Event;

use crate::effects::EffectCommand;
use crate::ui_actor::UiActor;

/// Dispatch effects via IoActor.
///
/// Converts events to EffectCommands and spawns async handlers.
/// For login validation, handles separately to await the provider response.
pub(crate) async fn dispatch(ui: &mut UiActor, evt: &Event, effect_tx: tokio::sync::mpsc::Sender<Event>) {
    if let Some(cmd) = EffectCommand::try_from_event(evt, &mut ui.state, &ui.caps) {
        // For login validation, handle separately
        if matches!(cmd, EffectCommand::LoginFlowSubmitKey { .. }) {
            let flow = ui.state.login_flow().cloned();
            if let Some(f) = flow {
                let tx = effect_tx.clone();
                let provider_handle = ui
                    .state
                    .actor_handles()
                    .as_ref()
                    .map(|h| h.provider.clone());
                if let Some(handle) = provider_handle {
                    tokio::spawn(crate::effects::login::run(
                        f.provider,
                        f.key,
                        tx,
                        handle.clone(),
                    ));
                }
            }
        } else {
            let state_clone = ui.state.clone();
            tokio::spawn(async move {
                cmd.dispatch_async(&state_clone).await;
            });
        }
    }
}
