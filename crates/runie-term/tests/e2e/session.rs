use super::support::*;

#[test]
#[ignore = "e2e: requires release binary"]
fn e2e_session_form_dialogs_open_and_cancel() {
    let mut p = spawn_runie().expect("spawn runie");
    wait_for(&mut p, "Type a message to start").expect("welcome prompt");
    open_cancel(&mut p, "save", "Fill in the form");
    open_cancel(&mut p, "load", "Fill in the form");
    open_cancel(&mut p, "delete", "Fill in the form");
    open_cancel(&mut p, "export", "Path");
    open_cancel(&mut p, "import", "Path");
    open_cancel(&mut p, "prompt", "Prompt name");
    open_cancel(&mut p, "name", "Fill in the form");
    open_cancel(&mut p, "fork", "Message index");
    open_cancel(&mut p, "compact", "Keep tokens");
    send_ctrl_c(&mut p).expect("ctrl-c");
}

#[test]
#[ignore = "e2e: requires release binary"]
fn e2e_session_commands_no_panic() {
    let mut p = spawn_runie().expect("spawn runie");
    wait_for(&mut p, "Type a message to start").expect("welcome prompt");
    run_wait(&mut p, "new", "started");
    run_wait(&mut p, "clone", "cloned");
    run_wait(&mut p, "sessions", "saved sessions");
    run_wait(&mut p, "history", "history");
    run_wait(&mut p, "session", "Messages:");
    open_cancel(&mut p, "tree", "Tree");
    run_wait(&mut p, "skills", "skills");
    run_command(&mut p, "diagnostics");
    wait_idle(&mut p);
    run_command(&mut p, "reload");
    wait_idle(&mut p);
    run_wait(&mut p, "reset", "cleared");
    run_wait(&mut p, "logout", "providers configured");
    send_ctrl_c(&mut p).expect("ctrl-c");
}
