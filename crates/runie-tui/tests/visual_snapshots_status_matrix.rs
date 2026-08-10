use super::*;
#[test]
fn turn_status_phase_matrix_snapshot() {
    let phases = [
        ("starting", TurnStatus::new(0)),
        ("waiting", TurnStatus::new(4).phase(TurnStatusPhase::Waiting).with_chrome(" 0.0s                            0.0s ⇣3.18k [stop]")),
        ("thinking", TurnStatus::new(2).phase(TurnStatusPhase::Thinking)),
        ("responding", TurnStatus::new(3).phase(TurnStatusPhase::Responding).with_chrome(" 0.0s                                                                              2.3s ⇣6.39k [stop]")),
    ];
    insta::assert_snapshot!(phases
        .iter()
        .map(|(name, status)| format!("{name}: {}", status.text()))
        .collect::<Vec<_>>()
        .join("\n"));
}
