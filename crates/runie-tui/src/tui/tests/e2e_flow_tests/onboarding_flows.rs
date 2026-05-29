use super::*;

#[test]
fn test_e2e_onboarding_enter() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    // Enter onboarding
    update(&mut state, &mut palette, Msg::EnterOnboarding);
    assert_eq!(state.mode, TuiMode::Onboarding);
    assert!(state.onboarding.is_some());
}

#[test]
fn test_e2e_onboarding_skip_exits() {
    let mut state = make_state();
    let mut palette = CommandPalette::new();

    // Enter onboarding
    update(&mut state, &mut palette, Msg::EnterOnboarding);
    assert_eq!(state.mode, TuiMode::Onboarding);

    // Skip onboarding
    update(&mut state, &mut palette, Msg::OnboardingSkip);
    assert_eq!(state.mode, TuiMode::Chat);
}
