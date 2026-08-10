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

macro_rules! theme_tokens {
    ($(($token:ident, $name:literal)),+ $(,)?) => {
        impl ThemeToken {
            pub const fn opaline_name(self) -> &'static str {
                match self { $(Self::$token => $name,)+ }
            }
        }
    };
}

theme_tokens! {
    (TextPrimary, "text.primary"),
    (TextMuted, "text.muted"),
    (TextFooterKey, "text.footer_key"),
    (TextAssistant, "text.assistant"),
    (TextHeaderPath, "text.header_path"),
    (TextHeaderMeter, "text.header_meter"),
    (TextModel, "text.model"),
    (AccentPrimary, "accent.primary"),
    (AccentSecondary, "accent.secondary"),
    (AccentThought, "accent.thought"),
    (Success, "success"),
    (Error, "error"),
    (Warning, "warning"),
    (BackgroundBase, "bg.base"),
    (BackgroundPanel, "bg.panel"),
    (BackgroundDiffDelete, "bg.diff_delete"),
    (BackgroundDiffInsert, "bg.diff_insert"),
    (BackgroundSelection, "bg.selection"),
    (BorderPrompt, "border.prompt"),
    (BorderSelection, "border.selection"),
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
