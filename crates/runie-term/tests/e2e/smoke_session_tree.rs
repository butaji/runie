use super::support::*;

#[test]
#[ignore = "e2e: requires release binary"]
fn e2e_session_tree_operations_no_panic() {
    let mut p = spawn_runie().expect("spawn runie");
    wait_for(&mut p, "Type a message to start").expect("welcome prompt");

    send_line(&mut p, "message one").expect("first message");
    std::thread::sleep(std::time::Duration::from_secs(2));

    send_line(&mut p, "message two").expect("second message");
    std::thread::sleep(std::time::Duration::from_secs(2));

    // Open session tree via slash command.
    send_line(&mut p, "/tree").expect("open tree");
    std::thread::sleep(std::time::Duration::from_millis(500));
    // Cycle filter.
    send(&mut p, "f").expect("cycle filter");
    std::thread::sleep(std::time::Duration::from_millis(300));
    send_escape(&mut p).expect("close tree");
    std::thread::sleep(std::time::Duration::from_millis(300));

    // Fork at first message.
    send_line(&mut p, "/fork 1").expect("fork");
    std::thread::sleep(std::time::Duration::from_secs(1));

    let output = capture_output(&mut p);
    assert_no_panic(&output);
    assert_no_stuck_timer(&output);
}
