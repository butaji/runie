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

// === Headless mode command handlers ===

use std::path::PathBuf;
use runie_ai::Provider;
use crate::provider_factory::create_provider;
use crate::settings::Settings;

/// Headless execution context
pub struct HeadlessContext {
    pub session_id: Option<String>,
    pub cwd: PathBuf,
    pub always_approve: bool,
}

impl Default for HeadlessContext {
    fn default() -> Self {
        Self {
            session_id: None,
            cwd: PathBuf::from("."),
            always_approve: false,
        }
    }
}

/// Execute a single prompt in headless mode
pub async fn headless_execute_prompt(
    prompt: &str,
    ctx: &HeadlessContext,
    settings: &Settings,
) -> Result<String, String> {
    if prompt.trim().is_empty() {
        return Err("Empty prompt".to_string());
    }

    let provider = create_provider(false, settings)
        .map_err(|e| format!("Provider error: {}", e))?;

    let messages = vec![runie_core::Message::User {
        content: prompt.to_string(),
        attachments: vec![],
    }];

    provider
        .chat_simple(messages)
        .await
        .map_err(|e| format!("Chat error: {}", e))
}

/// Create a new headless session
pub fn headless_create_session(
    ctx: &mut HeadlessContext,
    session_id: Option<String>,
    cwd: Option<PathBuf>,
) {
    ctx.session_id = session_id.or_else(|| Some(generate_session_id()));
    if let Some(path) = cwd {
        ctx.cwd = path;
    }
}

/// Resume an existing headless session
pub fn headless_resume_session(
    _ctx: &mut HeadlessContext,
    _session_id: &str,
) -> Result<(), String> {
    // TODO: Load session state from disk
    Ok(())
}

/// Continue the most recent headless session
pub fn headless_continue_session(_ctx: &mut HeadlessContext) -> Result<(), String> {
    // TODO: Find and resume most recent session
    Ok(())
}

fn generate_session_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("session-{:x}", timestamp)
}
