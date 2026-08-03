//! Pure sticky prompt layout math ported from Grok's scrollback pager.
//!
//! Rendering remains in `messages`; keeping this calculation independent makes
//! the push/collapse transitions deterministic and testable without a terminal.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PromptDescriptor {
    pub entry_idx: usize,
    pub y_virtual: usize,
    pub full_height: u16,
    pub min_height: u16,
    pub sticky: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RenderedPrompt {
    pub entry_idx: usize,
    pub render_height: u16,
    pub clip_top: u16,
}

impl RenderedPrompt {
    pub(crate) fn visible_height(self) -> u16 {
        self.render_height.saturating_sub(self.clip_top)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct StickyHeaderLayout {
    pub pushed: Option<RenderedPrompt>,
    pub pinned: Option<RenderedPrompt>,
}

impl StickyHeaderLayout {
    pub(crate) fn screen_rows(&self) -> u16 {
        let pushed = self.pushed.map_or(0, RenderedPrompt::visible_height);
        let pinned = self.pinned.map_or(0, RenderedPrompt::visible_height);
        let between = u16::from(pushed > 0 && pinned > 0);
        let after = u16::from(pinned > 0);
        pushed + between + pinned + after
    }
}

const HEADER_CONTENT_GAP: usize = 1;
pub(crate) const MIN_PINNED_HEIGHT: u16 = 4;

fn render_height(prompt: PromptDescriptor, scroll_offset: usize, viewport_height: u16) -> u16 {
    let past = scroll_offset.saturating_sub(prompt.y_virtual) as u16;
    prompt
        .full_height
        .saturating_sub(past)
        .max(prompt.min_height.max(1).min(prompt.full_height))
        .min(viewport_height)
}

pub(crate) fn compute(scroll_offset: usize, viewport_height: u16, prompts: &[PromptDescriptor]) -> StickyHeaderLayout {
    if scroll_offset == 0 || viewport_height == 0 {
        return StickyHeaderLayout::default();
    }
    let Some(pinned_idx) = prompts
        .iter()
        .rposition(|p| p.sticky && p.y_virtual < scroll_offset)
    else {
        return StickyHeaderLayout::default();
    };
    let prompt = prompts[pinned_idx];
    let height = render_height(prompt, scroll_offset, viewport_height);
    let Some(next) = prompts.get(pinned_idx + 1) else {
        return StickyHeaderLayout {
            pinned: Some(RenderedPrompt { entry_idx: prompt.entry_idx, render_height: height, clip_top: 0 }),
            ..Default::default()
        };
    };
    let next_row = next.y_virtual.saturating_sub(scroll_offset);
    if next_row > height as usize + HEADER_CONTENT_GAP {
        return StickyHeaderLayout {
            pinned: Some(RenderedPrompt { entry_idx: prompt.entry_idx, render_height: height, clip_top: 0 }),
            ..Default::default()
        };
    }
    let visible = next_row.saturating_sub(1) as u16;
    if visible == 0 {
        return StickyHeaderLayout::default();
    }
    let rendered = prompt.full_height.min(height);
    StickyHeaderLayout {
        pushed: Some(RenderedPrompt {
            entry_idx: prompt.entry_idx,
            render_height: rendered,
            clip_top: rendered.saturating_sub(visible),
        }),
        pinned: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prompt(entry_idx: usize, y_virtual: usize, full_height: u16) -> PromptDescriptor {
        PromptDescriptor { entry_idx, y_virtual, full_height, min_height: MIN_PINNED_HEIGHT, sticky: true }
    }

    #[test]
    fn pins_and_gradually_collapses_a_scrolled_prompt() {
        let prompts = [prompt(0, 0, 10), prompt(1, 30, 8)];
        assert_eq!(compute(2, 20, &prompts).pinned.unwrap().render_height, 8);
        assert_eq!(compute(8, 20, &prompts).pinned.unwrap().render_height, 4);
    }

    #[test]
    fn next_prompt_pushes_current_header_from_the_top() {
        let prompts = [prompt(0, 0, 8), prompt(1, 10, 8)];
        let layout = compute(7, 20, &prompts);
        let pushed = layout.pushed.unwrap();
        assert_eq!(pushed.entry_idx, 0);
        assert_eq!(pushed.visible_height(), 2);
        assert!(layout.pinned.is_none());
    }

    #[test]
    fn no_header_is_left_when_only_the_gap_is_visible() {
        let prompts = [prompt(0, 0, 8), prompt(1, 10, 8)];
        assert_eq!(compute(9, 20, &prompts), StickyHeaderLayout::default());
    }

    #[test]
    fn header_rows_include_only_real_gaps() {
        let prompts = [prompt(0, 0, 8), prompt(1, 30, 8)];
        let pinned = compute(2, 20, &prompts);
        assert_eq!(
            pinned.screen_rows(),
            pinned.pinned.unwrap().visible_height() + 1
        );
        let pushed = compute(7, 20, &[prompt(0, 0, 8), prompt(1, 10, 8)]);
        assert_eq!(
            pushed.screen_rows(),
            pushed.pushed.unwrap().visible_height()
        );
    }
}
