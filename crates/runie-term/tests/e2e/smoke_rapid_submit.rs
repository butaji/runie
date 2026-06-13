use super::support::*;

#[test]
#[ignore = "e2e: requires release binary"]
fn e2e_rapid_submit_stress() {
    let mut p = spawn_runie().expect("spawn runie");
    wait_for(&mut p, "Type a message to start").expect("welcome prompt");

    for i in 1..=5 {
        send_line(&mut p, &format!("message {i}")).expect("rapid submit");
        std::thread::sleep(std::time::Duration::from_millis(300));
    }

    std::thread::sleep(std::time::Duration::from_secs(2));
    let output = capture_output(&mut p);
    assert_no_panic(&output);
    assert_no_stuck_timer(&output);
}
