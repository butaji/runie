use super::support::*;

#[test]
#[ignore = "e2e: requires release binary"]
fn e2e_basic_interaction_responds_without_panic() {
    let mut p = spawn_runie().expect("spawn runie");
    wait_for(&mut p, "Type a message to start").expect("welcome prompt");

    send_line(&mut p, "hello").expect("send hello");
    std::thread::sleep(std::time::Duration::from_secs(3));

    let output = capture_output(&mut p);
    assert_no_panic(&output);
    assert_no_stuck_timer(&output);
    assert!(
        output.contains("hello"),
        "expected submitted text in output"
    );
}
