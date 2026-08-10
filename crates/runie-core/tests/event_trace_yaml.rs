use runie_core::replay_yaml;

#[test]
fn fixture_replays_through_public_yaml_event_api() {
    let trace = replay_yaml(
        include_str!("fixtures/event-trace.yaml"),
        10,
        |state: &mut i32, event: &i32| *state += event,
    )
    .expect("fixture is a valid ordered event trace");

    assert_eq!(trace.events(), &[2, 3, -4]);
    assert_eq!(trace.state(), &11);
}
