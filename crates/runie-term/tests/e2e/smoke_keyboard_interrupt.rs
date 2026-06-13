use super::support::*;

#[test]
#[ignore = "e2e: requires release binary"]
fn e2e_keyboard_interrupt_exits_gracefully() {
    let mut p = spawn_runie().expect("spawn runie");
    wait_for(&mut p, "Type a message to start").expect("welcome prompt");

    send(&mut p, "hello").expect("type hello");
    send_ctrl_c(&mut p).expect("ctrl-c");

    // The app should shut down; read whatever is left on the PTY.
    let output = capture_output(&mut p);
    assert_no_panic(&output);
}
