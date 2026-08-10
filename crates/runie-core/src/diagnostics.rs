//! Deterministic, serializable diagnostics bundle assembled from actor
//! snapshots. Collection remains outside this pure data boundary.

use crate::command_actor::DiagnosticReport;
use crate::telemetry::{usage_summary, TelemetrySnapshot, UsageSummary};
use serde::{Deserialize, Serialize};

const MAX_DIAGNOSTIC_POINTS: usize = 1_024;

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
    pub series: Vec<DiagnosticSeries>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticSeries {
    pub label: String,
    pub points: Vec<DiagnosticPoint>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticPoint {
    pub sequence: u64,
    pub value: f64,
}

impl DiagnosticVisualization {
    pub fn from_bundle(bundle: &DiagnosticBundle) -> Self {
        let usage = &bundle.usage;
        let series = [
            ("input_tokens", "pi.ai.usage.input_tokens"),
            ("output_tokens", "pi.ai.usage.output_tokens"),
            ("cost", "pi.ai.usage.cost"),
        ]
        .into_iter()
        .map(|(label, key)| diagnostic_series(bundle, label, key))
        .collect();
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
            series,
        }
    }

    /// Project diagnostic data into stable terminal rows without terminal
    /// state, colors, or side effects. A renderer may style these rows later.
    pub fn terminal_lines(&self, bundle: &DiagnosticBundle) -> Vec<String> {
        let mut lines = bundle
            .report
            .checks
            .iter()
            .map(|check| format!("check: {check}"))
            .collect::<Vec<_>>();
        lines.extend(
            self.metrics
                .iter()
                .map(|metric| format!("{}: {}", metric.label, metric.value)),
        );
        lines
    }
}

fn diagnostic_series(bundle: &DiagnosticBundle, label: &str, key: &str) -> DiagnosticSeries {
    let mut points = bundle
        .telemetry
        .spans
        .iter()
        .filter(|span| span.ended && span.name == "pi.ai.request")
        .filter_map(|span| {
            Some(DiagnosticPoint {
                sequence: span.end_sequence?,
                value: span
                    .attributes
                    .get(key)?
                    .as_f64()
                    .or_else(|| span.attributes.get(key)?.as_u64().map(|value| value as f64))?,
            })
        })
        .collect::<Vec<_>>();
    limit_diagnostic_points(&mut points);
    DiagnosticSeries {
        label: label.into(),
        points,
    }
}

fn limit_diagnostic_points(points: &mut Vec<DiagnosticPoint>) {
    points.sort_by_key(|point| point.sequence);
    if points.len() > MAX_DIAGNOSTIC_POINTS {
        let keep_from = points.len() - MAX_DIAGNOSTIC_POINTS;
        points.drain(..keep_from);
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
        assert_eq!(visualization.series.len(), 3);
        assert!(visualization
            .series
            .iter()
            .all(|series| series.points.is_empty()));
        assert_eq!(
            visualization.terminal_lines(&restored)[0],
            "check: workspace"
        );
    }

    #[test]
    fn visualization_projects_ended_request_series() {
        let mut attributes = std::collections::HashMap::new();
        attributes.insert("pi.ai.usage.input_tokens".into(), serde_json::json!(12));
        attributes.insert("pi.ai.usage.output_tokens".into(), serde_json::json!(8));
        attributes.insert("pi.ai.usage.cost".into(), serde_json::json!(0.25));
        let telemetry = TelemetrySnapshot {
            spans: vec![crate::telemetry::SpanSnapshot {
                id: 3,
                parent_id: None,
                name: "pi.ai.request".into(),
                attributes,
                events: vec![],
                status: crate::telemetry::SpanStatus::Ok,
                explicit_status: true,
                error: None,
                ended: true,
                end_sequence: Some(7),
            }],
            ..TelemetrySnapshot::default()
        };
        let bundle = DiagnosticBundle::from_snapshots(DiagnosticReport::default(), telemetry);
        let visualization = DiagnosticVisualization::from_bundle(&bundle);
        assert_eq!(visualization.series[0].points[0].sequence, 7);
        assert_eq!(visualization.series[0].points[0].value, 12.0);
        assert_eq!(visualization.series[2].points[0].value, 0.25);
    }

    #[test]
    fn visualization_keeps_newest_bounded_points_in_sequence_order() {
        let mut points = vec![
            DiagnosticPoint {
                sequence: 3,
                value: 3.0,
            },
            DiagnosticPoint {
                sequence: 1,
                value: 1.0,
            },
            DiagnosticPoint {
                sequence: 2,
                value: 2.0,
            },
        ];
        limit_diagnostic_points(&mut points);
        assert_eq!(
            points
                .iter()
                .map(|point| point.sequence)
                .collect::<Vec<_>>(),
            [1, 2, 3]
        );
    }
}
