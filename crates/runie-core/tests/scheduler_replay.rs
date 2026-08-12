use runie_core::replay_yaml;
use runie_core::tools::{reduce_scheduler_event, SchedulerEvent, SchedulerMetrics};

#[test]
fn scheduler_fixture_replays_queue_and_cancellation_lifecycle() {
    let trace = replay_yaml::<SchedulerMetrics, SchedulerEvent>(
        include_str!("fixtures/scheduler-trace.yaml"),
        SchedulerMetrics::default(),
        |state: &mut SchedulerMetrics, event: &SchedulerEvent| {
            let _ = reduce_scheduler_event(state, event.clone());
        },
    )
    .expect("scheduler fixture is a valid ordered trace");
    assert_eq!(trace.state().queued, 0);
    assert_eq!(trace.state().running, 0);
    assert_eq!(trace.state().cancelled_running, 1);
    assert_eq!(trace.state().cancelled_queued, 1);
}
