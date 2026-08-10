#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests {
    use super::super::{
        dimensions, frame_cell_difference_counts, ordered_common_frame_count, replay_frames,
        validate_capture_artifacts, validate_capture_metadata_shape, validate_resize_artifact,
        validate_resize_report, Cell,
    };
    use std::path::{Path, PathBuf};

    fn cast(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("artifacts")
            .join(name)
    }

    fn frame(symbol: &str) -> Vec<Cell> {
        vec![Cell {
            symbol: symbol.into(),
            width: 1,
            fg: "default".into(),
            bg: "default".into(),
            bold: false,
            italic: false,
            underline: false,
            inverse: false,
        }]
    }

    #[test]
    fn dimensions_reject_values_that_do_not_fit_terminal_geometry() {
        let error = dimensions(&serde_json::json!({"width": 65_536, "height": 24}))
            .expect_err("oversized cast geometry must be rejected");
        assert!(error.to_string().contains("width exceeds u16"));
    }

    #[test]
    fn ordered_common_frames_distinguish_cadence_from_missing_visual_states() {
        let left = vec![frame("a"), frame("b"), frame("c")];
        let right = vec![frame("a"), frame("a"), frame("b"), frame("c")];
        assert_eq!(ordered_common_frame_count(&left, &right), 3);
    }

    #[test]
    fn frame_difference_summary_counts_every_changed_frame_and_cell() {
        let left = vec![frame("a"), frame("b"), frame("c")];
        let mut changed = frame("b");
        changed[0].bold = true;
        let right = vec![frame("a"), changed, frame("d")];
        assert_eq!(frame_cell_difference_counts(&left, &right), vec![0, 1, 1]);
    }

    #[test]
    fn phase_marker_selects_visible_frames() {
        let path = cast("grok-rich.cast");
        let (_, frames) = replay_frames(&path, Some("❯")).expect("recorded prompt marker");
        assert!(!frames.is_empty());
    }

    #[test]
    fn phase_marker_can_select_a_numbered_occurrence() {
        let path = cast("runie-full.cast");
        let (_, frames) =
            replay_frames(&path, Some("session_start#2")).expect("recorded second session marker");
        assert!(!frames.is_empty());
    }

    #[test]
    fn phase_marker_can_require_multiple_visible_markers() {
        let path = cast("grok-rich.cast");
        let (_, frames) = replay_frames(&path, Some("Listed 1 dir&&Read 1 file"))
            .expect("combined markers must select a settled frame");
        assert!(!frames.is_empty());
    }

    #[test]
    fn missing_phase_marker_is_an_error() {
        let path = cast("grok-rich.cast");
        let error = replay_frames(&path, Some("__missing_phase_marker__"))
            .expect_err("missing markers must not produce an empty comparison");
        assert!(error.to_string().contains("phase marker"));
        assert!(error.to_string().contains("grok-rich.cast"));
    }

    #[test]
    fn capture_metadata_requires_provenance_and_artifacts() {
        let valid = serde_json::json!({
            "captured_at": "2026-08-08T00:00:00Z",
            "repo_revision": "abc123",
            "command": "target/debug/runie",
            "grok_path": "/usr/local/bin/grok",
            "grok_version": "grok 0.2.118",
            "capture_tools": {"tmux": "tmux 3.7b", "asciinema": "asciinema 3.2.1"},
            "terminal": {"cols": 80, "rows": 24, "term": "xterm-256color", "colorterm": "truecolor"},
            "probe": {"prompt": "Hey", "quit_key": "C-q"},
            "artifacts": {
                "cast": "/tmp/capture.cast",
                "raw": "/tmp/capture.raw",
                "settled_ansi": "/tmp/capture.settled.ansi",
                "grok_doctor": "/tmp/capture.grok-doctor.json",
                "resize_report": "/tmp/capture.resize.json"
            }
        });
        validate_capture_metadata_shape(&valid, Path::new("capture.meta.json"))
            .expect("complete capture metadata");

        let mut incomplete = valid;
        incomplete["grok_version"] = serde_json::Value::String(String::new());
        let error = validate_capture_metadata_shape(&incomplete, Path::new("capture.meta.json"))
            .expect_err("missing provenance must fail");
        assert!(error.to_string().contains("grok_version"));
    }

    #[test]
    fn resize_report_must_observe_declared_schedule() {
        let report = serde_json::json!({
            "valid": true,
            "observed": [
                {"at_ms": 250, "geometry": "80,12"},
                {"at_ms": 500, "geometry": "100,24"}
            ]
        });
        validate_resize_report(&report, "250,80,12;500,100,24").expect("valid report");
        let mut invalid = report;
        invalid["observed"][1]["geometry"] = serde_json::Value::String("99,24".into());
        assert!(validate_resize_report(&invalid, "250,80,12;500,100,24").is_err());
    }

    #[test]
    fn capture_metadata_rejects_missing_resize_report_artifact() {
        let metadata = serde_json::json!({
            "artifacts": {"resize_report": "/tmp/runie-nonexistent-resize-report.json"},
            "resize_schedule": "250,80,12"
        });
        let error = validate_resize_artifact(&metadata, Path::new("capture.meta.json"))
            .expect_err("missing resize report must invalidate capture evidence");
        assert!(error.to_string().contains("does not exist"));
    }

    #[test]
    fn capture_metadata_rejects_missing_required_capture_artifact() {
        let metadata = serde_json::json!({
            "artifacts": {
                "cast": "/tmp/runie-missing-cast.cast",
                "raw": "/tmp/runie-missing-cast.raw",
                "settled_ansi": "/tmp/runie-missing-cast.settled.ansi",
                "grok_doctor": "/tmp/runie-missing-cast.doctor.json"
            }
        });
        let error = validate_capture_artifacts(&metadata, Path::new("capture.meta.json"))
            .expect_err("missing capture artifact must invalidate evidence");
        assert!(error.to_string().contains("artifacts.cast"));
    }
}
