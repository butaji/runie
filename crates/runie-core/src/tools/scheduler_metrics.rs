use std::collections::BTreeMap;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum SchedulerCancellationReason {
    Unspecified,
    User,
    Abort,
    Dependency,
    Shutdown,
}

macro_rules! scheduler_cancellation_wire_names {
    ($($variant:ident => $wire:literal),+ $(,)?) => {
        impl SchedulerCancellationReason {
            pub const fn wire_name(self) -> &'static str {
                match self {
                    $(Self::$variant => $wire,)+
                }
            }

            pub fn from_wire_name(name: &str) -> Option<Self> {
                match name {
                    $($wire => Some(Self::$variant),)+
                    _ => None,
                }
            }
        }
    };
}

macro_rules! scheduler_metric_fields {
    ($($field:ident => $wire:literal),+ $(,)?) => {
        fn metric_rows(metrics: &SchedulerMetrics) -> Vec<SchedulerMetricRow> {
            vec![$(SchedulerMetricRow {
                name: $wire.into(),
                value: metrics.$field,
            }),+]
        }
    };
}

scheduler_cancellation_wire_names! {
    Unspecified => "unspecified",
    User => "user",
    Abort => "abort",
    Dependency => "dependency",
    Shutdown => "shutdown",
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SchedulerEvent {
    Enqueued { interactive: bool },
    Started,
    Finished { success: bool },
    Cancelled,
    CancelledWithReason { reason: SchedulerCancellationReason },
    CancelledQueued { reason: SchedulerCancellationReason },
    CancelledRunning { reason: SchedulerCancellationReason },
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SchedulerMetrics {
    pub queued: u64,
    pub running: u64,
    pub completed: u64,
    pub failed: u64,
    pub cancelled: u64,
    #[serde(default)]
    pub cancelled_queued: u64,
    #[serde(default)]
    pub cancelled_running: u64,
    pub interactive_enqueued: u64,
    pub background_enqueued: u64,
    #[serde(default)]
    pub cancelled_by_reason: BTreeMap<SchedulerCancellationReason, u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SchedulerMetricRow {
    pub name: String,
    pub value: u64,
}

scheduler_metric_fields! {
    queued => "queued",
    running => "running",
    completed => "completed",
    failed => "failed",
    cancelled => "cancelled",
    cancelled_queued => "cancelled_queued",
    cancelled_running => "cancelled_running",
    interactive_enqueued => "interactive_enqueued",
    background_enqueued => "background_enqueued",
}

impl SchedulerMetrics {
    pub fn rows(&self) -> Vec<SchedulerMetricRow> {
        let mut rows = metric_rows(self);
        rows.extend(
            self.cancelled_by_reason
                .iter()
                .map(|(reason, value)| SchedulerMetricRow {
                    name: format!("cancelled_{}", reason.wire_name()),
                    value: *value,
                }),
        );
        rows
    }

    pub fn terminal_lines(&self) -> Vec<String> {
        self.rows()
            .into_iter()
            .map(|row| format!("{}: {}", row.name, row.value))
            .collect()
    }

    pub fn active_terminal_lines(&self) -> Vec<String> {
        self.rows()
            .into_iter()
            .filter(|row| matches!(row.name.as_str(), "queued" | "running"))
            .map(|row| format!("{}: {}", row.name, row.value))
            .collect()
    }
}

/// Pure scheduler telemetry reducer. Queue ownership remains in the executor.
pub fn reduce_scheduler_event(
    metrics: &mut SchedulerMetrics,
    event: SchedulerEvent,
) -> Result<(), String> {
    match event {
        SchedulerEvent::Enqueued { interactive } => {
            metrics.queued += 1;
            if interactive {
                metrics.interactive_enqueued += 1;
            } else {
                metrics.background_enqueued += 1;
            }
        }
        SchedulerEvent::Started if metrics.queued > 0 => {
            metrics.queued -= 1;
            metrics.running += 1;
        }
        SchedulerEvent::Started => return Err("scheduler started without a queued call".into()),
        SchedulerEvent::Finished { success } => finish(metrics, success)?,
        SchedulerEvent::Cancelled => cancel(metrics, SchedulerCancellationReason::Unspecified)?,
        SchedulerEvent::CancelledWithReason { reason } => cancel(metrics, reason)?,
        SchedulerEvent::CancelledQueued { reason } => cancel_queued(metrics, reason)?,
        SchedulerEvent::CancelledRunning { reason } => cancel_running(metrics, reason)?,
    }
    Ok(())
}

fn count_cancellation(metrics: &mut SchedulerMetrics, reason: SchedulerCancellationReason) {
    *metrics.cancelled_by_reason.entry(reason).or_default() += 1;
}

fn cancel(
    metrics: &mut SchedulerMetrics,
    reason: SchedulerCancellationReason,
) -> Result<(), String> {
    if metrics.queued > 0 {
        metrics.queued -= 1;
        metrics.cancelled += 1;
        metrics.cancelled_queued += 1;
    } else if metrics.running > 0 {
        metrics.running -= 1;
        metrics.cancelled += 1;
        metrics.cancelled_running += 1;
    } else {
        return Err("scheduler cancelled without a queued call".into());
    }
    count_cancellation(metrics, reason);
    Ok(())
}

fn cancel_queued(
    metrics: &mut SchedulerMetrics,
    reason: SchedulerCancellationReason,
) -> Result<(), String> {
    if metrics.queued == 0 {
        return Err("scheduler cancelled queued work without a queued call".into());
    }
    metrics.queued -= 1;
    metrics.cancelled += 1;
    metrics.cancelled_queued += 1;
    count_cancellation(metrics, reason);
    Ok(())
}

fn cancel_running(
    metrics: &mut SchedulerMetrics,
    reason: SchedulerCancellationReason,
) -> Result<(), String> {
    if metrics.running == 0 {
        return Err("scheduler cancelled running work without a running call".into());
    }
    metrics.running -= 1;
    metrics.cancelled += 1;
    metrics.cancelled_running += 1;
    count_cancellation(metrics, reason);
    Ok(())
}

fn finish(metrics: &mut SchedulerMetrics, success: bool) -> Result<(), String> {
    if metrics.running == 0 {
        return Err("scheduler finished without a running call".into());
    }
    metrics.running -= 1;
    if success {
        metrics.completed += 1;
    } else {
        metrics.failed += 1;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scheduler_trace_reduces_queue_lifecycle_and_rejects_dead_ends() {
        let mut metrics = SchedulerMetrics::default();
        for event in [
            SchedulerEvent::Enqueued { interactive: true },
            SchedulerEvent::Enqueued { interactive: false },
            SchedulerEvent::Started,
            SchedulerEvent::Finished { success: true },
            SchedulerEvent::Cancelled,
        ] {
            reduce_scheduler_event(&mut metrics, event).unwrap();
        }
        assert_eq!(metrics.completed, 1);
        assert_eq!(metrics.cancelled, 1);
        assert_eq!(metrics.interactive_enqueued, 1);
        assert_eq!(metrics.background_enqueued, 1);
        assert!(reduce_scheduler_event(&mut metrics, SchedulerEvent::Started).is_err());
    }

    #[test]
    fn cancelling_running_work_releases_the_running_slot() {
        let mut metrics = SchedulerMetrics::default();
        for event in [
            SchedulerEvent::Enqueued { interactive: true },
            SchedulerEvent::Started,
            SchedulerEvent::Cancelled,
        ] {
            reduce_scheduler_event(&mut metrics, event).unwrap();
        }
        assert_eq!(metrics.running, 0);
        assert_eq!(metrics.cancelled, 1);
        assert_eq!(metrics.cancelled_running, 1);
    }

    #[test]
    fn terminal_lines_project_metrics_in_stable_order() {
        let metrics = SchedulerMetrics {
            queued: 1,
            running: 2,
            completed: 3,
            failed: 4,
            cancelled: 5,
            cancelled_queued: 8,
            cancelled_running: 9,
            interactive_enqueued: 6,
            background_enqueued: 7,
            cancelled_by_reason: BTreeMap::new(),
        };
        assert_eq!(
            metrics.terminal_lines(),
            vec![
                "queued: 1",
                "running: 2",
                "completed: 3",
                "failed: 4",
                "cancelled: 5",
                "cancelled_queued: 8",
                "cancelled_running: 9",
                "interactive_enqueued: 6",
                "background_enqueued: 7",
            ]
        );
    }

    #[test]
    fn active_terminal_lines_project_only_live_work() {
        let metrics = SchedulerMetrics {
            queued: 2,
            running: 1,
            completed: 9,
            ..Default::default()
        };
        assert_eq!(metrics.active_terminal_lines(), ["queued: 2", "running: 1"]);
    }

    #[test]
    fn rows_are_the_serializable_scheduler_projection() {
        let metrics = SchedulerMetrics {
            queued: 2,
            running: 1,
            ..SchedulerMetrics::default()
        };
        assert_eq!(
            metrics.rows()[..2],
            [
                SchedulerMetricRow {
                    name: "queued".into(),
                    value: 2,
                },
                SchedulerMetricRow {
                    name: "running".into(),
                    value: 1,
                },
            ]
        );
        assert_eq!(metrics.terminal_lines()[..2], ["queued: 2", "running: 1"]);
    }

    #[test]
    fn cancellation_reasons_are_replayable_and_counted() {
        let mut metrics = SchedulerMetrics::default();
        for event in [
            SchedulerEvent::Enqueued { interactive: true },
            SchedulerEvent::Started,
            SchedulerEvent::CancelledWithReason {
                reason: SchedulerCancellationReason::User,
            },
        ] {
            reduce_scheduler_event(&mut metrics, event).unwrap();
        }
        assert_eq!(
            metrics.cancelled_by_reason[&SchedulerCancellationReason::User],
            1
        );
        assert_eq!(metrics.terminal_lines()[9], "cancelled_user: 1");
    }

    #[test]
    fn cancellation_reason_wire_names_round_trip() {
        for reason in [
            SchedulerCancellationReason::Unspecified,
            SchedulerCancellationReason::User,
            SchedulerCancellationReason::Abort,
            SchedulerCancellationReason::Dependency,
            SchedulerCancellationReason::Shutdown,
        ] {
            assert_eq!(
                SchedulerCancellationReason::from_wire_name(reason.wire_name()),
                Some(reason)
            );
        }
        assert_eq!(SchedulerCancellationReason::from_wire_name("unknown"), None);
    }

    #[test]
    fn explicit_cancellation_events_release_the_declared_slot() {
        let mut metrics = SchedulerMetrics::default();
        for event in [
            SchedulerEvent::Enqueued { interactive: true },
            SchedulerEvent::Enqueued { interactive: false },
            SchedulerEvent::Started,
            SchedulerEvent::CancelledRunning {
                reason: SchedulerCancellationReason::Abort,
            },
            SchedulerEvent::CancelledQueued {
                reason: SchedulerCancellationReason::User,
            },
        ] {
            reduce_scheduler_event(&mut metrics, event).unwrap();
        }
        assert_eq!(metrics.queued, 0);
        assert_eq!(metrics.running, 0);
        assert_eq!(metrics.cancelled, 2);
        assert_eq!(metrics.cancelled_running, 1);
        assert_eq!(metrics.cancelled_queued, 1);
        assert_eq!(
            metrics.cancelled_by_reason[&SchedulerCancellationReason::Abort],
            1
        );
        assert_eq!(
            metrics.cancelled_by_reason[&SchedulerCancellationReason::User],
            1
        );
    }

    #[test]
    fn explicit_cancellation_rejects_the_wrong_empty_slot() {
        let mut metrics = SchedulerMetrics::default();
        assert!(reduce_scheduler_event(
            &mut metrics,
            SchedulerEvent::CancelledRunning {
                reason: SchedulerCancellationReason::User,
            }
        )
        .is_err());
    }
}
