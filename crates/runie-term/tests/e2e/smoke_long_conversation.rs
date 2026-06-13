use super::support::*;
use std::time::Instant;

#[test]
#[ignore = "e2e: requires release binary"]
fn e2e_long_conversation_stays_responsive() {
    let mut p = spawn_runie().expect("spawn runie");
    wait_for(&mut p, "Type a message to start").expect("welcome prompt");

    let start = Instant::now();
    for i in 1..=10 {
        send_line(&mut p, &format!("msg{i}")).expect("send message");
        if i % 3 == 0 {
            // Give the UI a moment to settle.
            std::thread::sleep(std::time::Duration::from_millis(300));
        }
    }

    std::thread::sleep(std::time::Duration::from_secs(3));
    let output = capture_output(&mut p);
    let elapsed = start.elapsed().as_secs();

    assert_no_panic(&output);
    assert_no_stuck_timer(&output);
    assert!(
        elapsed < 60,
        "long conversation took {elapsed}s, expected < 60s"
    );
}
