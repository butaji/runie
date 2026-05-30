//! ViewModel and rendering helpers for MessageList.

use ratatui::{buffer::Buffer, layout::Rect};
use crate::theme::ThemeWrapper;
use crate::tui::state::AnimationState;
use crate::components::message_list::render::WrapCache;
use crate::components::message_list::feed::FeedItem;
use super::types::{MessageItem, MessageList, BRAILLE_FRAMES, REVERSE_BRAILLE_FRAMES};

/// ViewModel for rendering MessageList
pub struct MessageListViewModel {
    pub feed: Feed,
    pub scroll_offset: usize,
    pub agent_running: bool,
    pub animation: AnimationState,
    pub wrap_cache: WrapCache,
}

impl MessageListViewModel {
    pub fn new(feed: Feed, scroll_offset: usize, agent_running: bool, animation: AnimationState, wrap_cache: WrapCache) -> Self {
        Self {
            feed,
            scroll_offset,
            agent_running,
            animation,
            wrap_cache,
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
        let spinner = BRAILLE_FRAMES[vm.animation.braille_frame % 10];
        let rewind_spinner = REVERSE_BRAILLE_FRAMES[vm.animation.braille_frame % 10];
        let mut row = render_message_list(vm, area, buf, theme, &colors, spinner, rewind_spinner, &mut wrap_cache);

        if vm.feed.is_empty() && !vm.agent_running {
            crate::components::message_list::render::render_empty_state(area, buf, colors.text_muted, colors.text_dim, area.x + 4);
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
        });
    }
}

struct MessageColors {
    accent_primary: ratatui::style::Color,
    text_secondary: ratatui::style::Color,
    text_muted: ratatui::style::Color,
    text_dim: ratatui::style::Color,
    success: ratatui::style::Color,
    error: ratatui::style::Color,
    code_path: ratatui::style::Color,
}

fn extract_message_colors(theme: &ThemeWrapper) -> MessageColors {
    MessageColors {
        accent_primary: theme.color("accent.primary").into(),
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
    let margin_x = area.x + 2;
    let text_x = area.x + 4;
    let total_items = vm.feed.len();

    for (idx, item) in vm.feed.items().iter().skip(vm.scroll_offset).enumerate() {
        if row >= max_rows { break; }
        let absolute_idx = vm.scroll_offset + idx;

        let show_cursor = crate::components::message_list::render::should_show_cursor_feed(&vm.animation, vm.agent_running, absolute_idx, total_items, item);
        let show_spinner = false; // Spinners for ToolRunning/PlanStep not rendered via Feed

        // For AssistantMessage, get thought duration from inline thoughts
        let thought_duration = if let FeedItem::AssistantMessage { thoughts, .. } = item {
            thoughts.first().map(|t| t.duration)
        } else {
            None
        };

        // Get turn_duration from AssistantMessage
        let turn_duration = if let FeedItem::AssistantMessage { turn_duration, .. } = item {
            *turn_duration
        } else {
            None
        };

        let rendered = render_single_msg_item(
            item, area, row, margin_x, text_x, max_rows, buf, theme, colors, spinner, show_cursor, show_spinner, rewind_spinner,
            &vm.animation, wrap_cache, vm.agent_running, thought_duration, turn_duration,
        );
        row += rendered;
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
) -> u16 {
    crate::components::message_list::render::render_single_msg_feed(
        item, area, row, margin_x, text_x, max_rows, buf, theme,
        colors.accent_primary, colors.text_secondary, colors.text_muted, colors.text_dim,
        colors.success, colors.error, colors.code_path, spinner, show_cursor, show_spinner, rewind_spinner,
        animation, wrap_cache, agent_running, thought_duration, turn_complete,
    )
}
