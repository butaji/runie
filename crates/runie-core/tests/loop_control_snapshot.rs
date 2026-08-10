//! Loop control mailbox acknowledgement and snapshot ordering coverage.

mod common;

use std::sync::Arc;

use common::{MockStreamFn, TestLoopBuilder};
use runie_core::r#loop::LoopControlSnapshot;
use runie_core::types::QueueMode;

#[tokio::test]
async fn control_setters_publish_snapshot_before_returning() {
    let test = TestLoopBuilder::new(Arc::new(MockStreamFn::hello())).build();
    assert_eq!(
        test.actor.control_snapshot(),
        LoopControlSnapshot {
            running: false,
            abort_requested: false,
            steering_mode: QueueMode::OneAtATime,
            follow_up_mode: QueueMode::OneAtATime,
        }
    );
    test.actor.set_steering_mode(QueueMode::All).await;
    let after_steering = test.actor.control_snapshot();
    assert_eq!(after_steering.steering_mode, QueueMode::All);
    assert_eq!(after_steering.follow_up_mode, QueueMode::OneAtATime);
    assert!(!after_steering.running);
    assert!(!after_steering.abort_requested);
    test.actor.set_follow_up_mode(QueueMode::All).await;
    let after_follow_up = test.actor.control_snapshot();
    assert_eq!(after_follow_up.steering_mode, QueueMode::All);
    assert_eq!(after_follow_up.follow_up_mode, QueueMode::All);
    assert!(!after_follow_up.running);
    assert!(!after_follow_up.abort_requested);
    test.actor.abort().await;
    let after_abort = test.actor.control_snapshot();
    assert!(after_abort.abort_requested);
    assert!(!after_abort.running);
    assert_eq!(after_abort.steering_mode, QueueMode::All);
    assert_eq!(after_abort.follow_up_mode, QueueMode::All);
}
