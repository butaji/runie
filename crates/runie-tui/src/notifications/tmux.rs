//! tmux DCS passthrough helpers.

use crate::terminal::context::{MultiplexerKind, TerminalContext};

/// Grok only uses DCS passthrough when the tmux server supports it reliably.
pub fn passthrough_available(ctx: &TerminalContext) -> bool {
    ctx.multiplexer == MultiplexerKind::Tmux
        && ctx
            .tmux_version
            .as_deref()
            .is_some_and(|version| version_at_least(version, 3, 3))
}

fn version_at_least(version: &str, required_major: u32, required_minor: u32) -> bool {
    let mut parts = version
        .split('.')
        .map(|part| part.parse::<u32>().unwrap_or(0));
    (parts.next().unwrap_or(0), parts.next().unwrap_or(0)) >= (required_major, required_minor)
}

pub fn tmux_passthrough(sequence: &str) -> String {
    let escaped = sequence.replace('\x1b', "\x1b\x1b");
    format!("\x1bPtmux;{escaped}\x1b\\")
}

#[cfg(test)]
mod tests {
    use super::{passthrough_available, tmux_passthrough};
    use crate::terminal::context::{MultiplexerKind, TerminalContext};

    #[test]
    fn doubles_escapes_and_wraps_dcs() {
        assert_eq!(
            tmux_passthrough("\x1b]9;done\x07"),
            "\x1bPtmux;\x1b\x1b]9;done\x07\x1b\\"
        );
    }

    #[test]
    fn passthrough_requires_tmux_33() {
        let base = TerminalContext {
            brand: crate::terminal::context::TerminalName::Unknown,
            multiplexer: MultiplexerKind::Tmux,
            tmux_version: None,
            term_program_version: None,
        };
        assert!(!passthrough_available(&base));
        assert!(!passthrough_available(&TerminalContext {
            tmux_version: Some("3.2.9".into()),
            ..base.clone()
        }));
        assert!(passthrough_available(&TerminalContext {
            tmux_version: Some("3.3.0".into()),
            ..base
        }));
    }
}
