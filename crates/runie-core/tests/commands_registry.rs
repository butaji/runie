use runie_core::commands::PI_BUILTIN_SLASH_COMMANDS;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct FixtureCommand {
    name: String,
    description: String,
    argument_hint: Option<String>,
}

#[test]
fn yaml_registry_matches_pi_source_contract_without_recompiling_cases() {
    let fixture: Vec<FixtureCommand> =
        serde_yaml::from_str(include_str!("fixtures/pi-slash-commands.yaml"))
            .expect("valid slash-command fixture");

    assert_eq!(fixture.len(), PI_BUILTIN_SLASH_COMMANDS.len());
    for (expected, actual) in fixture.iter().zip(PI_BUILTIN_SLASH_COMMANDS) {
        assert_eq!(expected.name, actual.name);
        assert_eq!(expected.description, actual.description);
        assert_eq!(expected.argument_hint.as_deref(), actual.argument_hint);
    }
}
