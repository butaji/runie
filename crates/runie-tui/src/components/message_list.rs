use ratatui::{buffer::Buffer, layout::Rect};
use crate::theme::ThemeWrapper;
use crate::tui::state::AnimationState;
use crate::components::message_list::render::WrapCache;
pub mod types;
pub mod render;
pub mod builder;
pub mod feed;

#[cfg(test)]
mod snapshots;
#[cfg(test)]
mod tests;

pub use types::{MessageItem, MessageList, PlanStatus};
pub(crate) use builder::FeedBuilder;
pub use feed::{Feed, FeedItem, Thought, ToolCall};

/// ViewModel for rendering MessageList
pub struct MessageListViewModel {
    pub feed: Feed,
    pub scroll_offset: usize,
    pub agent_running: bool,
    pub animation: AnimationState,
    pub wrap_cache: WrapCache,
    /// Timer for "⠼ Starting session… X.Xs" indicator shown after HomeScreen transition
    pub session_starting: Option<std::time::Instant>,
}

impl MessageListViewModel {

    #[must_use]

    pub fn new(feed: Feed, scroll_offset: usize, agent_running: bool, animation: AnimationState, wrap_cache: WrapCache, session_starting: Option<std::time::Instant>) -> Self {
        Self {
            feed,
            scroll_offset,
            agent_running,
            animation,
            wrap_cache,
            session_starting,
        }
    }
}

impl MessageList {
    pub fn render_ref(
        vm: &MessageListViewModel,
        area: Rect,
        buf: &mut Buffer,
        theme: &ThemeWrapper,
    ) {
        let colors = extract_message_colors(theme);
        let mut wrap_cache = vm.wrap_cache.clone();
        let spinner = crate::glyphs::SPINNER_FRAMES[vm.animation.braille_frame % 10];
        let rewind_spinner = crate::glyphs::SPINNER_FRAMES_REVERSE[vm.animation.braille_frame % 10];
        let _row = render_message_list(vm, area, buf, theme, &colors, spinner, rewind_spinner, &mut wrap_cache);

        // Render session starting indicator
        if let Some(start_time) = vm.session_starting {
            let elapsed = start_time.elapsed().as_secs_f64();
            let text_x = area.x + 5; // Grok-style 5-space indent
            render::render_session_starting(area, buf, colors.text_muted, text_x, elapsed, spinner);
        }

        if vm.feed.is_empty() && !vm.agent_running {
            render::render_empty_state(area, buf, colors.text_muted, colors.text_dim, area.x + 4);
        }
    }

    pub fn update_last_assistant(&mut self, new_text: &str) {
        if let Some(last) = self.messages.last_mut() {
            if let MessageItem::Assistant { ref mut text, .. } = last {
                *text = new_text.to_string();
            }
        }
    }

    pub fn has_assistant_in_progress(&self) -> bool {
        matches!(self.messages.last(), Some(MessageItem::Assistant { .. }))
    }

    pub fn add_or_update_assistant(&mut self, text: &str, model: Option<String>) {
        if let Some(last) = self.messages.last_mut() {
            if let MessageItem::Assistant { text: ref mut existing_text, .. } = last {
                *existing_text = text.to_string();
                return;
            }
        }
        self.messages.push(MessageItem::Assistant {
            text: text.to_string(),
            model,
            timestamp: None,
            expanded: true,
            thought_duration: None,
            turn_duration: None,
        });
    }
}

pub(crate) struct MessageColors {
    pub accent_primary: ratatui::style::Color,
    pub accent_secondary: ratatui::style::Color,
    pub accent_tertiary: ratatui::style::Color,
    pub text_secondary: ratatui::style::Color,
    pub text_muted: ratatui::style::Color,
    pub text_dim: ratatui::style::Color,
    pub success: ratatui::style::Color,
    pub error: ratatui::style::Color,
    pub code_path: ratatui::style::Color,
}

