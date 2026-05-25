/// Slash commands for TUI - shared between runie-cli and runie-tui

#[derive(Debug, Clone)]
pub enum SlashCommand {
    New,
    Clear,
    Model(String),
    Compact,
    Save(Option<String>),
    Load(String),
    Tree,
    Fork,
    Quit,
    Help,
    Unknown(String),
}

pub fn parse_slash_command(input: &str) -> Option<SlashCommand> {
    if !input.starts_with('/') {
        return None;
    }
    
    let parts: Vec<&str> = input.split_whitespace().collect();
    let cmd = parts[0];
    let args = &parts[1..];
    
    match cmd {
        "/new" | "/n" => Some(SlashCommand::New),
        "/clear" | "/c" => Some(SlashCommand::Clear),
        "/model" | "/m" => {
            if args.is_empty() {
                Some(SlashCommand::Help)
            } else {
                Some(SlashCommand::Model(args[0].to_string()))
            }
        }
        "/compact" => Some(SlashCommand::Compact),
        "/save" => Some(SlashCommand::Save(args.first().map(|s| s.to_string()))),
        "/load" => {
            if args.is_empty() {
                None
            } else {
                Some(SlashCommand::Load(args[0].to_string()))
            }
        }
        "/tree" | "/t" => Some(SlashCommand::Tree),
        "/fork" | "/f" => Some(SlashCommand::Fork),
        "/quit" | "/q" | "/exit" => Some(SlashCommand::Quit),
        "/help" | "/h" | "/?" => Some(SlashCommand::Help),
        _ => Some(SlashCommand::Unknown(cmd.to_string())),
    }
}

pub fn format_help() -> String {
    r#"Available commands:
  /new, /n           Start new session
  /clear, /c         Clear conversation
  /model, /m <name>  Switch model
  /compact           Compact session (summarize old messages)
  /save [name]       Save session
  /load <name>       Load session
  /tree, /t          Open session tree navigator
  /fork, /f          Fork at current position
  /quit, /q, /exit   Exit runie
  /help, /h, /?       Show this help

Keyboard shortcuts:
  Enter              Submit message
  Shift+Enter        New line
  Ctrl+C             Exit
  Ctrl+B             Toggle sidebar
  Ctrl+K / Ctrl+P    Command palette"#
        .to_string()
}
