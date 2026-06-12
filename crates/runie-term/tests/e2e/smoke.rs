use super::support::*;

#[test]
#[ignore = "e2e: requires release binary"]
fn e2e_app_starts_without_panic() {
    let mut p = spawn_runie().expect("spawn runie");
    wait_for(&mut p, "Type a message to start").expect("welcome prompt");
    send_ctrl_c(&mut p).expect("ctrl-c");
}

#[test]
#[ignore = "e2e: requires release binary"]
fn e2e_command_palette_opens_and_searches() {
    let mut p = spawn_runie().expect("spawn runie");
    wait_for(&mut p, "Type a message to start").expect("welcome prompt");

    send(&mut p, "\x10").expect("ctrl-p palette"); // Ctrl+P
    wait_for(&mut p, "Commands").expect("palette title");

    send_line(&mut p, "save").expect("type filter");
    wait_for(&mut p, "save").expect("fuzzy search result");

    send_escape(&mut p).expect("close palette");
    send_ctrl_c(&mut p).expect("ctrl-c");
}

#[test]
#[ignore = "e2e: requires release binary"]
fn e2e_help_command_responds() {
    let mut p = spawn_runie().expect("spawn runie");
    wait_for(&mut p, "Type a message to start").expect("welcome prompt");

    send(&mut p, "\x10").expect("ctrl-p palette"); // Ctrl+P
    wait_for(&mut p, "Commands").expect("palette title");
    send_line(&mut p, "help").expect("type help");
    wait_for(&mut p, "/help, h, ?").expect("help content");

    send_ctrl_c(&mut p).expect("ctrl-c");
}

#[test]
#[ignore = "e2e: requires release binary"]
fn e2e_login_flow_opens_and_cancel_works() {
    let mut p = spawn_runie().expect("spawn runie");
    wait_for(&mut p, "Type a message to start").expect("welcome prompt");

    send(&mut p, "\x10").expect("ctrl-p palette"); // Ctrl+P
    wait_for(&mut p, "Commands").expect("palette title");
    send_line(&mut p, "login").expect("type login");
    wait_for(&mut p, "Login").expect("login dialog");

    send_line(&mut p, "").expect("select provider");
    wait_for(&mut p, "API Key").expect("key input form");

    send_escape(&mut p).expect("cancel");
    send_ctrl_c(&mut p).expect("ctrl-c");
}

#[test]
#[ignore = "e2e: requires release binary"]
fn e2e_rapid_submit_no_panic_or_stuck_timer() {
    let mut p = spawn_runie().expect("spawn runie");
    wait_for(&mut p, "Type a message to start").expect("welcome prompt");

    for _ in 0..3 {
        send_line(&mut p, "list files").expect("submit");
    }

    // Give the agent a moment to process, then assert no panic output.
    p.process_mut().set_kill_timeout(Some(2_000));
    p.process_mut().exit().ok();
    let output = p.exp_eof().unwrap_or_default();
    assert!(
        !output.to_lowercase().contains("panic"),
        "panic detected: {}",
        output
    );
    assert!(
        !output.contains("thread '"),
        "thread panic detected: {}",
        output
    );
}

#[test]
#[ignore = "e2e: requires release binary"]
fn e2e_dialog_flows_settings_model_theme_scoped() {
    let mut p = spawn_runie().expect("spawn runie");
    wait_for(&mut p, "Type a message to start").expect("welcome prompt");

    run_command(&mut p, "settings");
    wait_for(&mut p, "Models").expect("settings dialog");
    send_escape(&mut p).expect("close settings");
    wait_idle(&mut p);

    run_command(&mut p, "model");
    wait_for(&mut p, "Select").expect("model selector");
    send_line(&mut p, "gpt").expect("filter gpt");
    wait_for(&mut p, "gpt").expect("model fuzzy search");
    send_escape(&mut p).expect("close model");
    wait_idle(&mut p);

    run_command(&mut p, "theme");
    wait_for(&mut p, "runie").expect("theme picker");
    send_escape(&mut p).expect("close theme");
    wait_idle(&mut p);

    run_command(&mut p, "scoped-models");
    wait_for(&mut p, "Enable").expect("scoped models dialog");
    send_escape(&mut p).expect("close scoped models");
    wait_idle(&mut p);

    run_command(&mut p, "thinking");
    wait_for(&mut p, "off").expect("thinking selector");
    send_escape(&mut p).expect("close thinking");
    wait_idle(&mut p);

    send_ctrl_c(&mut p).expect("ctrl-c");
}
