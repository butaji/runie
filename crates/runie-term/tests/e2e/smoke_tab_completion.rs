use super::support::*;

fn sleep(ms: u64) {
    std::thread::sleep(std::time::Duration::from_millis(ms));
}

fn spawn_and_wait() -> IsolatedSession {
    let mut p = spawn_runie().expect("spawn runie");
    wait_for(&mut p, "Type a message to start").expect("welcome prompt");
    p
}

fn tab_twice(p: &mut IsolatedSession) {
    send_tab(p).expect("first tab");
    sleep(300);
    send_tab(p).expect("second tab");
    sleep(300);
}

fn clear_line(p: &mut IsolatedSession) {
    send_ctrl_u(p).expect("clear line");
    sleep(200);
}

fn assert_clean_shutdown(p: &mut IsolatedSession) {
    let output = capture_output(p);
    assert_no_panic(&output);
    assert_no_stuck_timer(&output);
}

#[test]
#[ignore = "e2e: requires release binary"]
fn e2e_tab_simple_prefix_completion() {
    let mut p = spawn_and_wait();
    send(&mut p, "Cargo").expect("type Cargo");
    tab_twice(&mut p);
    clear_line(&mut p);
}

#[test]
#[ignore = "e2e: requires release binary"]
fn e2e_tab_folder_completion() {
    let mut p = spawn_and_wait();
    send(&mut p, "crate").expect("type crate");
    tab_twice(&mut p);
    clear_line(&mut p);
}

#[test]
#[ignore = "e2e: requires release binary"]
fn e2e_tab_relative_path_completion() {
    let mut p = spawn_and_wait();
    send(&mut p, "./scr").expect("type ./scr");
    tab_twice(&mut p);
    clear_line(&mut p);
}

#[test]
#[ignore = "e2e: requires release binary"]
fn e2e_tab_multiple_cycles() {
    let mut p = spawn_and_wait();
    send(&mut p, "C").expect("type C");
    for _ in 0..4 {
        send_tab(&mut p).expect("cycle");
        sleep(200);
    }
    clear_line(&mut p);
}

#[test]
#[ignore = "e2e: requires release binary"]
fn e2e_tab_empty_input() {
    let mut p = spawn_and_wait();
    send_tab(&mut p).expect("tab empty");
    sleep(300);
}

#[test]
#[ignore = "e2e: requires release binary"]
fn e2e_tab_no_match() {
    let mut p = spawn_and_wait();
    send(&mut p, "zzzzzzzxyz").expect("type no-match");
    send_tab(&mut p).expect("tab no-match");
    sleep(300);
    clear_line(&mut p);
}

#[test]
#[ignore = "e2e: requires release binary"]
fn e2e_tab_submit_with_ghost_completion() {
    let mut p = spawn_and_wait();
    send(&mut p, "Cargo").expect("type Cargo");
    send_tab(&mut p).expect("tab");
    sleep(300);
    send_line(&mut p, "").expect("submit");
    sleep(1000);
    assert_clean_shutdown(&mut p);
}

#[test]
#[ignore = "e2e: requires release binary"]
fn e2e_tab_resize_stress() {
    let mut p = spawn_and_wait();
    send(&mut p, "Cargo").expect("type Cargo");
    send_tab(&mut p).expect("tab");
    sleep(300);

    for i in 1..=5 {
        resize(&mut p, 20 + i, 60 + i * 4).expect("resize");
        send_tab(&mut p).expect("tab after resize");
        sleep(100);
    }
    assert_clean_shutdown(&mut p);
}

#[test]
#[ignore = "e2e: requires release binary"]
fn e2e_tab_rapid_submissions() {
    for (prefix, wait_secs) in [("crate", 1), ("C", 1)] {
        let mut p = spawn_and_wait();
        send(&mut p, prefix).expect("type prefix");
        send_tab(&mut p).expect("tab");
        send_line(&mut p, "").expect("submit");
        sleep(wait_secs * 1000);
        assert_clean_shutdown(&mut p);
    }
}
