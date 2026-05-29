#[cfg(test)]
mod tests {
    use crate::actors::spawn_actor;
    use crate::actors::timer::TimerActor;
    use tokio::time::{timeout, Duration};

    #[tokio::test]
    async fn test_timer_actor_emits_ticks() {
        let (handle, mut rx) = spawn_actor(TimerActor::new(10));

        // Should receive at least one tick within 100ms
        let result = timeout(Duration::from_millis(100), rx.recv()).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());

        // Clean up
        handle.shutdown();
    }
}
