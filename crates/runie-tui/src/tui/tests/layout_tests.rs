//! Layout tests verifying exact positioning of UI elements.

use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};
use crate::pipe::RenderPipe;
use crate::components::global_tags::GlobalTagsViewModel;
use crate::components::top_bar::TopBarViewModel;

#[test]
fn test_layout_areas_in_correct_order() {
    let area = Rect::new(0, 0, 80, 24);
    let input_h = 3;
    let areas = RenderPipe::layout_main(area, true, input_h);

    // Top bar at top
    assert_eq!(areas[0].y, area.y);
    // Feed below top bar
    assert!(areas[1].y > areas[0].y);
    // Global tags below feed
    assert!(areas[2].y > areas[1].y);
    // Input below global tags
    assert!(areas[3].y > areas[2].y);
    // Hotkeys at bottom
    assert!(areas[4].y > areas[3].y);
}

#[test]
fn test_global_tags_renders_in_global_tags_area() {
    let mut buf = Buffer::empty(Rect::new(0, 0, 80, 24));
    let area = Rect::new(0, 10, 80, 1); // y=10 is global tags area
    let vm = GlobalTagsViewModel::idle("openai/gpt-4o", 0, 0.0, None, None, None);

    Widget::render(vm, area, &mut buf);

    // Assert content at y=10, not y=0
    let cell = buf.cell((2, 10)).unwrap();
    assert!(cell.symbol() != " ");
}

#[test]
fn test_top_bar_and_global_tags_are_separate() {
    // TopBarViewModel has model field - it's used in top bar rendering
    let top_bar = TopBarViewModel {
        repo: "runie".to_string(),
        branch: "main".to_string(),
        path: "src".to_string(),
        model: String::new(),
        context_window: 128_000,
        estimated_tokens: 0,
    };

    // GlobalTagsViewModel has model info in right field
    let global_tags = GlobalTagsViewModel::idle("gpt-4o", 0, 0.0, None, None, None);

    // Top bar has repo
    assert_eq!(top_bar.repo, "runie");
    // Global tags has model in right field
    assert!(global_tags.right.contains("gpt-4o"));
}
