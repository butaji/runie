#[tokio::test]
async fn prompt_actor_publishes_shared_snapshot_data() {
    let bus = runie_core::events::EventBus::new();
    let actor = PromptActor::new(&bus);
    actor.set_model_caption("shared-model".into()).await;
    let shared = actor.shared_model_snapshot();
    assert_eq!(shared.model_caption, "shared-model");
    assert_eq!(shared, actor.shared_model_snapshot());
}
