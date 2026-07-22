use super::{ItemAction, Panel, PanelItem, PanelStack};

fn sample_panel() -> Panel {
    Panel::new("root", "Settings")
        .header("Appearance")
        .toggle("Dark mode", true, ItemAction::Toggle("dark_mode".into()))
        .select(
            "Theme",
            "runie",
            vec!["runie".into(), "dracula".into(), "nord".into()],
            "theme",
        )
        .separator()
        .header("Behavior")
        .item("Open advanced", ItemAction::Push("advanced".into()))
}

// ─── PanelStack ─────────────────────────────────────────────────────────

#[test]
fn panel_stack_starts_with_root() {
    let root = Panel::new("root", "Main");
    let stack = PanelStack::new(root.clone());
    assert_eq!(stack.len(), 1);
    assert_eq!(stack.current().unwrap().id, "root");
}

#[test]
fn panel_stack_push_adds_panel() {
    let root = Panel::new("root", "Main");
    let mut stack = PanelStack::new(root);
    stack.push(Panel::new("sub", "Sub"));
    assert_eq!(stack.len(), 2);
    assert_eq!(stack.current().unwrap().id, "sub");
}

#[test]
fn panel_stack_pop_removes_top() {
    let root = Panel::new("root", "Main");
    let mut stack = PanelStack::new(root);
    stack.push(Panel::new("sub", "Sub"));
    let popped = stack.pop();
    assert!(popped.is_some());
    assert_eq!(stack.len(), 1);
    assert_eq!(stack.current().unwrap().id, "root");
}

#[test]
fn panel_stack_pop_root_returns_none() {
    let root = Panel::new("root", "Main");
    let mut stack = PanelStack::new(root);
    assert!(stack.pop().is_none());
    assert_eq!(stack.len(), 1);
}

#[test]
fn panel_stack_breadcrumb_shows_path() {
    let root = Panel::new("root", "Main");
    let mut stack = PanelStack::new(root);
    stack.push(Panel::new("sub", "Sub"));
    let crumbs = stack.breadcrumb();
    assert_eq!(crumbs, vec!["Main", "Sub"]);
}

// ─── Panel navigation ───────────────────────────────────────────────────

#[test]
fn panel_select_down_wraps() {
    let mut panel = sample_panel();
    panel.select_down();
    assert_eq!(panel.selected, 1);
    panel.select_down();
    assert_eq!(panel.selected, 2);
    panel.select_down();
    assert_eq!(panel.selected, 0); // wraps
}

#[test]
fn panel_select_up_wraps() {
    let mut panel = sample_panel();
    panel.selected = 0;
    panel.select_up();
    assert_eq!(panel.selected, 2); // wraps to last navigable
}

#[test]
fn panel_navigable_count_ignores_headers_and_separators() {
    let panel = sample_panel();
    // Headers (2) + Toggle (1) + Select (1) + Separator (1) + Action (1) = 6 items
    // Navigable: Toggle + Select + Action = 3
    assert_eq!(panel.navigable_count(), 3);
}

#[test]
fn panel_raw_index_maps_nav_to_raw() {
    let panel = sample_panel();
    // item[0] = Header("Appearance") -> not navigable
    // item[1] = Toggle -> nav 0
    // item[2] = Select -> nav 1
    // item[3] = Separator -> not navigable
    // item[4] = Header("Behavior") -> not navigable
    // item[5] = Action -> nav 2
    assert_eq!(panel.raw_index(0), Some(1)); // Toggle
    assert_eq!(panel.raw_index(1), Some(2)); // Select
    assert_eq!(panel.raw_index(2), Some(5)); // Action
}

#[test]
fn panel_selected_item_returns_correct_item() {
    let mut panel = sample_panel();
    panel.selected = 1;
    let item = panel.selected_item();
    assert!(matches!(item, Some(PanelItem::Select { .. })));
}

// ─── Panel builder ──────────────────────────────────────────────────────

#[test]
#[allow(clippy::cognitive_complexity)]
fn panel_builder_chains() {
    let panel = Panel::new("test", "Test")
        .item("Do thing", ItemAction::Close)
        .toggle("Flag", false, ItemAction::Toggle("flag".into()))
        .select("Choice", "a", vec!["a".into(), "b".into()], "choice")
        .header("Section")
        .separator();

    assert_eq!(panel.items.len(), 5);
    assert!(matches!(panel.items[0], PanelItem::Action { .. }));
    assert!(matches!(panel.items[1], PanelItem::Toggle { .. }));
    assert!(matches!(panel.items[2], PanelItem::Select { .. }));
    assert!(matches!(panel.items[3], PanelItem::Header(_)));
    assert!(matches!(panel.items[4], PanelItem::Separator));
}

