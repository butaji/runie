//! Runtime-discovered YAML replay tests for actor-owned telemetry.

use std::path::PathBuf;

use runie_core::telemetry::{TelemetryActor, TelemetryScenario};

#[tokio::test]
async fn every_telemetry_yaml_replays_to_its_declared_snapshot() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("telemetry");
    let mut fixtures: Vec<_> = std::fs::read_dir(&root)
        .expect("telemetry fixture directory")
        .map(|entry| entry.expect("telemetry fixture entry").path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "yaml")
        })
        .collect();
    fixtures.sort();
    assert!(!fixtures.is_empty(), "no telemetry YAML fixtures found");

    for path in fixtures {
        let source = std::fs::read_to_string(&path).expect("telemetry fixture source");
        let scenario: TelemetryScenario =
            serde_yaml::from_str(&source).expect("telemetry fixture YAML");
        let actor = TelemetryActor::new();
        actor.replay(scenario.actions).await;
        assert_eq!(
            actor.snapshot(),
            scenario.expected,
            "telemetry replay mismatch: {}",
            path.display()
        );
    }
}
