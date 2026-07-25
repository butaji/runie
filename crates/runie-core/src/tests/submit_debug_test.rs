#[cfg(test)]
mod submit_debug_tests {
    use runie_core::actors::input::{InputMsg, InputActor};
    use runie_core::bus::EventBus;
    use runie_core::event::Event;

    #[tokio::test]
    async fn input_actor_publishes_input_changed_after_submit() {
        let bus = EventBus::<Event>::new(100);
        let mut rx1 = bus.subscribe();
        let mut rx2 = bus.subscribe();
        
        // Start InputActor
        let handle = InputActor::start_default(bus.clone()).await.unwrap();
        
        // Type some text first
        handle.send_message(InputMsg::InsertChar('x')).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        
        // Drain InsertChar events
        while let Ok(evt) = rx1.try_recv() {
            eprintln!("rx1 pre-submit: {:?}", std::mem::discriminant(&evt));
        }
        while let Ok(evt) = rx2.try_recv() {
            eprintln!("rx2 pre-submit: {:?}", std::mem::discriminant(&evt));
        }
        
        // Submit
        handle.send_message(InputMsg::Submit { content: "x".into() }).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        
        // Read events
        let mut found = false;
        while let Ok(evt) = rx1.try_recv() {
            eprintln!("rx1 event: {:?}", std::mem::discriminant(&evt));
            if matches!(&evt, Event::InputChanged { .. }) {
                found = true;
            }
        }
        while let Ok(evt) = rx2.try_recv() {
            eprintln!("rx2 event: {:?}", std::mem::discriminant(&evt));
        }
        
        assert!(found, "InputChanged should be published after Submit - got no InputChanged on rx1");
        
        handle.shutdown().await;
        eprintln!("TEST PASSED: InputActor published InputChanged after Submit");
    }
}
