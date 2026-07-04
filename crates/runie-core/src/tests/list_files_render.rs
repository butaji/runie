//! Unit test reproducing list_files tool result rendering.

#[cfg(test)]
mod tests {
    use crate::model::AppState;
    use crate::Event;

    #[test]
    fn tool_end_renders_tool_done_header() {
        let mut state = AppState::default();
        let config = crate::config::Config::default();
        state.apply_config(&config);

        state.update(Event::UserMessageSubmitted {
            id: "req.0".into(),
            content: "list files".into(),
        });
        state.update(Event::Thinking { id: "req.0".into() });
        state.update(Event::ThoughtDone { id: "req.0".into() });
        state.update(Event::ToolStart {
            id: "req.0".into(),
            name: "list_dir".into(),
            input: serde_json::json!({"path": "."}),
        });
        state.update(Event::tool_end(
            "req.0",
            0.1,
            "Cargo.lock\nCargo.toml\nsrc/",
        ));

        state.ensure_fresh();
        let elements = &state.view().cached_feed.as_ref().unwrap().elements;
        let text: String = elements.iter().map(|e| format!("{:?}\n", e)).collect();
        println!("{text}");
        assert!(
            text.contains("list_dir"),
            "feed should contain tool name:\n{text}"
        );
    }
}