#[test]
fn panel_filter_tracks_text() {
    let mut panel = Panel::new("test", "Test")
        .with_filter()
        .item("abc", ItemAction::Close)
        .item("def", ItemAction::Close);
    panel.push_filter('a');
    assert_eq!(panel.filter, "a");
    panel.pop_filter();
    assert_eq!(panel.filter, "");
}

// ─── PanelStack activate ────────────────────────────────────────────────

#[test]
fn stack_pop_protects_root() {
    let mut stack = PanelStack::new(Panel::new("root", "Root"));
    assert_eq!(stack.len(), 1);
    assert!(stack.pop().is_none(), "pop on root must be a no-op");
    assert_eq!(stack.len(), 1);
}

#[test]
fn stack_push_then_pop_returns_top() {
    let mut stack = PanelStack::new(Panel::new("root", "Root"));
    stack.push(Panel::new("child", "Child"));
    assert_eq!(stack.len(), 2);
    let popped = stack.pop().expect("must pop child");
    assert_eq!(popped.id, "child");
    assert_eq!(stack.len(), 1);
    assert_eq!(stack.current().unwrap().id, "root");
}

#[test]
fn stack_activate_on_toggle_returns_toggle_action() {
    let mut stack =
        PanelStack::new(Panel::new("test", "Test").toggle("Flag", false, ItemAction::Toggle("flag_key".into())));
    let action = stack.activate();
    assert!(matches!(action, Some(ItemAction::Toggle(k)) if k == "flag_key"));
}

#[test]
fn stack_activate_on_select_returns_cycle_action() {
    let mut stack =
        PanelStack::new(Panel::new("test", "Test").select("Choice", "a", vec!["a".into(), "b".into()], "choice_key"));
    let action = stack.activate();
    assert!(matches!(action, Some(ItemAction::Cycle(k)) if k == "choice_key"));
}

#[test]
fn stack_activate_on_action_returns_clone() {
    let mut stack = PanelStack::new(Panel::new("test", "Test").item("Close", ItemAction::Close));
    let action = stack.activate();
    assert!(matches!(action, Some(ItemAction::Close)));
}

// ─── Panel filtering ────────────────────────────────────────────────────

#[test]
fn panel_filtered_items_shows_all_when_no_filter() {
    let panel = Panel::new("test", "Test")
        .with_filter()
        .header("Section")
        .item("alpha", ItemAction::Close)
        .item("beta", ItemAction::Close);
    let filtered = panel.filtered_items();
    assert_eq!(filtered.len(), 3, "No filter = all items visible");
}

#[test]
fn panel_filtered_items_hides_non_matching() {
    let mut panel = Panel::new("test", "Test")
        .with_filter()
        .header("Section")
        .item("alpha", ItemAction::Close)
        .item("xyz", ItemAction::Close);
    panel.push_filter('a');
    let filtered = panel.filtered_items();
    assert_eq!(
        filtered.len(),
        1,
        "Only matching navigable items when filtering"
    );
    assert!(filtered
        .iter()
        .any(|i| matches!(i, PanelItem::Action { label, .. } if label == "alpha")));
    assert!(!filtered
        .iter()
        .any(|i| matches!(i, PanelItem::Action { label, .. } if label == "xyz")));
}

#[test]
fn panel_filtered_items_is_case_insensitive() {
    let mut panel = Panel::new("test", "Test")
        .with_filter()
        .item("ALPHA", ItemAction::Close)
        .item("beta", ItemAction::Close);
    panel.push_filter('a');
    let filtered = panel.filtered_items();
    assert!(filtered
        .iter()
        .any(|i| matches!(i, PanelItem::Action { label, .. } if label == "ALPHA")));
}

#[test]
fn panel_has_filter_matches_reflects_filter() {
    let mut panel = Panel::new("test", "Test")
        .with_filter()
        .item("alpha", ItemAction::Close);
    assert!(panel.has_filter_matches(), "empty filter always matches");
    panel.push_filter('a');
    assert!(panel.has_filter_matches());
    panel.push_filter('z');
    assert!(!panel.has_filter_matches());
}

