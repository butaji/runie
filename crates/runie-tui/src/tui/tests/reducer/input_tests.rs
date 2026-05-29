use super::*;

#[test]
fn test_textarea_input() {
    let mut state = make_state();
    type_char(&mut state, 'h');
    type_char(&mut state, 'i');
    assert_eq!(state.textarea.lines(), &["hi".to_string()]);
}

#[test]
fn test_quit() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();
    update(&mut state, &mut palette, Msg::Quit);
    assert!(!state.running);
}

#[test]
fn test_toggle_sidebar() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();
    assert!(!state.show_sidebar);
    update(&mut state, &mut palette, Msg::ToggleSidebar);
    assert!(state.show_sidebar);
    update(&mut state, &mut palette, Msg::ToggleSidebar);
    assert!(!state.show_sidebar);
}
