//! Deterministic, serializable diagnostics bundle assembled from actor
//! snapshots. Collection remains outside this pure data boundary.

use crate::command_actor::DiagnosticReport;
use crate::telemetry::{usage_summary, TelemetrySnapshot, UsageSummary};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticBundle {
    pub report: DiagnosticReport,
    pub usage: UsageSummary,
    pub telemetry: TelemetrySnapshot,
}

impl DiagnosticBundle {
    pub fn from_snapshots(report: DiagnosticReport, telemetry: TelemetrySnapshot) -> Self {
        Self {
            usage: usage_summary(&telemetry),
            report,
            telemetry,
        }
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn from_json(value: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundle_round_trips_and_derives_usage_from_telemetry() {
        let report = DiagnosticReport {
            fix_requested: false,
            checks: vec!["workspace".into()],
        };
        let telemetry = TelemetrySnapshot::default();
        let bundle = DiagnosticBundle::from_snapshots(report, telemetry);
        let restored = DiagnosticBundle::from_json(&bundle.to_json().unwrap()).unwrap();
        assert_eq!(restored, bundle);
        assert_eq!(restored.usage.requests, 0);
    }
}
