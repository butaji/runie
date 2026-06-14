use super::support::*;

fn sleep(ms: u64) {
    std::thread::sleep(std::time::Duration::from_millis(ms));
}

fn open_and_run(p: &mut IsolatedSession, name: &str) {
    run_command(p, name);
    sleep(800);
}

fn pop_to_palette(p: &mut IsolatedSession, expected: &str) {
    send_escape(p).expect("esc pop to palette");
    sleep(500);
    wait_for(p, expected).expect("back at palette with preserved filter");
}

fn close_palette_and_idle(p: &mut IsolatedSession) {
    send_escape(p).expect("esc on main menu closes");
    sleep(300);
    wait_idle(p);
}

fn exit_no_panic(p: &mut IsolatedSession) {
    send_ctrl_c(p).expect("ctrl-c");
    let output = capture_output(p);
    assert_no_panic(&output);
}

#[test]
#[ignore = "e2e: requires release binary"]
fn f8b_esc_on_command_bar_main_menu_closes() {
    let mut p = spawn_runie().expect("spawn runie");
    wait_for(&mut p, "Type a message to start").expect("welcome prompt");

    send(&mut p, "\x10").expect("ctrl-p open");
    sleep(300);
    wait_for(&mut p, "Commands").expect("palette title");

    close_palette_and_idle(&mut p);
    exit_no_panic(&mut p);
}

#[test]
#[ignore = "e2e: requires release binary"]
fn f9_global_back_stack_palette_pushes_subdialog_esc_pops_then_closes() {
    let mut p = spawn_runie().expect("spawn runie");
    wait_for(&mut p, "Type a message to start").expect("welcome prompt");

    open_and_run(&mut p, "scoped-models");
    pop_to_palette(&mut p, "scoped-models");
    close_palette_and_idle(&mut p);
    exit_no_panic(&mut p);
}

#[test]
#[ignore = "e2e: requires release binary"]
fn f10_ctrl_p_model_esc_pops_to_palette_not_closes() {
    let mut p = spawn_runie().expect("spawn runie");
    wait_for(&mut p, "Type a message to start").expect("welcome prompt");

    open_and_run(&mut p, "model");
    pop_to_palette(&mut p, "model");

    send_line(&mut p, "model").unwrap_or_else(|_| panic!("model again"));
    sleep(800);
    pop_to_palette(&mut p, "model");

    close_palette_and_idle(&mut p);
    exit_no_panic(&mut p);
}

#[test]
#[ignore = "e2e: requires release binary"]
fn f10b_ctrl_p_settings_esc_pops_to_palette_not_closes() {
    let mut p = spawn_runie().expect("spawn runie");
    wait_for(&mut p, "Type a message to start").expect("welcome prompt");

    open_and_run(&mut p, "settings");
    pop_to_palette(&mut p, "settings");

    send_line(&mut p, "settings").unwrap_or_else(|_| panic!("settings again"));
    sleep(800);
    pop_to_palette(&mut p, "settings");

    close_palette_and_idle(&mut p);
    exit_no_panic(&mut p);
}

#[test]
#[ignore = "e2e: requires release binary"]
fn f10c_ctrl_p_save_form_esc_pops_to_palette() {
    let mut p = spawn_runie().expect("spawn runie");
    wait_for(&mut p, "Type a message to start").expect("welcome prompt");

    open_and_run(&mut p, "save");
    pop_to_palette(&mut p, "save");

    send_line(&mut p, "save").unwrap_or_else(|_| panic!("save again"));
    sleep(800);
    pop_to_palette(&mut p, "save");

    close_palette_and_idle(&mut p);
    exit_no_panic(&mut p);
}

#[test]
#[ignore = "e2e: requires release binary"]
fn f10d_ctrl_p_providers_esc_pops_to_palette() {
    let mut p = spawn_runie().expect("spawn runie");
    wait_for(&mut p, "Type a message to start").expect("welcome prompt");

    open_and_run(&mut p, "providers");
    wait_for(&mut p, "No providers").expect("providers dialog");

    send_line(&mut p, "").expect("select Add provider");
    sleep(500);
    wait_for(&mut p, "Login").expect("login dialog");

    pop_to_palette(&mut p, "providers");

    send_line(&mut p, "providers").unwrap_or_else(|_| panic!("providers again"));
    sleep(500);
    wait_for(&mut p, "No providers").expect("providers dialog again");

    pop_to_palette(&mut p, "providers");
    close_palette_and_idle(&mut p);
    exit_no_panic(&mut p);
}

#[test]
#[ignore = "e2e: requires release binary"]
fn f10e_ctrl_p_name_form_esc_pops_to_palette() {
    let mut p = spawn_runie().expect("spawn runie");
    wait_for(&mut p, "Type a message to start").expect("welcome prompt");

    open_and_run(&mut p, "name");
    pop_to_palette(&mut p, "name");

    send_line(&mut p, "name").unwrap_or_else(|_| panic!("name again"));
    sleep(800);
    pop_to_palette(&mut p, "name");

    close_palette_and_idle(&mut p);
    exit_no_panic(&mut p);
}
