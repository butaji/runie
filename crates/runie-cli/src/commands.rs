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
            let parts: Vec<&str> = input[1..].split_whitespace().collect();
            match parts.get(0).map(|s| *s) {
                Some("compact") => Ok(Command::Compact),
                Some("tree") => Ok(Command::Tree),
                Some("branch") => {
                    let msg_id = parts.get(1).ok_or("Missing message ID")?;
                    Ok(Command::Branch(msg_id.to_string()))
                }
                Some("save") => Ok(Command::Save),
                Some("load") => {
                    let session_id = parts.get(1).ok_or("Missing session ID")?;
                    Ok(Command::Load(session_id.to_string()))
                }
                Some("quit") | Some("exit") => Ok(Command::Quit),
                _ => Err(format!("Unknown command: {}", input)),
            }
        } else {
            Ok(Command::Chat(input.to_string()))
        }
    }
}
