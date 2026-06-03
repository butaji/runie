use ratatui::{buffer::Buffer, layout::Rect, style::Style};
use crate::theme::ThemeWrapper;
use crate::tui::state::AnimationState;
use crate::components::message_list::render::WrapCache;
pub mod types;
pub mod render;
pub mod builder;
pub mod feed;

// ─── Scrollbar ─────────────────────────────────────────────────────────────────

/// Grok-style scrollbar: thin vertical bar on right edge.
/// Only visible when content overflows the visible area.
fn render_scrollbar(
    scroll_offset: usize,
    total_items: usize,
    visible_rows: usize,
    area: Rect,
    buf: &mut Buffer,
    track_color: ratatui::style::Color,
    thumb_color: ratatui::style::Color,
) {
    let scrollbar_x = area.right().saturating_sub(1);
    if scrollbar_x < area.x {
        return; // No space for scrollbar
    }

    let scrollbar_height = area.height;
    if scrollbar_height < 2 {
        return; // Need at least 2 rows for a meaningful scrollbar
    }

    // Only show scrollbar if there's more content than visible rows
    // total_items is the total number of scrollable items
    // visible_rows is the number of rows that fit on screen
    if total_items <= visible_rows {
        return;
    }

    // Calculate thumb size proportionally: visible / total * scrollbar_height
    // This represents how much of the content is visible
    let thumb_height = if total_items > 0 {
        ((visible_rows as u32 * scrollbar_height as u32) / total_items as u32).max(1) as u16
    } else {
        1
    };

    // Calculate thumb position based on scroll ratio
    // max_scroll is the last valid scroll offset (when last item is at bottom of screen)
    let max_scroll = total_items.saturating_sub(visible_rows);
    let scroll_ratio = if max_scroll > 0 {
        (scroll_offset as f32 / max_scroll as f32).min(1.0)
    } else {
        0.0
    };

    // thumb_y positions the thumb within the scrollbar track
    // At scroll_ratio=0 (top), thumb is at top; at scroll_ratio=1 (bottom), thumb is at bottom
    let track_available = scrollbar_height.saturating_sub(thumb_height);
    let thumb_y = area.y + (scroll_ratio * track_available as f32) as u16;

    // Render track (│ character)
    let track_style = Style::default().fg(track_color);
    for y in area.y..area.y + scrollbar_height {
        buf.get_mut(scrollbar_x, y)
            .set_char('│')
            .set_style(track_style);
    }

    // Render thumb (█ block)
    let thumb_style = Style::default().fg(thumb_color);
    for y in thumb_y..thumb_y.saturating_add(thumb_height).min(area.y + scrollbar_height) {
        buf.get_mut(scrollbar_x, y)
            .set_char('█')
            .set_style(thumb_style);
    }
}

#[cfg(test)]
mod snapshots;
#[cfg(test)]
mod tests;

pub use types::{MessageItem, MessageList, PlanStatus};
pub(crate) use builder::FeedBuilder;
pub use feed::{Feed, FeedItem, Thought, ToolCall};
// Re-export thinking block types for external testing
pub use render::{ThinkingBlock, render_thinking_block};
// Re-export tool call block types for external testing
pub use render::{ToolCallBlock, ToolStatus, render_tool_call_block};

/// ViewModel for rendering MessageList
pub struct MessageListViewModel {
    pub feed: Feed,
    pub scroll_offset: usize,
    pub agent_running: bool,
    pub animation: AnimationState,
    pub wrap_cache: WrapCache,
    /// Timer for "⠼ Starting session… X.Xs" indicator shown after HomeScreen transition
    pub session_starting: Option<std::time::Instant>,
    /// Streaming thinking content (from state.thinking.text during agent streaming)
    pub streaming_think_content: Option<String>,
}

impl MessageListViewModel {

    #[must_use]

    pub fn new(feed: Feed, scroll_offset: usize, agent_running: bool, animation: AnimationState, wrap_cache: WrapCache, session_starting: Option<std::time::Instant>, streaming_think_content: Option<String>) -> Self {
        Self {
            feed,
            scroll_offset,
            agent_running,
            animation,
            wrap_cache,
            session_starting,
            streaming_think_content,
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
        let _rendered_rows = render_message_list(vm, area, buf, theme, &colors, spinner, rewind_spinner, &mut wrap_cache);

        // Compute scrollbar info
        // total_items = count of non-SystemNotice items (scrollable content)
        // visible_rows = actual visible capacity (area.height), NOT rendered_rows which can be inflated by wrapping
        let all_items: Vec<_> = vm.feed.items().iter().collect();
        let visible_items: Vec<_> = all_items.iter().filter(|item| !matches!(item, FeedItem::SystemNotice { .. })).collect();
        let total_items = visible_items.len();
        let visible_rows = area.height as usize;

        // Render Grok-style scrollbar on right edge
        // Show when content overflows visible area
        if total_items > visible_rows {
            render_scrollbar(
                vm.scroll_offset,
                total_items,
                visible_rows,
                area,
                buf,
                colors.text_dim,    // track color (subtle)
                colors.accent_secondary, // thumb color
            );
        }

        // Render session starting indicator (auto-timeout after 10 seconds)
        if let Some(start_time) = vm.session_starting {
            let elapsed = start_time.elapsed().as_secs_f64();
            if elapsed < 10.0 {
                let text_x = area.x + 5; // Grok-style 5-space indent
                render::render_session_starting(area, buf, colors.text_muted, text_x, elapsed, spinner);
            }
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
            vm.streaming_think_content.as_deref(),
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
    streaming_think_content: Option<&str>,
) -> u16 {
    render::render_single_msg_feed(
        item, area, row, margin_x, text_x, max_rows, buf, theme,
        colors.accent_primary, colors.accent_secondary, colors.text_secondary, colors.text_muted, colors.text_dim,
        colors.success, colors.error, colors.code_path, spinner, show_cursor, show_spinner, rewind_spinner,
        animation, wrap_cache, agent_running, thought_duration, turn_complete, is_last_item, thoughts_collapsed,
        streaming_think_content,
    )
}