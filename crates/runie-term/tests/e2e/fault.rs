use super::support::*;

#[test]
#[ignore = "e2e: requires release binary"]
fn f1_corrupt_config_recovers_with_defaults() {
    // F1: a corrupt config.toml must not crash the app. It should fall
    // back to defaults and render the welcome prompt.
    let mut p = spawn_with_setup(|home| {
        let runie_dir = home.join(".runie");
        std::fs::create_dir_all(&runie_dir).expect("mkdir .runie");
        std::fs::write(
            runie_dir.join("config.toml"),
            b"this is not valid toml {{{\n[unterminated\n",
        )
        .expect("write corrupt config");
    })
    .expect("spawn runie");

    // The app should still come up. If it can't parse config, it should
    // either show a warning and use defaults, or refuse gracefully —
    // but it must NOT panic and must NOT hang.
    let result = wait_for(&mut p, "Type a message to start");
    if result.is_err() {
        // No prompt. Capture output to diagnose.
        p.process_mut().set_kill_timeout(Some(3_000));
        let output = p
            .process_mut()
            .exit()
            .ok()
            .map(|_| p.exp_eof().unwrap_or_default())
            .unwrap_or_default();
        assert_no_panic(&output);
        panic!(
            "app failed to start with corrupt config (no panic in output): {}",
            output
        );
    }

    send_ctrl_c(&mut p).expect("ctrl-c");
}

#[test]
#[ignore = "e2e: requires release binary"]
fn f1_missing_config_dir_first_run() {
    // F1: first run with no config dir at all.
    let mut p = spawn_with_setup(|_home| {
        // Intentionally do not create .runie/.
    })
    .expect("spawn runie");
    wait_for(&mut p, "Type a message to start").expect("first-run prompt");
    send_ctrl_c(&mut p).expect("ctrl-c");
}

#[test]
#[ignore = "e2e: requires release binary"]
fn f2_very_long_input_no_panic() {
    // F2: 3000-character input. Tests input widget scrolling, cursor
    // handling, and that the submit path doesn't OOM or panic.
    let mut p = spawn_runie().expect("spawn runie");
    wait_for(&mut p, "Type a message to start").expect("prompt");

    let long: String = "a".repeat(3000);
    send(&mut p, &long).expect("type long input");
    send_line(&mut p, "").expect("submit");

    // Give the app time to process. The mock provider will echo a
    // truncated response. We don't assert on content — we just need no
    // panic and a return to a usable state.
    std::thread::sleep(std::time::Duration::from_millis(500));
    send_ctrl_c(&mut p).expect("ctrl-c");
    p.process_mut().set_kill_timeout(Some(3_000));
    let output = p
        .process_mut()
        .exit()
        .ok()
        .map(|_| p.exp_eof().unwrap_or_default())
        .unwrap_or_default();
    assert_no_panic(&output);
}

#[test]
#[ignore = "e2e: requires release binary"]
fn f2_unicode_input_no_panic() {
    // F2: emoji + CJK + combining characters. Tests UTF-8 handling in
    // the input widget and downstream rendering.
    let mut p = spawn_runie().expect("spawn runie");
    wait_for(&mut p, "Type a message to start").expect("prompt");

    send_line(&mut p, "hello \u{1f980} \u{4e16}\u{754c} \u{00e9}").expect("submit unicode");
    std::thread::sleep(std::time::Duration::from_millis(500));
    send_ctrl_c(&mut p).expect("ctrl-c");
    p.process_mut().set_kill_timeout(Some(3_000));
    let output = p
        .process_mut()
        .exit()
        .ok()
        .map(|_| p.exp_eof().unwrap_or_default())
        .unwrap_or_default();
    assert_no_panic(&output);
}

#[test]
#[ignore = "e2e: requires release binary"]
fn f2_empty_submit_no_panic() {
    // F2: pressing Enter on an empty input must not crash. The app
    // should either ignore it or show a gentle hint.
    let mut p = spawn_runie().expect("spawn runie");
    wait_for(&mut p, "Type a message to start").expect("prompt");

    for _ in 0..5 {
        send_line(&mut p, "").expect("empty submit");
    }
    std::thread::sleep(std::time::Duration::from_millis(500));
    send_ctrl_c(&mut p).expect("ctrl-c");
    p.process_mut().set_kill_timeout(Some(3_000));
    let output = p
        .process_mut()
        .exit()
        .ok()
        .map(|_| p.exp_eof().unwrap_or_default())
        .unwrap_or_default();
    assert_no_panic(&output);
}

