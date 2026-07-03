//! Session tree and session list dialog builders.

use super::{ItemAction, Panel, PanelItem, PanelStack};

use crate::Event;

/// A single session row for the session list dialog.
pub struct SessionRow {
    pub id: String,
    pub display_name: String,
    pub summary: Option<String>,
    pub message_count: usize,
    pub is_starred: bool,
    pub is_system: bool,
}

/// Build a session tree panel from a list of (depth, content, event).
pub fn session_tree(items: Vec<(usize, String, Event)>) -> PanelStack {
    let mut panel = Panel::new("session-tree", " Session Tree ").with_filter();
    if items.is_empty() {
        panel = panel.header("No session tree. Use /fork or /clone to create branches.");
    }
    for (_depth, content, evt) in items {
        let truncated: String = content.chars().take(50).collect();
        let label = if content.chars().count() > 50 {
            format!("{}…", truncated)
        } else {
            content
        };
        panel = panel.item(label, ItemAction::Emit(evt));
    }
    PanelStack::new(panel)
}

/// Build a session list panel with fuzzy search, star/unstar, rename, delete, and resume.
pub fn session_list(sessions: Vec<SessionRow>) -> PanelStack {
    if sessions.is_empty() {
        let panel = Panel::new("session-list", " Sessions ")
            .with_filter()
            .header("No sessions yet. Start a new conversation to create one.");
        return PanelStack::new(panel);
    }

    let mut panel = Panel::new("session-list", " Sessions ")
        .with_filter()
        .keep_open();
    panel = add_session_section(panel, &sessions, true, false, Some("System"));
    panel = add_session_section(panel, &sessions, false, true, Some("Starred"));
    let show_recent_header = sessions.iter().any(|s| s.is_system)
        || sessions.iter().any(|s| s.is_starred && !s.is_system);
    panel = add_session_section(
        panel,
        &sessions,
        false,
        false,
        if show_recent_header {
            Some("Recent")
        } else {
            None
        },
    );

    PanelStack::new(panel)
}

fn add_session_section(
    panel: Panel,
    sessions: &[SessionRow],
    is_system: bool,
    is_starred: bool,
    header: Option<&str>,
) -> Panel {
    let mut panel = panel;
    let mut first = true;
    for session in sessions {
        if !session_matches_section(session, is_system, is_starred) {
            continue;
        }
        if first {
            if let Some(h) = header {
                panel = panel.header(h);
            }
            first = false;
        }
        add_session_item(&mut panel, session);
    }
    if !first && (is_system || is_starred) {
        panel = panel.separator();
    }
    panel
}

fn session_matches_section(session: &SessionRow, is_system: bool, is_starred: bool) -> bool {
    if is_system {
        return session.is_system;
    }
    if is_starred {
        return session.is_starred && !session.is_system;
    }
    !session.is_starred && !session.is_system
}

fn add_session_item(panel: &mut Panel, session: &SessionRow) {
    let star = if session.is_starred { "★" } else { "☆" };
    let count_label = format!("[{} msgs]", session.message_count);

    // Format: ☆ name [N msgs] — summary
    let label = if let Some(summary) = &session.summary {
        if summary.is_empty() {
            format!("{} {} {}", star, session.display_name, count_label)
        } else {
            format!(
                "{} {} {} — {}",
                star, session.display_name, count_label, summary
            )
        }
    } else {
        format!("{} {} {}", star, session.display_name, count_label)
    };

    let id = session.id.clone();
    let evt = crate::Event::SelectSession { id };
    panel.items.push(PanelItem::Action {
        label,
        action: ItemAction::Emit(evt),
    });
}
