//! Terminal brand and multiplexer context used by notification/link routing.

use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalName {
    Iterm2,
    WezTerm,
    Warp,
    Kitty,
    Ghostty,
    Vte,
    Foot,
    AppleTerminal,
    Alacritty,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MultiplexerKind {
    None,
    Tmux,
    Screen,
    Zellij,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalContext {
    pub brand: TerminalName,
    pub multiplexer: MultiplexerKind,
    pub tmux_version: Option<String>,
    pub term_program_version: Option<String>,
}

impl TerminalContext {
    pub fn detect() -> Self {
        let program = std::env::var("TERM_PROGRAM")
            .unwrap_or_default()
            .to_ascii_lowercase();
        let brand = match program.as_str() {
            "iterm.app" => TerminalName::Iterm2,
            "wezterm" => TerminalName::WezTerm,
            "warpterminal" => TerminalName::Warp,
            "kitty" if std::env::var_os("KITTY_WINDOW_ID").is_some() => TerminalName::Kitty,
            "ghostty" => TerminalName::Ghostty,
            "apple_terminal" => TerminalName::AppleTerminal,
            "alacritty" => TerminalName::Alacritty,
            _ if std::env::var_os("VTE_VERSION").is_some() => TerminalName::Vte,
            _ if std::env::var_os("WEZTERM_EXECUTABLE").is_some() => TerminalName::WezTerm,
            _ => TerminalName::Unknown,
        };
        let (multiplexer, tmux_version) = if let Ok(version) = std::env::var("TMUX") {
            let _ = version;
            (MultiplexerKind::Tmux, std::env::var("TMUX_VERSION").ok())
        } else if std::env::var_os("ZELLIJ").is_some() {
            (MultiplexerKind::Zellij, None)
        } else if std::env::var_os("STY").is_some() {
            (MultiplexerKind::Screen, None)
        } else {
            (MultiplexerKind::None, None)
        };
        Self { brand, multiplexer, tmux_version, term_program_version: std::env::var("TERM_PROGRAM_VERSION").ok() }
    }

    pub fn is_tmux_backed(&self) -> bool {
        self.multiplexer == MultiplexerKind::Tmux
    }
}

static CONTEXT: OnceLock<TerminalContext> = OnceLock::new();

pub fn terminal_context() -> &'static TerminalContext {
    CONTEXT.get_or_init(TerminalContext::detect)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_environment_is_safe() {
        let context = TerminalContext {
            brand: TerminalName::Unknown,
            multiplexer: MultiplexerKind::None,
            tmux_version: None,
            term_program_version: None,
        };
        assert!(!context.is_tmux_backed());
    }
}