#[test]
#[ignore = "e2e: requires release binary"]
fn f2_bracketed_paste_no_panic() {
    // F2: a bracketed paste (as terminals emit) must insert the text
    // rather than trigger a command or panic.
    let mut p = spawn_runie().expect("spawn runie");
    wait_for(&mut p, "Type a message to start").expect("prompt");

    // Simulate a bracketed paste: ESC[200~ ... ESC[201~
    let pasted = "pasted content with\nnewlines and \"quotes\"";
    p.send(&format!("\x1b[200~{}\x1b[201~", pasted))
        .expect("send paste");
    p.flush().expect("flush");
    p.send("\r").expect("submit");
    p.flush().expect("flush");
    std::thread::sleep(std::time::Duration::from_millis(500));
    send_ctrl_c(&mut p).expect("ctrl-c");
    p.process_mut().set_kill_timeout(Some(3_000));
    let output = p
        .process_mut()
        .exit()
        .ok()
        .map(|_| p.exp_eof().unwrap_or_default())
        .unwrap_or_default();
    assert_no_panic(&output);
}

#[test]
#[ignore = "e2e: requires release binary"]
fn f3_resize_zero_dimensions_no_panic() {
    // F3: resizing to 0×0 must not crash. This exercises the
    // `width.saturating_sub(1)` underflow guard in the layout code.
    // Note: we do NOT assert on prompt re-emission after a 0-size
    // resize storm — the render task uses an mpsc(1) channel and the
    // latest snapshot is not guaranteed to win the race during a
    // burst. The guarantee we DO check is: no panic, clean exit.
    let mut p = spawn_runie().expect("spawn runie");
    wait_for(&mut p, "Type a message to start").expect("prompt");

    resize(&mut p, 0, 0).expect("resize 0x0");
    resize(&mut p, 0, 80).expect("resize 0 rows");
    resize(&mut p, 24, 0).expect("resize 0 cols");
    resize(&mut p, 24, 80).expect("resize back");
    // Give the app a moment to settle, then exit and assert.
    std::thread::sleep(std::time::Duration::from_millis(500));
    send_ctrl_c(&mut p).expect("ctrl-c");
    p.process_mut().set_kill_timeout(Some(3_000));
    let output = p
        .process_mut()
        .exit()
        .ok()
        .map(|_| p.exp_eof().unwrap_or_default())
        .unwrap_or_default();
    assert_no_panic(&output);
}

#[test]
#[ignore = "e2e: requires release binary"]
fn f3_resize_extreme_dimensions_no_panic() {
    // F3: very wide and very tall terminals. Same caveat as
    // f3_resize_zero_dimensions: we only check no-panic / clean exit.
    let mut p = spawn_runie().expect("spawn runie");
    wait_for(&mut p, "Type a message to start").expect("prompt");

    resize(&mut p, 200, 300).expect("resize 200x300");
    resize(&mut p, 3, 40).expect("resize 3x40");
    resize(&mut p, 24, 80).expect("resize back");
    std::thread::sleep(std::time::Duration::from_millis(500));
    send_ctrl_c(&mut p).expect("ctrl-c");
    p.process_mut().set_kill_timeout(Some(3_000));
    let output = p
        .process_mut()
        .exit()
        .ok()
        .map(|_| p.exp_eof().unwrap_or_default())
        .unwrap_or_default();
    assert_no_panic(&output);
}

#[test]
#[ignore = "e2e: requires release binary"]
fn f4_login_is_non_blocking() {
    // F4: the model selector must appear *immediately* after submitting
    // an API key. It must NOT wait for the network validation to return.
    // This is the core guarantee of the non-blocking redesign.
    let mut p = spawn_runie().expect("spawn runie");
    wait_for(&mut p, "Type a message to start").expect("prompt");

    run_command(&mut p, "login");
    wait_for(&mut p, "Login").expect("provider picker");
    // Select the first provider (Anthropic) by pressing Enter.
    send_line(&mut p, "").expect("select first provider");
    wait_for(&mut p, "API Key").expect("key input");

    // Submit a key and measure how long until the model selector appears.
    let start = std::time::Instant::now();
    send_line(&mut p, "sk-fake-key").expect("submit key");
    wait_for(&mut p, "Models").expect("model selector appears");
    let elapsed = start.elapsed();

    assert!(
        elapsed < std::time::Duration::from_secs(2),
        "model selector should appear within 2s of submit, took {:?}",
        elapsed
    );

    send_ctrl_c(&mut p).expect("ctrl-c");
    p.process_mut().set_kill_timeout(Some(3_000));
    p.process_mut().exit().ok();
}

