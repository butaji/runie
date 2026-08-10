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

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SchedulerEvent {
    Enqueued { interactive: bool },
    Started,
    Finished { success: bool },
    Cancelled,
    CancelledWithReason { reason: SchedulerCancellationReason },
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SchedulerMetrics {
    pub queued: u64,
    pub running: u64,
    pub completed: u64,
    pub failed: u64,
    pub cancelled: u64,
    pub interactive_enqueued: u64,
    pub background_enqueued: u64,
    #[serde(default)]
    pub cancelled_by_reason: BTreeMap<SchedulerCancellationReason, u64>,
}

impl SchedulerMetrics {
    pub fn terminal_lines(&self) -> Vec<String> {
        let mut lines = vec![
            format!("queued: {}", self.queued),
            format!("running: {}", self.running),
            format!("completed: {}", self.completed),
            format!("failed: {}", self.failed),
            format!("cancelled: {}", self.cancelled),
            format!("interactive_enqueued: {}", self.interactive_enqueued),
            format!("background_enqueued: {}", self.background_enqueued),
        ];
        lines.extend(
            self.cancelled_by_reason
                .iter()
                .map(|(reason, count)| format!("cancelled_{reason:?}: {count}")),
        );
        lines
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
    } else if metrics.running > 0 {
        metrics.running -= 1;
    } else {
        return Err("scheduler cancelled without a queued call".into());
    }
    metrics.cancelled += 1;
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
    }

    #[test]
    fn terminal_lines_project_metrics_in_stable_order() {
        let metrics = SchedulerMetrics {
            queued: 1,
            running: 2,
            completed: 3,
            failed: 4,
            cancelled: 5,
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
                "interactive_enqueued: 6",
                "background_enqueued: 7",
            ]
        );
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
        assert_eq!(metrics.terminal_lines()[7], "cancelled_User: 1");
    }
}
