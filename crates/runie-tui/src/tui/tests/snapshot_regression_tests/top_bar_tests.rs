use super::*;

#[test]
fn snapshot_top_bar_gauge_0pct() {
    let vm = TopBarViewModel {
        repo: "runie".to_string(), branch: "main".to_string(), path: "src".to_string(),
        model: "claude".to_string(), context_window: 128_000, estimated_tokens: 0,
    };
    let colors = make_test_colors();
    let area = Rect::new(0, 0, 80, 2);
    let mut buf = Buffer::empty(area);
    render_top_bar(&vm, area, &mut buf, &colors);
    insta::assert_snapshot!("snapshot_top_bar_gauge_0pct", buffer_to_string(&buf));
}

#[test]
fn snapshot_top_bar_gauge_50pct() {
    let vm = TopBarViewModel {
        repo: "runie".to_string(), branch: "main".to_string(), path: "src".to_string(),
        model: "claude".to_string(), context_window: 128_000, estimated_tokens: 64_000,
    };
    let colors = make_test_colors();
    let area = Rect::new(0, 0, 80, 2);
    let mut buf = Buffer::empty(area);
    render_top_bar(&vm, area, &mut buf, &colors);
    insta::assert_snapshot!("snapshot_top_bar_gauge_50pct", buffer_to_string(&buf));
}

#[test]
fn snapshot_top_bar_gauge_100pct() {
    let vm = TopBarViewModel {
        repo: "runie".to_string(), branch: "main".to_string(), path: "src".to_string(),
        model: "claude".to_string(), context_window: 128_000, estimated_tokens: 128_000,
    };
    let colors = make_test_colors();
    let area = Rect::new(0, 0, 80, 2);
    let mut buf = Buffer::empty(area);
    render_top_bar(&vm, area, &mut buf, &colors);
    insta::assert_snapshot!("snapshot_top_bar_gauge_100pct", buffer_to_string(&buf));
}

#[test]
fn snapshot_top_bar_gauge_over_100pct() {
    let vm = TopBarViewModel {
        repo: "runie".to_string(), branch: "main".to_string(), path: "src".to_string(),
        model: "claude".to_string(), context_window: 128_000, estimated_tokens: 150_000,
    };
    let colors = make_test_colors();
    let area = Rect::new(0, 0, 80, 2);
    let mut buf = Buffer::empty(area);
    render_top_bar(&vm, area, &mut buf, &colors);
    insta::assert_snapshot!("snapshot_top_bar_gauge_over_100pct", buffer_to_string(&buf));
}

#[test]
fn snapshot_dark_theme_top_bar() {
    let vm = TopBarViewModel {
        repo: "runie".to_string(), branch: "main".to_string(), path: "src".to_string(),
        model: "claude".to_string(), context_window: 128_000, estimated_tokens: 64_000,
    };
    let colors = make_test_colors();
    let area = Rect::new(0, 0, 80, 2);
    let mut buf = Buffer::empty(area);
    render_top_bar(&vm, area, &mut buf, &colors);
    insta::assert_snapshot!("snapshot_dark_theme_top_bar", buffer_to_string(&buf));
}