fn extract_message_colors(theme: &ThemeWrapper) -> MessageColors {
    MessageColors {
        accent_primary: theme.color("accent.primary").into(),
        accent_secondary: theme.color("accent.secondary").into(),
        accent_tertiary: theme.color("accent.tertiary").into(),
        text_secondary: theme.color("text.secondary").into(),
        text_muted: theme.color("text.muted").into(),
        text_dim: theme.color("text.dim").into(),
        success: theme.color("success").into(),
        error: theme.color("error").into(),
        code_path: theme.color("code.path").into(),
    }
}

fn render_message_list(
    vm: &MessageListViewModel,
    area: Rect,
    buf: &mut Buffer,
    theme: &ThemeWrapper,
    colors: &MessageColors,
    spinner: char,
    rewind_spinner: char,
    wrap_cache: &mut WrapCache,
) -> u16 {
    let mut row = 0u16;
    let max_rows = area.height;
    let margin_x = area.x;
    let text_x = area.x + 3;

    // Filter out SystemNotice items for chat view (Grok-style: no "New session started" in scrollback)
    let items: Vec<_> = vm.feed.items().iter().collect();
    let visible_items: Vec<_> = items.iter().filter(|item| !matches!(item, FeedItem::SystemNotice { .. })).collect();
    let visible_count = visible_items.len();

    for (idx, item) in visible_items.iter().skip(vm.scroll_offset).enumerate() {
        if row >= max_rows { break; }
        let absolute_idx = vm.scroll_offset + idx;

        let show_cursor = render::should_show_cursor_feed(&vm.animation, vm.agent_running, absolute_idx, visible_count, item);
        let show_spinner = false; // Spinners for ToolRunning/PlanStep not rendered via Feed

        // For AssistantMessage, get thought duration from inline thoughts
        let thought_duration = if let FeedItem::AssistantMessage { thoughts, .. } = item {
            thoughts.first().map(|t| t.duration)
        } else {
            None
        };

        // Get turn_duration and thoughts_collapsed from AssistantMessage
        let turn_duration = if let FeedItem::AssistantMessage { turn_duration, .. } = item {
            *turn_duration
        } else {
            None
        };

        let thoughts_collapsed = if let FeedItem::AssistantMessage { thoughts_collapsed, .. } = item {
            *thoughts_collapsed
        } else {
            false
        };

        let is_last_item = absolute_idx == visible_count.saturating_sub(1);
        let rendered = render_single_msg_item(
            item, area, row, margin_x, text_x, max_rows, buf, theme, colors, spinner, show_cursor, show_spinner, rewind_spinner,
            &vm.animation, wrap_cache, vm.agent_running, thought_duration, turn_duration, is_last_item, thoughts_collapsed,
        );
        row += rendered;
        // Draw separator between items (not after last)
        if idx < visible_count.saturating_sub(1) && row < max_rows {
            let current_item = visible_items[absolute_idx];
            let next_item = visible_items[absolute_idx + 1];
            row += render::render_item_separator(area, row, buf, colors.text_muted, current_item, next_item);
        }
    }
    row
}

fn render_single_msg_item(
    item: &FeedItem,
    area: Rect,
    row: u16,
    margin_x: u16,
    text_x: u16,
    max_rows: u16,
    buf: &mut Buffer,
    theme: &ThemeWrapper,
    colors: &MessageColors,
    spinner: char,
    show_cursor: bool,
    show_spinner: bool,
    rewind_spinner: char,
    animation: &AnimationState,
    wrap_cache: &mut WrapCache,
    agent_running: bool,
    thought_duration: Option<f32>,
    turn_complete: Option<f32>,
    is_last_item: bool,
    thoughts_collapsed: bool,
) -> u16 {
    render::render_single_msg_feed(
        item, area, row, margin_x, text_x, max_rows, buf, theme,
        colors.accent_primary, colors.accent_secondary, colors.text_secondary, colors.text_muted, colors.text_dim,
        colors.success, colors.error, colors.code_path, spinner, show_cursor, show_spinner, rewind_spinner,
        animation, wrap_cache, agent_running, thought_duration, turn_complete, is_last_item, thoughts_collapsed,
    )
}