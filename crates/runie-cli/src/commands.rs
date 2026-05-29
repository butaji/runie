/// CLI command handlers.

pub use runie_core::{SlashCommand, parse_slash_command, format_help};

pub enum Command {
    Chat(String),
    Compact,
    Tree,
    Branch(String),
    Save,
    Load(String),
    Quit,
}

pub struct CommandParser;

impl CommandParser {
    pub fn parse(input: &str) -> Result<Command, String> {
        let input = input.trim();

        if input.starts_with('/') {
            parse_slash_cmd(&input[1..])
        } else {
            Ok(Command::Chat(input.to_string()))
        }
    }
}

/// Parse slash command (after the leading '/').
fn parse_slash_cmd(input: &str) -> Result<Command, String> {
    let parts: Vec<&str> = input.split_whitespace().collect();
    let cmd = parts.first().map(|s| *s).unwrap_or("");

    match cmd {
        "compact" => Ok(Command::Compact),
        "tree" => Ok(Command::Tree),
        "branch" => parse_branch_cmd(&parts),
        "save" => Ok(Command::Save),
        "load" => parse_load_cmd(&parts),
        "quit" | "exit" => Ok(Command::Quit),
        _ => Err(format!("Unknown command: /{}", cmd)),
    }
}

fn parse_branch_cmd(parts: &[&str]) -> Result<Command, String> {
    let msg_id = parts.get(1).ok_or("Missing message ID")?;
    Ok(Command::Branch(msg_id.to_string()))
}

fn parse_load_cmd(parts: &[&str]) -> Result<Command, String> {
    let session_id = parts.get(1).ok_or("Missing session ID")?;
    Ok(Command::Load(session_id.to_string()))
}
