//! Renderer-independent semantic theme vocabulary.

/// Tokens shared by declarative paint intents and terminal renderers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeToken {
    TextPrimary,
    TextMuted,
    TextFooterKey,
    TextAssistant,
    TextHeaderPath,
    TextHeaderMeter,
    TextModel,
    AccentPrimary,
    AccentSecondary,
    AccentThought,
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
            Self::TextHeaderMeter => "text.header_meter",
            Self::TextModel => "text.model",
            Self::AccentPrimary => "accent.primary",
            Self::AccentSecondary => "accent.secondary",
            Self::AccentThought => "accent.thought",
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

#[cfg(test)]
mod tests {
    use super::ThemeToken;

    #[test]
    fn semantic_tokens_have_stable_opaline_names() {
        let tokens = [
            (ThemeToken::TextPrimary, "text.primary"),
            (ThemeToken::TextMuted, "text.muted"),
            (ThemeToken::AccentPrimary, "accent.primary"),
            (ThemeToken::BackgroundBase, "bg.base"),
            (ThemeToken::BackgroundPanel, "bg.panel"),
            (ThemeToken::BorderPrompt, "border.prompt"),
            (ThemeToken::BorderSelection, "border.selection"),
        ];
        for (token, expected) in tokens {
            assert_eq!(token.opaline_name(), expected);
        }
    }
}
