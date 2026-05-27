// ============================================================================
// View Model Builder Tests - Diff Viewer
// ============================================================================

use crate::components::DiffViewer;
use crate::tui::state::AppState;
use crate::tui::view_models::ViewModels;
use crate::components::CommandPalette;
use crate::components::diff_viewer::DiffLine;

fn make_state() -> AppState {
    AppState::default()
}

fn build_vms(state: &AppState) -> ViewModels {
    let render = crate::tui::state::RenderState::from(state);
    ViewModels::from_render_state(&render, &CommandPalette::default())
}

#[test]
fn test_diff_viewer_vm_none_when_not_visible() {
    let mut state = make_state();
    state.diff_viewer = None;
    let vms = build_vms(&state);
    assert!(vms.diff_viewer.is_none());
}

#[test]
fn test_diff_viewer_vm_with_diff() {
    let mut state = make_state();
    state.diff_viewer = Some(DiffViewer::new(
        "test.rs".to_string(),
        "hello".to_string(),
        "hello world".to_string(),
    ));
    let vms = build_vms(&state);
    let vm = vms.diff_viewer.unwrap();
    assert_eq!(vm.filename, "test.rs");
    assert!(vm.visible);
    assert_eq!(vm.scroll_offset, 0);
    assert_eq!(vm.diff_lines.len(), 2);
}

#[test]
fn test_diff_viewer_vm_identical_content() {
    let mut state = make_state();
    state.diff_viewer = Some(DiffViewer::new(
        "foo.txt".to_string(),
        "line1\nline2".to_string(),
        "line1\nline2".to_string(),
    ));
    let vms = build_vms(&state);
    let vm = vms.diff_viewer.unwrap();
    assert_eq!(vm.diff_lines.len(), 2);
    assert!(matches!(vm.diff_lines[0], DiffLine::Context(_)));
    assert!(matches!(vm.diff_lines[1], DiffLine::Context(_)));
}

#[test]
fn test_diff_viewer_vm_modified_content() {
    let mut state = make_state();
    state.diff_viewer = Some(DiffViewer::new(
        "bar.txt".to_string(),
        "old line".to_string(),
        "new line".to_string(),
    ));
    let vms = build_vms(&state);
    let vm = vms.diff_viewer.unwrap();
    assert_eq!(vm.diff_lines.len(), 2);
    assert!(matches!(vm.diff_lines[0], DiffLine::Removed(_)));
    assert!(matches!(vm.diff_lines[1], DiffLine::Added(_)));
}

#[test]
fn test_diff_viewer_vm_scroll_offset() {
    let mut state = make_state();
    let mut diff = DiffViewer::new(
        "test.rs".to_string(),
        "1\n2\n3\n4\n5".to_string(),
        "1\n2\n3\n4\n5".to_string(),
    );
    diff.scroll_offset = 10;
    state.diff_viewer = Some(diff);
    let vms = build_vms(&state);
    assert_eq!(vms.diff_viewer.unwrap().scroll_offset, 10);
}

#[test]
fn test_diff_viewer_vm_added_lines() {
    let mut state = make_state();
    state.diff_viewer = Some(DiffViewer::new(
        "new.txt".to_string(),
        "".to_string(),
        "added line".to_string(),
    ));
    let vms = build_vms(&state);
    let vm = vms.diff_viewer.unwrap();
    assert_eq!(vm.diff_lines.len(), 1);
    assert!(matches!(vm.diff_lines[0], DiffLine::Added(_)));
}

#[test]
fn test_diff_viewer_vm_removed_lines() {
    let mut state = make_state();
    state.diff_viewer = Some(DiffViewer::new(
        "removed.txt".to_string(),
        "removed line".to_string(),
        "".to_string(),
    ));
    let vms = build_vms(&state);
    let vm = vms.diff_viewer.unwrap();
    assert_eq!(vm.diff_lines.len(), 1);
    assert!(matches!(vm.diff_lines[0], DiffLine::Removed(_)));
}

#[test]
fn test_diff_viewer_vm_filename_preserved() {
    let mut state = make_state();
    state.diff_viewer = Some(DiffViewer::new(
        "my_file.rs".to_string(),
        "old".to_string(),
        "new".to_string(),
    ));
    let vms = build_vms(&state);
    assert_eq!(vms.diff_viewer.unwrap().filename, "my_file.rs");
}