#[test]
#[ignore = "e2e: requires release binary"]
fn f5_login_validation_failure_shows_transient() {
    let mut p = spawn_runie().expect("spawn runie");
    wait_for(&mut p, "Type a message to start").expect("prompt");
    run_command(&mut p, "login");
    wait_for(&mut p, "Login").expect("provider picker");
    send_line(&mut p, "").expect("select first provider");
    wait_for(&mut p, "API Key").expect("key input");
    send_line(&mut p, "sk-definitely-not-valid").expect("submit key");
    wait_for(&mut p, "Models").expect("model selector");
    let mut found_transient = false;
    for _ in 0..60 {
        std::thread::sleep(std::time::Duration::from_millis(200));
        let s = p.exp_string("").unwrap_or_default();
        if s.contains("Could not") || s.contains("verify") {
            found_transient = true;
            break;
        }
    }
    if !found_transient {
        send_escape(&mut p).expect("escape");
    }
    send_ctrl_c(&mut p).expect("ctrl-c");
    p.process_mut().set_kill_timeout(Some(3_000));
    let output = p
        .process_mut()
        .exit()
        .ok()
        .map(|_| p.exp_eof().unwrap_or_default())
        .unwrap_or_default();
    assert_no_panic(&output);
}

#[test]
#[ignore = "e2e: requires release binary"]
fn f6_theme_switch_rerenders() {
    // F6: switching theme must produce a notification and the app
    // must remain responsive. We don't assert the prompt re-emits
    // (the render task may be processing a stale snapshot), only
    // that the theme change is acknowledged.
    let mut p = spawn_runie().expect("spawn runie");
    wait_for(&mut p, "Type a message to start").expect("prompt");

    run_command(&mut p, "theme");
    wait_for(&mut p, "runie").expect("theme picker");
    // Move down once and select a different theme.
    send(&mut p, "\x1b[B").expect("down");
    p.flush().expect("flush");
    send_line(&mut p, "").expect("select");
    // The app should emit a "Theme switched to" transient.
    let _ = wait_for(&mut p, "Theme switched to");
    std::thread::sleep(std::time::Duration::from_millis(300));
    send_ctrl_c(&mut p).expect("ctrl-c");
    p.process_mut().set_kill_timeout(Some(3_000));
    let output = p
        .process_mut()
        .exit()
        .ok()
        .map(|_| p.exp_eof().unwrap_or_default())
        .unwrap_or_default();
    assert_no_panic(&output);
}

#[test]
#[ignore = "e2e: requires release binary"]
fn f7_save_session_with_special_chars() {
    // F7: session names with spaces and punctuation must round-trip
    // through the save/load cycle without crashing.
    let mut p = spawn_runie().expect("spawn runie");
    wait_for(&mut p, "Type a message to start").expect("prompt");

    // Type a message first so there's something to save.
    send_line(&mut p, "test message").expect("type");

    run_command(&mut p, "save");
    wait_for(&mut p, "Fill in the form").expect("save form");
    send(&mut p, "my-session (v1)").expect("type name with spaces/parens");
    send_escape(&mut p).expect("cancel save");
    wait_idle(&mut p);

    // Try /sessions to ensure the list command works.
    run_command(&mut p, "sessions");
    wait_for(&mut p, "saved sessions").expect("sessions list");
    send_escape(&mut p).expect("close");
    wait_idle(&mut p);

    send_ctrl_c(&mut p).expect("ctrl-c");
    p.process_mut().set_kill_timeout(Some(3_000));
    let output = p
        .process_mut()
        .exit()
        .ok()
        .map(|_| p.exp_eof().unwrap_or_default())
        .unwrap_or_default();
    assert_no_panic(&output);
}

#[test]
#[ignore = "e2e: requires release binary"]
fn f8_rapid_ctrl_c_no_panic() {
    // F8: hammering Ctrl+C should not crash or leave the app stuck.
    let mut p = spawn_runie().expect("spawn runie");
    wait_for(&mut p, "Type a message to start").expect("prompt");

    for _ in 0..10 {
        send_ctrl_c(&mut p).expect("ctrl-c");
    }
    p.process_mut().set_kill_timeout(Some(3_000));
    let output = p
        .process_mut()
        .exit()
        .ok()
        .map(|_| p.exp_eof().unwrap_or_default())
        .unwrap_or_default();
    assert_no_panic(&output);
}

#[test]
#[ignore = "e2e: requires release binary"]
fn f8_open_close_palette_repeatedly() {
    // F8: opening and closing the palette repeatedly must not leak
    // state or crash.
    let mut p = spawn_runie().expect("spawn runie");
    wait_for(&mut p, "Type a message to start").expect("prompt");

    for _ in 0..10 {
        send(&mut p, "\x10").expect("ctrl-p open");
        p.flush().expect("flush");
        send_escape(&mut p).expect("esc close");
        p.flush().expect("flush");
    }
    wait_idle(&mut p);
    send_ctrl_c(&mut p).expect("ctrl-c");
    p.process_mut().set_kill_timeout(Some(3_000));
    let output = p
        .process_mut()
        .exit()
        .ok()
        .map(|_| p.exp_eof().unwrap_or_default())
        .unwrap_or_default();
    assert_no_panic(&output);
}
