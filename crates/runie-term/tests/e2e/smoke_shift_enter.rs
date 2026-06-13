use super::support::*;

#[test]
#[ignore = "e2e: requires release binary"]
fn e2e_shift_enter_inserts_newline() {
    let mut p = spawn_runie().expect("spawn runie");
    wait_for(&mut p, "Type a message to start").expect("welcome prompt");

    send(&mut p, "line1").expect("type line1");
    // Send the kitty-keyboard Shift+Enter sequence directly. Terminals that
    // support modified-key reporting send this for Shift+Enter; crossterm
    // parses it as Enter with the SHIFT modifier and maps it to Newline.
    send_csi(&mut p, "13;2u").expect("shift+enter");
    send(&mut p, "line2").expect("type line2");
    std::thread::sleep(std::time::Duration::from_millis(300));

    let output = capture_output(&mut p);
    assert_no_panic(&output);
    assert!(
        output.contains("line1") && output.contains("line2"),
        "expected both lines in input area: {}",
        output
    );
}
