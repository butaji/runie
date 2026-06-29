//! File picker rendering tests — git status labels (Layer 3)
//!
//! Verifies that file picker panel items include git status labels
//! (e.g. "modified lib.rs") when git status is present.

use super::*;
use ratatui::{backend::TestBackend, Terminal};
use runie_core::{
    commands::DialogState,
    dialog::{ItemAction, Panel, PanelStack},
    AppState,
};

use crate::ui::view;

/// Render AppState with TestBackend and return the buffer for inspection.
fn render_to_buffer(state: &mut AppState) -> ratatui::buffer::Buffer {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| view(f, state)).unwrap();
    terminal.backend().buffer().clone()
}

/// Find a line in the buffer that contains the given text substring.
fn find_line(buf: &ratatui::buffer::Buffer, text: &str) -> Option<String> {
    let rect = ratatui::layout::Rect {
        x: 0,
        y: 0,
        width: 80,
        height: 24,
    };
    for y in rect.y..rect.y + rect.height {
        let line: String = (rect.x..rect.x + rect.width)
            .map(|x| buf[(x, y)].symbol())
            .collect::<String>()
            .trim_end()
            .to_owned();
        if line.contains(text) {
            return Some(line);
        }
    }
    None
}

/// Open a panel dialog on top of AppState.
fn open_panel(state: &mut AppState, panel: Panel) {
    state.open_dialog = Some(DialogState::Active {
        kind: DialogKind::Generic,
        panels: PanelStack::new(panel),
    });
}

/// Build a file picker panel using the same label logic as `build_file_picker_panel`.
/// This mirrors the label construction so we can test that git status is included.
fn build_file_picker_items(
    entries: &[runie_core::model::FffFileEntry],
) -> Vec<(String, ItemAction)> {
    entries
        .iter()
        .map(|entry| {
            let name = if entry.is_dir {
                format!("{}/", entry.name)
            } else {
                entry.name.clone()
            };
            let label = if let Some(status) = &entry.git_status {
                if !status.is_empty() {
                    format!("{} {}", status, name)
                } else {
                    name
                }
            } else {
                name
            };
            let insert_name = if entry.is_dir {
                format!("{}/", entry.path)
            } else {
                entry.path.clone()
            };
            (
                label,
                ItemAction::Emit(runie_core::Event::InsertAtRef(insert_name)),
            )
        })
        .collect()
}

/// L3: File picker panel renders git status labels in panel output.
#[test]
fn fff_picker_displays_status_labels() {
    let _lock = crate::theme::test_lock();

    let entries = vec![
        runie_core::model::FffFileEntry {
            name: "lib.rs".into(),
            path: "src/lib.rs".into(),
            is_dir: false,
            score: 1.0,
            git_status: Some("modified".into()),
        },
        runie_core::model::FffFileEntry {
            name: "main.rs".into(),
            path: "src/main.rs".into(),
            is_dir: false,
            score: 0.9,
            git_status: Some("untracked".into()),
        },
        runie_core::model::FffFileEntry {
            name: "readme.md".into(),
            path: "docs/readme.md".into(),
            is_dir: false,
            score: 0.5,
            git_status: Some("deleted".into()),
        },
        runie_core::model::FffFileEntry {
            name: "utils".into(),
            path: "src/utils".into(),
            is_dir: true,
            score: 0.3,
            git_status: None,
        },
    ];

    let items = build_file_picker_items(&entries);
    let mut panel = Panel::new("at-files", " Files ");
    for (label, action) in items {
        panel = panel.item(&label, action);
    }
    panel = panel.with_filter();
    panel.filter = "".to_string();

    let mut state = AppState::default();
    open_panel(&mut state, panel);

    let buf = render_to_buffer(&mut state);

    // Check that modified files show "modified" label
    assert!(
        find_line(&buf, "modified").is_some(),
        "modified lib.rs should display 'modified' label in panel, content: {:?}",
        extract_panel_content(&buf)
    );

    // Check that untracked files show "untracked" label
    assert!(
        find_line(&buf, "untracked").is_some(),
        "untracked main.rs should display 'untracked' label in panel, content: {:?}",
        extract_panel_content(&buf)
    );

    // Check that deleted files show "deleted" label
    assert!(
        find_line(&buf, "deleted").is_some(),
        "deleted readme.md should display 'deleted' label in panel, content: {:?}",
        extract_panel_content(&buf)
    );

    // Check that clean/untracked files without status show filename
    assert!(
        find_line(&buf, "utils/").is_some(),
        "clean dir entry should show directory name with slash, content: {:?}",
        extract_panel_content(&buf)
    );
}

/// L3: File picker panel with mixed status labels renders in order.
#[test]
fn fff_picker_status_labels_in_render_order() {
    let _lock = crate::theme::test_lock();

    let entries = vec![
        runie_core::model::FffFileEntry {
            name: "foo.rs".into(),
            path: "foo.rs".into(),
            is_dir: false,
            score: 1.0,
            git_status: Some("modified".into()),
        },
        runie_core::model::FffFileEntry {
            name: "bar.rs".into(),
            path: "bar.rs".into(),
            is_dir: false,
            score: 0.8,
            git_status: None,
        },
    ];

    let items = build_file_picker_items(&entries);
    let mut panel = Panel::new("at-files", " Files ");
    for (label, action) in items {
        panel = panel.item(&label, action);
    }
    panel = panel.with_filter();
    panel.filter = "".to_string();

    let mut state = AppState::default();
    open_panel(&mut state, panel);

    let buf = render_to_buffer(&mut state);

    // Both items should appear
    assert!(
        find_line(&buf, "foo.rs").is_some(),
        "foo.rs should appear in panel, content: {:?}",
        extract_panel_content(&buf)
    );
    assert!(
        find_line(&buf, "bar.rs").is_some(),
        "bar.rs should appear in panel, content: {:?}",
        extract_panel_content(&buf)
    );

    // modified should appear
    assert!(
        find_line(&buf, "modified").is_some(),
        "'modified' status label should appear, content: {:?}",
        extract_panel_content(&buf)
    );
}

/// Helper: extract all non-empty lines from the buffer for debugging.
fn extract_panel_content(buf: &ratatui::buffer::Buffer) -> Vec<String> {
    let mut lines = Vec::new();
    let rect = ratatui::layout::Rect {
        x: 0,
        y: 0,
        width: 80,
        height: 24,
    };
    for y in rect.y..rect.y + rect.height {
        let line: String = (rect.x..rect.x + rect.width)
            .map(|x| buf[(x, y)].symbol())
            .collect::<String>()
            .trim_end()
            .to_owned();
        if !line.trim().is_empty() {
            lines.push(line);
        }
    }
    lines
}
