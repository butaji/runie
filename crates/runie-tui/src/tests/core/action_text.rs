use runie_core::labels::action_text;

#[test]
fn working_tag_gets_ellipsis() {
    let out = action_text('⠙', "Working", 1.5);
    assert!(
        out.contains("Working..."),
        "-ing tag must have '...': got {}",
        out
    );
}

#[test]
fn thinking_tag_gets_ellipsis() {
    let out = action_text('⠹', "Thinking", 0.0);
    assert!(
        out.contains("Thinking..."),
        "-ing tag must have '...': got {}",
        out
    );
}

#[test]
fn running_tag_gets_ellipsis() {
    let out = action_text('⠸', "Running", 5.5);
    assert!(
        out.contains("Running..."),
        "-ing tag must have '...': got {}",
        out
    );
}

#[test]
fn non_ing_tag_no_ellipsis() {
    let out = action_text('⠼', "Done", 1.0);
    assert!(
        !out.contains("Done..."),
        "Non-ing tag must NOT have '...': got {}",
        out
    );
    assert!(
        out.contains("Done "),
        "Non-ing tag should have plain space: got {}",
        out
    );
}

#[test]
fn action_text_includes_spinner() {
    // Use '⠷' (throbber BRAILLE_SIX[0]) as a representative braille spinner frame.
    let out = action_text('⠷', "Working", 1.5);
    assert!(
        out.starts_with('⠷'),
        "Must start with spinner frame: got {}",
        out
    );
}

#[test]
fn action_text_includes_timer() {
    let out = action_text('⠦', "Working", 2.3);
    assert!(
        out.contains("2.3s"),
        "Must contain elapsed timer: got {}",
        out
    );
}

#[test]
fn action_text_zero_elapsed() {
    let out = action_text('⠧', "Thinking", 0.0);
    assert_eq!(out, "⠧ Thinking... 0.0s");
}

#[test]
fn action_text_full_format() {
    assert_eq!(action_text('⠙', "Working", 10.0), "⠙ Working... 10.0s");
    assert_eq!(action_text('⠹', "Running", 999.9), "⠹ Running... 999.9s");
}

#[test]
fn action_text_non_ing_format() {
    assert_eq!(action_text('◆', "Ran", 0.5), "◆ Ran 0.5s");
}
