use super::*;

#[tokio::test]
async fn jobs_are_owned_and_reduce_to_completion() {
    let actor = BackgroundProcessActor::new();
    let id = actor.start("printf done").await.unwrap();
    loop {
        if let Some(job) = actor.snapshot().into_iter().find(|job| job.id == id) {
            if job.status == BackgroundStatus::Completed {
                assert_eq!(job.output, "done");
                assert_eq!(actor.shared_snapshot().get()[0].output, "done");
                assert_eq!(actor.shared_snapshot().strong_count(), 2);
                break;
            }
        }
        tokio::task::yield_now().await;
    }
}

#[test]
fn output_projection_is_bounded_and_marks_truncation() {
    let output = bounded_output("x".repeat(BACKGROUND_OUTPUT_MAX_BYTES + 1));
    assert!(output.len() <= BACKGROUND_OUTPUT_MAX_BYTES);
    assert!(output.ends_with(OUTPUT_TRUNCATION_MARKER));
}

#[test]
fn terminal_line_preserves_status_exit_and_output_as_data() {
    let job = BackgroundJob { id: "3".into(), command: "printf done".into(), status: BackgroundStatus::Completed, output: "done\nnext".into(), exit_code: Some(0) };
    assert_eq!(job.terminal_line(), "3 · Completed · printf done exit=0 · done\\nnext");
}

#[test]
fn summary_projects_bounded_output_facts_without_rendering() {
    let jobs = vec![BackgroundJob { id: "3".into(), command: "printf done".into(), status: BackgroundStatus::Completed, output: "done\nnext".into(), exit_code: Some(0) }];
    assert_eq!(background_job_summaries(&jobs)[0].output_lines, 2);
    assert_eq!(background_job_summaries(&jobs)[0].output_bytes, 9);
    assert_eq!(background_job_summaries(&jobs)[0].output_preview.as_deref(), Some("done\nnext"));
}

#[test]
fn summary_owns_bounded_terminal_projection() {
    let summary = background_job_summaries(&[BackgroundJob {
        id: "job-1".into(),
        command: "printf output".into(),
        status: BackgroundStatus::Completed,
        output: "output".into(),
        exit_code: Some(0),
    }])[0]
        .clone();
    assert!(summary.terminal_line().contains("preview=\"output\""));
}

#[test]
fn output_metadata_is_one_shared_projection_for_summary_and_read_rows() {
    let output = format!("first\nsecond{OUTPUT_TRUNCATION_MARKER}");
    let metadata = output_metadata(&output);
    let summary = background_job_summaries(&[BackgroundJob {
        id: "job-1".into(),
        command: "printf output".into(),
        status: BackgroundStatus::Failed,
        output,
        exit_code: Some(1),
    }])[0]
        .clone();

    assert_eq!(summary.output_lines, metadata.lines);
    assert_eq!(summary.output_bytes, metadata.bytes);
    assert_eq!(summary.output_preview, metadata.preview);
    assert_eq!(summary.truncated, metadata.truncated);
}

#[tokio::test]
async fn failed_jobs_preserve_failure_output_and_status() {
    let actor = BackgroundProcessActor::new();
    let id = actor.start("printf failed >&2; exit 3").await.unwrap();
    loop {
        if let Some(job) = actor.snapshot().into_iter().find(|job| job.id == id) {
            if job.status == BackgroundStatus::Failed { assert_eq!(job.output, "failed"); assert_eq!(job.exit_code, Some(3)); break; }
        }
        tokio::task::yield_now().await;
    }
}

#[tokio::test]
async fn output_read_is_actor_owned_and_unknown_ids_are_empty() {
    let actor = BackgroundProcessActor::new();
    let id = actor.start("printf captured").await.unwrap();
    loop { if actor.read_output(&id).await.as_ref().map(|output| output.text.as_str()) == Some("captured") { break; } tokio::task::yield_now().await; }
    let output = actor.read_output(&id).await.expect("captured output");
    assert_eq!(output.text, "captured");
    assert_eq!(output.output_lines, 1);
    assert_eq!(output.command, "printf captured");
    assert_eq!(output.status, BackgroundStatus::Completed);
    assert_eq!(output.exit_code, Some(0));
    assert_eq!(output.terminal_lines(), vec!["background output 0 · Completed · printf captured exit=Some(0) · 1 lines/8 bytes", "captured"]);
    assert_eq!(actor.read_output("missing").await, None);
}

#[tokio::test]
async fn cancel_all_reduces_every_running_job_to_cancelled() {
    let actor = BackgroundProcessActor::new();
    let first = actor.start("yes running").await.unwrap();
    let second = actor.start("yes running").await.unwrap();
    tokio::task::yield_now().await;
    assert_eq!(actor.cancel_all().await, 2);
    for id in [first, second] { assert_eq!(actor.snapshot().into_iter().find(|job| job.id == id).unwrap().status, BackgroundStatus::Cancelled); }
}

#[tokio::test]
async fn clear_finished_removes_terminal_rows_but_preserves_running_jobs() {
    let actor = BackgroundProcessActor::new();
    let completed = actor.start("printf done").await.unwrap();
    let running = actor.start("yes running").await.unwrap();
    loop { if actor.snapshot().iter().any(|job| job.id == completed && job.status == BackgroundStatus::Completed) { break; } tokio::task::yield_now().await; }
    assert_eq!(actor.clear_finished().await, 1);
    assert!(actor.snapshot().iter().all(|job| job.id != completed));
    assert!(actor.snapshot().iter().any(|job| job.id == running));
    actor.cancel_all().await;
}
