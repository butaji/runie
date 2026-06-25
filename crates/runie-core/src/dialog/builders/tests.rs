use crate::dialog::builders::{
    command_palette, model_selector, scoped_models, session_list, session_tree, settings,
    theme_picker, SessionRow,
};
use crate::dialog::PanelItem;
use crate::Event;

fn dummy_evt() -> Event {
    crate::Event::Abort
}

#[test]
fn command_palette_builds() {
    use crate::commands::CommandRow;
    let stack = command_palette(vec![
        CommandRow::new("Commands", "/save", "Save session", dummy_evt()),
        CommandRow::new("Commands", "/load", "Load session", dummy_evt()),
        CommandRow::new("Skills", "test-skill", "A skill", dummy_evt()),
    ]);
    assert_eq!(stack.len(), 1);
    let panel = stack.current().unwrap();
    assert!(panel.filterable);
    assert!(panel.items.len() > 2);
    // Launcher: must keep_open_on_activate so Esc returns to palette.
    assert!(
        panel.keep_open_on_activate,
        "command palette must keep open on activation for back-stack navigation"
    );
    assert!(
        panel
            .items
            .iter()
            .any(|i| matches!(i, PanelItem::Command { .. })),
        "command palette items should be Command variants"
    );
}

#[test]
fn model_selector_groups_by_provider() {
    let stack = model_selector(
        vec![],
        vec![
            ("openai".into(), vec![("gpt-4o".into(), dummy_evt())]),
            ("anthropic".into(), vec![("claude-3".into(), dummy_evt())]),
        ],
        "gpt-4o",
    );
    let panel = stack.current().unwrap();
    // Has headers for both providers
    let headers: Vec<_> = panel
        .items
        .iter()
        .filter(|i| matches!(i, PanelItem::Header(_)))
        .collect();
    assert_eq!(headers.len(), 2);
}

#[test]
fn settings_builds_with_categories() {
    let stack = settings(vec![
        (
            "Models".into(),
            vec![PanelItem::Select {
                label: "Provider".into(),
                current: "mock".into(),
                options: vec!["mock".into(), "openai".into()],
                key: "provider".into(),
            }],
        ),
        (
            "Appearance".into(),
            vec![PanelItem::Select {
                label: "Theme".into(),
                current: "runie".into(),
                options: vec!["runie".into(), "dracula".into()],
                key: "theme".into(),
            }],
        ),
    ]);
    let panel = stack.current().unwrap();
    let headers: Vec<_> = panel
        .items
        .iter()
        .filter(|i| matches!(i, PanelItem::Header(_)))
        .collect();
    assert_eq!(headers.len(), 2);
}

#[test]
fn scoped_models_groups_by_provider() {
    let stack = scoped_models(vec![
        ("openai".into(), "gpt-4o".into(), true),
        ("openai".into(), "gpt-4o-mini".into(), false),
        ("anthropic".into(), "claude-3".into(), true),
    ]);
    let panel = stack.current().unwrap();
    // 2 provider headers + 1 separator + 3 model items = 6
    assert_eq!(panel.items.len(), 6);
    // Model items must be Toggle variants so Space toggles them instead of
    // adding a character to the panel filter.
    let toggles: Vec<_> = panel
        .items
        .iter()
        .filter(|i| matches!(i, PanelItem::Toggle { .. }))
        .collect();
    assert_eq!(
        toggles.len(),
        3,
        "scoped model items should be Toggle variants"
    );
}

#[test]
fn theme_picker_keeps_dialog_open() {
    let stack = theme_picker(vec![
        ("runie".into(), dummy_evt()),
        ("dracula".into(), dummy_evt()),
    ]);
    let panel = stack.current().unwrap();
    assert!(panel.keep_open_on_activate);
}

#[test]
fn session_tree_handles_empty() {
    let stack = session_tree(vec![]);
    let panel = stack.current().unwrap();
    assert!(panel.filterable);
    // Has a header about no tree
    let has_header = panel
        .items
        .iter()
        .any(|i| matches!(i, PanelItem::Header(_)));
    assert!(has_header);
}

#[test]
fn session_list_builds_with_sections() {
    let sessions = vec![
        SessionRow {
            id: "sys1".into(),
            display_name: "Scheduled Tasks".into(),
            summary: Some("Periodic automation".into()),
            message_count: 10,
            is_starred: false,
            is_system: true,
        },
        SessionRow {
            id: "star1".into(),
            display_name: "Important Chat".into(),
            summary: Some("Key discussion".into()),
            message_count: 5,
            is_starred: true,
            is_system: false,
        },
        SessionRow {
            id: "reg1".into(),
            display_name: "Regular Session".into(),
            summary: Some("Just a chat".into()),
            message_count: 3,
            is_starred: false,
            is_system: false,
        },
    ];

    let stack = session_list(sessions);
    let panel = stack.current().unwrap();
    assert!(panel.filterable);
    assert!(panel.keep_open_on_activate);

    // Has 3 headers: System, Starred, Recent
    let headers: Vec<_> = panel
        .items
        .iter()
        .filter(|i| matches!(i, PanelItem::Header(_)))
        .collect();
    assert_eq!(headers.len(), 3);

    // Has 3 session items (Action variants)
    let actions: Vec<_> = panel
        .items
        .iter()
        .filter(|i| matches!(i, PanelItem::Action { .. }))
        .collect();
    assert_eq!(actions.len(), 3);
}

#[test]
fn session_list_empty_shows_message() {
    let stack = session_list(vec![]);
    let panel = stack.current().unwrap();
    assert!(panel.filterable);
    let has_header = panel
        .items
        .iter()
        .any(|i| matches!(i, PanelItem::Header(_)));
    assert!(has_header);
}
