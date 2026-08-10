//! Pure sticky prompt-header layout math.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PromptDescriptor {
    pub entry_idx: usize,
    pub y_virtual: usize,
    pub full_height: u16,
    pub min_height: u16,
    pub sticky: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderedPrompt {
    pub entry_idx: usize,
    pub render_height: u16,
    pub clip_top: u16,
}

impl RenderedPrompt {
    pub const fn visible_height(self) -> u16 {
        self.render_height.saturating_sub(self.clip_top)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StickyHeaderLayout {
    pub pushed: Option<RenderedPrompt>,
    pub pinned: Option<RenderedPrompt>,
}

impl StickyHeaderLayout {
    pub fn header_screen_rows(&self) -> u16 {
        let pushed = self.pushed.map_or(0, RenderedPrompt::visible_height);
        let pinned = self.pinned.map_or(0, RenderedPrompt::visible_height);
        if pushed == 0 && pinned == 0 {
            0
        } else if self.pushed.is_some() && self.pinned.is_none() {
            pushed
        } else {
            pushed + if pushed > 0 && pinned > 0 { 1 } else { 0 } + pinned + 1
        }
    }

    pub fn scroll_for_content(&self, scroll_offset: usize) -> usize {
        scroll_offset + self.header_screen_rows() as usize
    }
}

const HEADER_GAP: u16 = 1;
pub const MIN_PINNED_HEIGHT: u16 = 4;

pub fn compute_sticky_layout(
    scroll_offset: usize,
    prompts: &[PromptDescriptor],
) -> StickyHeaderLayout {
    if scroll_offset == 0 {
        return StickyHeaderLayout::default();
    }
    let Some(index) = prompts
        .iter()
        .rposition(|p| p.sticky && p.y_virtual < scroll_offset)
    else {
        return StickyHeaderLayout::default();
    };
    let prompt = prompts[index];
    let height = prompt
        .full_height
        .saturating_sub((scroll_offset - prompt.y_virtual) as u16)
        .max(prompt.min_height.max(MIN_PINNED_HEIGHT));
    let Some(next) = prompts.get(index + 1) else {
        return pinned_layout(prompt.entry_idx, height);
    };
    let next_row = next.y_virtual.saturating_sub(scroll_offset);
    if next_row == 0 || next_row > (height + HEADER_GAP) as usize {
        return pinned_layout(prompt.entry_idx, height);
    }
    let visible = (next_row as u16).saturating_sub(HEADER_GAP);
    if visible == 0 {
        return StickyHeaderLayout::default();
    }
    let render_height = prompt.full_height.min(height);
    StickyHeaderLayout {
        pushed: Some(RenderedPrompt {
            entry_idx: prompt.entry_idx,
            render_height,
            clip_top: render_height.saturating_sub(visible),
        }),
        pinned: None,
    }
}

fn pinned_layout(entry_idx: usize, height: u16) -> StickyHeaderLayout {
    StickyHeaderLayout {
        pushed: None,
        pinned: Some(RenderedPrompt {
            entry_idx,
            render_height: height,
            clip_top: 0,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const PROMPTS: &[PromptDescriptor] = &[
        PromptDescriptor {
            entry_idx: 1,
            y_virtual: 0,
            full_height: 6,
            min_height: 4,
            sticky: true,
        },
        PromptDescriptor {
            entry_idx: 2,
            y_virtual: 12,
            full_height: 5,
            min_height: 4,
            sticky: true,
        },
    ];

    #[test]
    fn prompt_pins_and_collapses() {
        let layout = compute_sticky_layout(3, PROMPTS);
        assert_eq!(layout.pinned.unwrap().render_height, 4);
        assert_eq!(layout.scroll_for_content(3), 8);
    }

    #[test]
    fn next_prompt_pushes_previous_header() {
        let layout = compute_sticky_layout(10, PROMPTS);
        let pushed = layout.pushed.unwrap();
        assert_eq!(pushed.entry_idx, 1);
        assert!(pushed.clip_top > 0);
        assert!(layout.pinned.is_none());
    }
}
