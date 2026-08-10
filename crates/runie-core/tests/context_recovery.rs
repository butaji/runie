mod common;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use common::{MockStreamFn, TestLoopBuilder};
use runie_core::types::AgentContext;

#[tokio::test]
async fn context_recovery_hook_runs_before_provider_turn() {
    let calls = Arc::new(AtomicUsize::new(0));
    let observed = calls.clone();
    let recovery: runie_core::r#loop::driver::ContextRecoveryHook =
        Arc::new(move |context, model| {
            observed.fetch_add(1, Ordering::SeqCst);
            assert_eq!(model.id, "test-model");
            Box::pin(async move { Ok(context) })
        });
    let test = TestLoopBuilder::new(Arc::new(MockStreamFn::hello()))
        .context_recovery(recovery)
        .build();
    test.actor.set_model(common::default_model()).await;
    test.actor
        .prompt(vec![], AgentContext::default())
        .await
        .expect("prompt should use recovery hook");
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}
