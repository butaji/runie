use super::{Panel, PanelItem, PanelStack, ItemAction};

fn sample_panel() -> Panel {
    Panel::new("root", "Settings")
        .header("Appearance")
        .toggle("Dark mode", true, "dark_mode")
        .select("Theme", "runie", vec!["runie".into(), "dracula".into(), "nord".into()], "theme")
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
fn panel_builder_chains() {
    let panel = Panel::new("test", "Test")
        .item("Do thing", ItemAction::Close)
        .toggle("Flag", false, "flag")
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
fn stack_activate_on_toggle_returns_toggle_action() {
    let mut stack = PanelStack::new(
        Panel::new("test", "Test").toggle("Flag", false, "flag_key")
    );
    let action = stack.activate();
    assert!(matches!(action, Some(ItemAction::Toggle(k)) if k == "flag_key"));
}

#[test]
fn stack_activate_on_select_returns_cycle_action() {
    let mut stack = PanelStack::new(
        Panel::new("test", "Test").select("Choice", "a", vec!["a".into(), "b".into()], "choice_key")
    );
    let action = stack.activate();
    assert!(matches!(action, Some(ItemAction::Cycle(k)) if k == "choice_key"));
}

#[test]
fn stack_activate_on_action_returns_clone() {
    let mut stack = PanelStack::new(
        Panel::new("test", "Test").item("Close", ItemAction::Close)
    );
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
    assert_eq!(filtered.len(), 2, "Header + matching 'alpha'");
    assert!(filtered.iter().any(|i| matches!(i, PanelItem::Action { label, .. } if label == "alpha")));
    assert!(!filtered.iter().any(|i| matches!(i, PanelItem::Action { label, .. } if label == "xyz")));
}

#[test]
fn panel_filtered_items_is_case_insensitive() {
    let mut panel = Panel::new("test", "Test")
        .with_filter()
        .item("ALPHA", ItemAction::Close)
        .item("beta", ItemAction::Close);
    panel.push_filter('a');
    let filtered = panel.filtered_items();
    assert!(filtered.iter().any(|i| matches!(i, PanelItem::Action { label, .. } if label == "ALPHA")));
}

#[test]
fn panel_filtered_items_keeps_header_with_match() {
    let mut panel = Panel::new("test", "Test")
        .with_filter()
        .header("Group A")
        .item("alice", ItemAction::Close)
        .header("Group B")
        .item("bob", ItemAction::Close);
    panel.push_filter('a');
    let filtered = panel.filtered_items();
    assert!(filtered.iter().any(|i| matches!(i, PanelItem::Header(t) if t == "Group A")));
    assert!(!filtered.iter().any(|i| matches!(i, PanelItem::Header(t) if t == "Group B")));
}
