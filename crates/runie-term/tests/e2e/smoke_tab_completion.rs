use super::support::*;

#[test]
#[ignore = "e2e: requires release binary"]
fn e2e_tab_file_completion_no_panic() {
    let mut p = spawn_runie().expect("spawn runie");
    wait_for(&mut p, "Type a message to start").expect("welcome prompt");

    // Simple prefix completion.
    send(&mut p, "Cargo").expect("type Cargo");
    send_tab(&mut p).expect("first tab");
    std::thread::sleep(std::time::Duration::from_millis(300));
    send_tab(&mut p).expect("second tab");
    std::thread::sleep(std::time::Duration::from_millis(300));
    send_ctrl_u(&mut p).expect("clear line");
    std::thread::sleep(std::time::Duration::from_millis(200));

    // Folder completion.
    send(&mut p, "crate").expect("type crate");
    send_tab(&mut p).expect("tab");
    std::thread::sleep(std::time::Duration::from_millis(300));
    send_tab(&mut p).expect("accept");
    std::thread::sleep(std::time::Duration::from_millis(200));
    send_ctrl_u(&mut p).expect("clear");
    std::thread::sleep(std::time::Duration::from_millis(200));

    // Relative path.
    send(&mut p, "./scr").expect("type ./scr");
    send_tab(&mut p).expect("tab");
    std::thread::sleep(std::time::Duration::from_millis(300));
    send_tab(&mut p).expect("complete");
    std::thread::sleep(std::time::Duration::from_millis(200));
    send_ctrl_u(&mut p).expect("clear");
    std::thread::sleep(std::time::Duration::from_millis(200));

    // Multiple cycles.
    send(&mut p, "C").expect("type C");
    for _ in 0..4 {
        send_tab(&mut p).expect("cycle");
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
    send_ctrl_u(&mut p).expect("clear");
    std::thread::sleep(std::time::Duration::from_millis(200));

    // Tab on empty input.
    send_tab(&mut p).expect("tab empty");
    std::thread::sleep(std::time::Duration::from_millis(300));

    // Tab with no match.
    send(&mut p, "zzzzzzzxyz").expect("type no-match");
    send_tab(&mut p).expect("tab no-match");
    std::thread::sleep(std::time::Duration::from_millis(300));
    send_ctrl_u(&mut p).expect("clear");
    std::thread::sleep(std::time::Duration::from_millis(200));

    // Submit with ghost completion.
    send(&mut p, "Cargo").expect("type Cargo");
    send_tab(&mut p).expect("tab");
    std::thread::sleep(std::time::Duration::from_millis(300));
    send_line(&mut p, "").expect("submit");
    std::thread::sleep(std::time::Duration::from_secs(1));

    // Resize stress during completion.
    for i in 1..=5 {
        resize(&mut p, 20 + i, 60 + i * 4).expect("resize");
        send_tab(&mut p).expect("tab after resize");
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    // Rapid tab submissions.
    send(&mut p, "crate").expect("type crate");
    send_tab(&mut p).expect("tab");
    send_line(&mut p, "").expect("submit");
    std::thread::sleep(std::time::Duration::from_secs(1));

    send(&mut p, "C").expect("type C");
    send_tab(&mut p).expect("tab");
    send_line(&mut p, "").expect("submit");
    std::thread::sleep(std::time::Duration::from_secs(1));

    let output = capture_output(&mut p);
    assert_no_panic(&output);
    assert_no_stuck_timer(&output);
}
