use super::*;

pub(super) fn clear_finished(
    reply: oneshot::Sender<usize>,
    jobs: &mut BTreeMap<String, BackgroundJob>,
    publisher: &BackgroundSnapshotPublisher,
) {
    let ids = jobs
        .values()
        .filter(|job| job.status != BackgroundStatus::Running)
        .map(|job| job.id.clone())
        .collect::<Vec<_>>();
    let cleared = ids.iter().filter(|id| jobs.remove(*id).is_some()).count();
    if cleared > 0 {
        publish(publisher, jobs);
    }
    let _ = reply.send(cleared);
}
