# Session Info (/session)

**Status**: todo
**Milestone**: R2
**Category**: Sessions

## Description

Show session metadata and statistics: message count, token usage, duration, model history.

## Architecture

```rust
fn cmd_session(_args: &str) -> Option<Event> {
    Some(Event::ShowSessionInfo)
}

fn update_show_session_info(state: &AppState) -> String {
    let total_tokens: usize = state.messages.iter().map(estimate_tokens).sum();
    let msg_count = state.messages.len();
    let user_msgs = state.messages.iter().filter(|m| m.role == Role::User).count();
    let assistant_msgs = state.messages.iter().filter(|m| m.role == Role::Assistant).count();
    let tool_msgs = state.messages.iter().filter(|m| m.role == Role::Tool).count();

    format!(
        "Session: {}\n\
         Messages: {} total ({} user, {} assistant, {} tool)\n\
         Tokens: {} estimated\n\
         Provider: {}\n\
         Model: {}\n\
         Created: {}\n\
         Updated: {}",
        state.session_display_name.as_deref().unwrap_or("unnamed"),
        msg_count, user_msgs, assistant_msgs, tool_msgs,
        total_tokens,
        state.current_provider,
        state.current_model,
        format_timestamp(state.session_created_at),
        format_timestamp(state.session_updated_at),
    )
}
```

## Acceptance Criteria

- [ ] `/session` shows session metadata
- [ ] Message counts by role
- [ ] Estimated token count
- [ ] Provider and model
- [ ] Creation and update timestamps

## Tests

### Layer 1
- [ ] `session_info_counts_messages` — correct role counts
- [ ] `session_info_shows_tokens` — token estimate present
