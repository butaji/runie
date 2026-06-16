use super::support::*;

#[test]
#[ignore = "e2e: requires release binary"]
fn e2e_session_persistence_no_panic() {
    let mut p = spawn_runie().expect("spawn runie");
    wait_for(&mut p, "Type a message to start").expect("welcome prompt");

    send_line(&mut p, "test message").expect("first message");
    std::thread::sleep(std::time::Duration::from_secs(2));

    send_line(&mut p, "another message").expect("second message");
    std::thread::sleep(std::time::Duration::from_secs(2));

    let output = capture_output(&mut p);
    assert_no_panic(&output);
    assert_no_stuck_timer(&output);
    assert!(
        output.contains("test message") && output.contains("another message"),
        "expected both messages in session output"
    );
}
