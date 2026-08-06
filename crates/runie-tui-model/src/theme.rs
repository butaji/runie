//! Renderer-independent semantic theme vocabulary.

/// Tokens shared by declarative paint intents and terminal renderers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeToken {
    TextPrimary,
    TextMuted,
    TextFooterKey,
    TextAssistant,
    TextHeaderPath,
    AccentPrimary,
    AccentSecondary,
    Success,
    Error,
    Warning,
    BackgroundBase,
    BackgroundPanel,
    BackgroundDiffDelete,
    BackgroundDiffInsert,
    BackgroundSelection,
    BorderPrompt,
    BorderSelection,
}

impl ThemeToken {
    pub const fn opaline_name(self) -> &'static str {
        match self {
            Self::TextPrimary => "text.primary",
            Self::TextMuted => "text.muted",
            Self::TextFooterKey => "text.footer_key",
            Self::TextAssistant => "text.assistant",
            Self::TextHeaderPath => "text.header_path",
            Self::AccentPrimary => "accent.primary",
            Self::AccentSecondary => "accent.secondary",
            Self::Success => "success",
            Self::Error => "error",
            Self::Warning => "warning",
            Self::BackgroundBase => "bg.base",
            Self::BackgroundPanel => "bg.panel",
            Self::BackgroundDiffDelete => "bg.diff_delete",
            Self::BackgroundDiffInsert => "bg.diff_insert",
            Self::BackgroundSelection => "bg.selection",
            Self::BorderPrompt => "border.prompt",
            Self::BorderSelection => "border.selection",
        }
    }
}