#[test]
fn panel_filtered_items_falls_back_to_all_navigable_when_nothing_matches() {
    let mut panel = Panel::new("test", "Test")
        .with_filter()
        .item("alpha", ItemAction::Close)
        .item("beta", ItemAction::Close);
    panel.push_filter('z');
    // Fallback keeps navigation usable for pickers; rendering uses
    // has_filter_matches() to show an empty state instead.
    assert_eq!(panel.filtered_items().len(), 2);
    assert!(!panel.has_filter_matches());
}

#[test]
fn panel_filtered_items_drops_headers_when_filtering() {
    let mut panel = Panel::new("test", "Test")
        .with_filter()
        .header("Group A")
        .item("alice", ItemAction::Close)
        .header("Group B")
        .item("bob", ItemAction::Close);
    panel.push_filter('a');
    let filtered = panel.filtered_items();
    assert_eq!(filtered.len(), 1, "Headers dropped when filtering");
    assert!(filtered
        .iter()
        .any(|i| matches!(i, PanelItem::Action { label, .. } if label == "alice")));
    assert!(!filtered
        .iter()
        .any(|i| matches!(i, PanelItem::Header(t) if t == "Group A")));
    assert!(!filtered
        .iter()
        .any(|i| matches!(i, PanelItem::Header(t) if t == "Group B")));
}

// ─── Accelerator parsing ────────────────────────────────────────────────

#[test]
fn parse_accel_finds_letter() {
    assert_eq!(super::parse_accel("_Submit"), Some('S'));
    assert_eq!(super::parse_accel("_Cancel"), Some('C'));
    assert_eq!(super::parse_accel("Sa_ve"), Some('v'));
}

#[test]
fn parse_accel_none_when_no_underscore() {
    assert_eq!(super::parse_accel("Submit"), None);
    assert_eq!(super::parse_accel(""), None);
}

#[test]
fn parse_accel_none_when_underscore_last() {
    assert_eq!(super::parse_accel("Submit_"), None);
}

#[test]
fn strip_accel_removes_underscore() {
    assert_eq!(super::strip_accel("_Submit"), "Submit");
    assert_eq!(super::strip_accel("_Cancel"), "Cancel");
    assert_eq!(super::strip_accel("Submit"), "Submit");
}

#[test]
fn find_button_by_accel_matches_case_insensitive() {
    let panel = Panel::new("test", "Test")
        .item("_Submit", ItemAction::Close)
        .item("_Cancel", ItemAction::Push("back".into()));
    assert!(matches!(
        panel.find_button_by_accel('S'),
        Some(ItemAction::Close)
    ));
    assert!(matches!(
        panel.find_button_by_accel('s'),
        Some(ItemAction::Close)
    ));
    assert!(matches!(
        panel.find_button_by_accel('C'),
        Some(ItemAction::Push(_))
    ));
    assert!(panel.find_button_by_accel('X').is_none());
}

// ─── Panel closable DSL ─────────────────────────────────────────────────

#[test]
fn panel_defaults_to_closable() {
    let panel = Panel::new("root", "Root");
    assert!(panel.closable);
}

#[test]
fn panel_closable_builder_sets_false() {
    let panel = Panel::new("root", "Root").closable(false);
    assert!(!panel.closable);
}

#[test]
fn panel_non_closable_alias_sets_false() {
    let panel = Panel::new("root", "Root").non_closable();
    assert!(!panel.closable);
}

#[test]
fn panel_stack_root_closable_preserved() {
    let root = Panel::new("root", "Root").non_closable();
    let stack = PanelStack::new(root);
    assert_eq!(stack.root().map(|p| p.closable), Some(false));
}

#[test]
fn panel_title_has_exactly_one_leading_and_trailing_space() {
    let panel = Panel::new("test", "Settings");
    assert_eq!(panel.title, " Settings ");
}

#[test]
fn panel_title_trims_extra_whitespace() {
    let panel = Panel::new("test", "  Settings  ");
    assert_eq!(panel.title, " Settings ");
}

#[test]
fn panel_with_title_normalizes() {
    let panel = Panel::new("test", "Old").with_title("New");
    assert_eq!(panel.title, " New ");
}

#[test]
fn panel_title_alias_normalizes() {
    let panel = Panel::new("test", "Old").title("New");
    assert_eq!(panel.title, " New ");
}

#[test]
fn panel_empty_title_stays_empty() {
    let panel = Panel::new("test", "   ");
    assert_eq!(panel.title, "");
}
