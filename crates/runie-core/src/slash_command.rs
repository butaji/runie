/// Slash commands for TUI - shared between runie-cli and runie-tui

#[derive(Debug, Clone)]
pub enum SlashCommand {
    New,
    Clear,
    Model(String),
    Tree,
    Fork,
    Copy,
    Quit,
    Help,
    Cost,
    Unknown(String),
}

fn parse_cmd(input: &str) -> Option<SlashCommand> {
    let parts: Vec<&str> = input.split_whitespace().collect();
    let cmd = parts[0];
    let args = &parts[1..];

    static COMMANDS: &[(&[&str], fn(&[&str]) -> SlashCommand)] = &[
        (&["/new", "/n"], |_| SlashCommand::New),
        (&["/clear", "/c"], |_| SlashCommand::Clear),
        (&["/model", "/m"], parse_model_cmd),
        (&["/tree", "/t"], |_| SlashCommand::Tree),
        (&["/fork", "/f"], |_| SlashCommand::Fork),
        (&["/copy"], |_| SlashCommand::Copy),
        (&["/quit", "/q", "/exit"], |_| SlashCommand::Quit),
        (&["/help", "/h", "/?"], |_| SlashCommand::Help),
        (&["/cost"], |_| SlashCommand::Cost),
    ];

    for (aliases, handler) in COMMANDS {
        if aliases.iter().any(|&a| a == cmd) {
            return Some(handler(args));
        }
    }

    Some(SlashCommand::Unknown(cmd.to_string()))
}

fn parse_model_cmd(args: &[&str]) -> SlashCommand {
    if args.is_empty() {
        SlashCommand::Help
    } else {
        SlashCommand::Model(args[0].to_string())
    }
}

pub fn parse_slash_command(input: &str) -> Option<SlashCommand> {
    if input.starts_with('/') {
        parse_cmd(input)
    } else {
        None
    }
}

pub fn format_help() -> String {
    r#"Available commands:
  /new, /n           Start new session
  /clear, /c         Clear conversation
  /model, /m <name>  Switch model
  /tree, /t          Open session tree navigator
  /fork, /f          Fork at current position
  /copy              Copy last response to clipboard
  /cost              Show cost statistics
  /quit, /q, /exit   Exit runie
  /help, /h, /?       Show this help

Keyboard shortcuts:
  Enter              Submit message
  Shift+Enter        New line
  Ctrl+C             Exit
  Ctrl+O             Copy last response
  Ctrl+B             Toggle sidebar
  Ctrl+K / Ctrl+P    Command palette"#
        .to_string()
}
