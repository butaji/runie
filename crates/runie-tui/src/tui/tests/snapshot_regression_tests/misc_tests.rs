use super::*;

#[test]
fn snapshot_sidebar_visible() {
    let vm = AgentListViewModel {
        plan_steps: vec![
            (1, "Step 1".to_string(), PlanStatus::Complete),
            (2, "Step 2".to_string(), PlanStatus::Active),
            (3, "Step 3".to_string(), PlanStatus::Pending),
        ],
        running_jobs: vec![],
        active_count: 0,
        tokens: 1000,
        cost: 0.005,
        agent_running: false,
        braille_frame: 0,
    };
    let colors = make_test_colors();
    let area = Rect::new(80 - SIDEBAR_WIDTH, 2, SIDEBAR_WIDTH, 20);
    let mut buf = Buffer::empty(area);
    render_agent_list(&vm, area, &mut buf, &colors);
    insta::assert_snapshot!("snapshot_sidebar_visible", buffer_to_string(&buf));
}

#[test]
fn snapshot_diff_viewer() {
    let diff = DiffViewer::new(
        "src/main.rs".to_string(),
        "fn main() {\n    println!(\"Hello\");\n}".to_string(),
        "fn main() {\n    println!(\"Hello, World!\");\n}".to_string(),
    );
    let theme = ThemeWrapper::default();
    let area = Rect::new(0, 0, 80, 24);
    let mut buf = Buffer::empty(area);
    diff.render_ref(area, &mut buf, &theme);
    insta::assert_snapshot!("snapshot_diff_viewer", buffer_to_string(&buf));
}
