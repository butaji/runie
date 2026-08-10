#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SchedulerEvent {
    Enqueued { interactive: bool },
    Started,
    Finished { success: bool },
    Cancelled,
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
        SchedulerEvent::Cancelled if metrics.queued > 0 => {
            metrics.queued -= 1;
            metrics.cancelled += 1;
        }
        SchedulerEvent::Cancelled if metrics.running > 0 => {
            metrics.running -= 1;
            metrics.cancelled += 1;
        }
        SchedulerEvent::Cancelled => return Err("scheduler cancelled without a queued call".into()),
    }
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
}
