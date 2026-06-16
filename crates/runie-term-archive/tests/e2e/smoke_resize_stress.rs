use super::support::*;

#[test]
#[ignore = "e2e: requires release binary"]
fn e2e_resize_stress_no_panic() {
    let mut p = spawn_runie().expect("spawn runie");
    wait_for(&mut p, "Type a message to start").expect("welcome prompt");

    for i in 1..=20 {
        let cols = 40 + i * 2;
        let rows = 10 + i;
        resize(&mut p, rows, cols).expect("resize");
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    resize(&mut p, 30, 120).expect("final resize");
    std::thread::sleep(std::time::Duration::from_millis(300));

    let output = capture_output(&mut p);
    assert_no_panic(&output);
}
