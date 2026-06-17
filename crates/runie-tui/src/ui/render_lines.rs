//! Per-element line rendering with caching.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, LazyLock, Mutex};

use ratatui::text::Line;
use ratatui::widgets::{Paragraph, Wrap};
use crate::core_ui::Element;

use crate::message as msg;

type CacheKey = (u64, u16, String);
type CacheEntry = (Vec<Line<'static>>, usize);
static RENDER_CACHE: LazyLock<Mutex<HashMap<CacheKey, Arc<CacheEntry>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Per-element rendered lines / markdown blocks are cached by `(content_hash, content_width)`.
/// Only elements with stable, hashable content are cached; dynamic indicators are re-rendered.
pub fn element_line_count(elem: &Element, content_width: u16) -> usize {
    to_lines_and_count(elem, content_width).1
}

/// Render an element to terminal lines, reusing a cached result when the
/// content and width are unchanged.
#[allow(dead_code)]
pub fn to_lines_internal(elem: &Element, content_width: u16) -> Vec<Line<'static>> {
    to_lines_and_count(elem, content_width).0
}

/// Render an element and return both the lines and the number of terminal
/// rows Ratatui will use after its wrap pass.
pub fn to_lines_and_count(elem: &Element, content_width: u16) -> (Vec<Line<'static>>, usize) {
    let theme = crate::theme::current_theme_name();
    if let Some(key) = cache_key(elem) {
        let cache_key = (key, content_width, theme);
        if let Some(cached) = cached_entry(&cache_key) {
            return (cached.0.clone(), cached.1);
        }
        let lines = render_element(elem, content_width);
        let count = wrapped_row_count(&lines, content_width);
        if let Ok(mut guard) = RENDER_CACHE.lock() {
            guard.insert(cache_key, Arc::new((lines.clone(), count)));
        }
        return (lines, count);
    }
    let lines = render_element(elem, content_width);
    let count = wrapped_row_count(&lines, content_width);
    (lines, count)
}

fn cached_entry(key: &CacheKey) -> Option<Arc<CacheEntry>> {
    let guard = RENDER_CACHE.lock().ok()?;
    guard.get(key).map(Arc::clone)
}

fn wrapped_row_count(lines: &[Line<'_>], width: u16) -> usize {
    Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .line_count(width)
}

fn render_element(elem: &Element, content_width: u16) -> Vec<Line<'static>> {
    use runie_core::Element::*;
    match elem {
        Spacer { .. } => vec![Line::from("")],
        UserMessage { content, timestamp } => {
            msg::render_user_message(content, *timestamp, content_width)
        }
        AgentMessage {
            content, timestamp, ..
        } => msg::render_agent_message(content, *timestamp, content_width),
        Thinking { started, .. } => msg::render_thinking(*started),
        ThoughtSummary {
            content,
            duration_secs,
            ..
        } => msg::render_thought_summary(content, *duration_secs),
        ThoughtMarker { content, .. } => msg::render_thought_marker(content, content_width),
        ToolRunning { name, args, started, .. } => {
            msg::render_tool_running(name, args, started.elapsed().as_secs_f64())
        }
        ToolDone {
            name,
            args,
            duration_secs,
            output,
            bytes_transferred,
            error,
            ..
        } => msg::render_tool_done(name, args, *duration_secs, output, *bytes_transferred, *error),
        ToolSummary {
            name,
            duration_secs,
            ..
        } => msg::render_tool_summary(name, "", *duration_secs),
        TurnComplete { duration_secs, .. } => msg::render_turn_complete(*duration_secs),
    }
}

fn cache_key(elem: &Element) -> Option<u64> {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    match elem {
        Element::UserMessage { content, timestamp } => {
            hash_text(&mut hasher, "UserMessage", content, *timestamp)
        }
        Element::AgentMessage {
            content,
            timestamp,
            provider,
        } => hash_agent(&mut hasher, content, *timestamp, provider),
        Element::ThoughtMarker { content, timestamp } => {
            hash_text(&mut hasher, "ThoughtMarker", content, *timestamp)
        }
        Element::ThoughtSummary {
            content,
            duration_secs,
            timestamp,
        } => hash_summary(&mut hasher, content, *timestamp, *duration_secs),
        Element::ToolDone {
            name,
            args: _,
            duration_secs,
            output,
            bytes_transferred: _,
            error: _,
            timestamp,
        } => hash_tool_done(&mut hasher, name, *timestamp, output, *duration_secs),
        _ => return None,
    }
    Some(hasher.finish())
}

fn hash_text(
    hasher: &mut std::collections::hash_map::DefaultHasher,
    disc: &str,
    content: &str,
    timestamp: f64,
) {
    disc.hash(hasher);
    content.hash(hasher);
    timestamp.to_bits().hash(hasher);
}

fn hash_agent(
    hasher: &mut std::collections::hash_map::DefaultHasher,
    content: &str,
    timestamp: f64,
    provider: &str,
) {
    "AgentMessage".hash(hasher);
    content.hash(hasher);
    provider.hash(hasher);
    timestamp.to_bits().hash(hasher);
}

fn hash_summary(
    hasher: &mut std::collections::hash_map::DefaultHasher,
    content: &str,
    timestamp: f64,
    duration_secs: f64,
) {
    "ThoughtSummary".hash(hasher);
    content.hash(hasher);
    duration_secs.to_bits().hash(hasher);
    timestamp.to_bits().hash(hasher);
}

fn hash_tool_done(
    hasher: &mut std::collections::hash_map::DefaultHasher,
    name: &str,
    timestamp: f64,
    output: &str,
    duration_secs: f64,
) {
    "ToolDone".hash(hasher);
    name.hash(hasher);
    output.hash(hasher);
    duration_secs.to_bits().hash(hasher);
    timestamp.to_bits().hash(hasher);
}

#[cfg(test)]
pub(crate) fn cache_len() -> usize {
    RENDER_CACHE.lock().map_or(0, |g| g.len())
}

#[cfg(test)]
pub(crate) fn clear_cache() {
    if let Ok(mut guard) = RENDER_CACHE.lock() {
        guard.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use runie_core::ui::elements::Element;
    use std::sync::Mutex;

    static CACHE_TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn element_render_cache_hits_for_same_width_and_content() {
        let _guard = CACHE_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        clear_cache();
        clear_cache();
        let before = cache_len();
        let elem = Element::agent("unique cache test content").at(0.0);
        let _ = to_lines_internal(&elem, 80);
        let after_first = cache_len();
        assert!(after_first > before, "first render should populate cache");
        let _ = to_lines_internal(&elem, 80);
        assert_eq!(
            cache_len(),
            after_first,
            "second render should be a cache hit"
        );
    }

    #[test]
    fn long_feed_renders_in_reasonable_time() {
        let _guard = CACHE_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        clear_cache();
        let elements: Vec<Element> = (0..50)
            .map(|i| {
                Element::agent(format!("line {}\n```rust\nlet x = {i};\n```\n- a\n- b", i)).at(0.0)
            })
            .collect();

        // Warm cache.
        for elem in &elements {
            let _ = to_lines_internal(elem, 80);
        }
        let cached_count = cache_len();
        assert!(
            cached_count >= elements.len(),
            "all elements should be cached"
        );

        let start = std::time::Instant::now();
        for elem in &elements {
            let _ = to_lines_internal(elem, 80);
        }
        let elapsed = start.elapsed();
        assert!(
            elapsed.as_millis() < 50,
            "cached feed render took too long: {:?}",
            elapsed
        );
    }
}
