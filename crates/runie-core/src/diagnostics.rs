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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticMetric {
    pub label: String,
    pub value: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticVisualization {
    pub metrics: Vec<DiagnosticMetric>,
}

impl DiagnosticVisualization {
    pub fn from_bundle(bundle: &DiagnosticBundle) -> Self {
        let usage = &bundle.usage;
        Self {
            metrics: vec![
                metric("requests", usage.requests as f64),
                metric("input_tokens", usage.input_tokens as f64),
                metric("output_tokens", usage.output_tokens as f64),
                metric("cache_read_tokens", usage.cache_read_tokens as f64),
                metric("cache_write_tokens", usage.cache_write_tokens as f64),
                metric("reasoning_tokens", usage.reasoning_tokens as f64),
                metric("total_tokens", usage.total_tokens as f64),
                metric("cost", usage.cost),
            ],
        }
    }
}

fn metric(label: &str, value: f64) -> DiagnosticMetric {
    DiagnosticMetric {
        label: label.into(),
        value,
    }
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
        let visualization = DiagnosticVisualization::from_bundle(&restored);
        assert_eq!(visualization.metrics[0].label, "requests");
        assert_eq!(visualization.metrics.last().unwrap().label, "cost");
    }
}
