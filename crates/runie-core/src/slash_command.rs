/// Slash commands for TUI - shared between runie-cli and runie-tui

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum SlashCommand {
    New,
    Clear,
    Model(String),
    Tree,
    Fork,
    Copy,
    Onboard,
    Quit,
    Help,
    Cost,
    Status,
    Models,
    // Session
    Home,
    Resume,
    Sessions,
    Rename(String),
    Share,
    SessionInfo,
    // Context
    Context,
    Compact(Option<String>),
    CompactMode,
    Rewind,
    Usage,
    // UI
    Theme(Option<String>),
    Multiline,
    // Permission
    AlwaysApprove,
    Plan,
    Feedback(Option<String>),
    // Utility
    Btw(String),
    Logout,
    // Extensions (stub)
    Hooks,
    Plugins,
    Skills,
    Mcps,
    Extensions,
    // Shell (stub)
    Flush,
    Memory,
    Dream,
    Imagine(String),
    ImagineVideo(String),
    Unknown(String),
}

/// Build command lookup map for O(1) access
fn build_command_map() -> HashMap<&'static str, fn(&[&str]) -> SlashCommand> {
    let mut map = HashMap::new();
    add_basic_commands(&mut map);
    add_session_commands(&mut map);
    add_context_commands(&mut map);
    add_ui_commands(&mut map);
    add_permission_commands(&mut map);
    add_utility_commands(&mut map);
    add_extension_commands(&mut map);
    add_shell_commands(&mut map);
    map
}

fn add_basic_commands(map: &mut HashMap<&'static str, fn(&[&str]) -> SlashCommand>) {
    map.insert("/new", |_| SlashCommand::New);
    map.insert("/n", |_| SlashCommand::New);
    map.insert("/clear", |_| SlashCommand::Clear);
    map.insert("/c", |_| SlashCommand::Clear);
    map.insert("/model", parse_model_cmd);
    map.insert("/m", parse_model_cmd);
    map.insert("/tree", |_| SlashCommand::Tree);
    map.insert("/t", |_| SlashCommand::Tree);
    map.insert("/fork", |_| SlashCommand::Fork);
    map.insert("/f", |_| SlashCommand::Fork);
    map.insert("/copy", |_| SlashCommand::Copy);
    map.insert("/onboard", |_| SlashCommand::Onboard);
    map.insert("/o", |_| SlashCommand::Onboard);
    map.insert("/quit", |_| SlashCommand::Quit);
    map.insert("/q", |_| SlashCommand::Quit);
    map.insert("/exit", |_| SlashCommand::Quit);
    map.insert("/help", |_| SlashCommand::Help);
    map.insert("/h", |_| SlashCommand::Help);
    map.insert("/?", |_| SlashCommand::Help);
    map.insert("/cost", |_| SlashCommand::Cost);
    map.insert("/status", |_| SlashCommand::Status);
    map.insert("/models", |_| SlashCommand::Models);
}

fn add_session_commands(map: &mut HashMap<&'static str, fn(&[&str]) -> SlashCommand>) {
    map.insert("/home", |_| SlashCommand::Home);
    map.insert("/resume", |_| SlashCommand::Resume);
    map.insert("/sessions", |_| SlashCommand::Sessions);
    map.insert("/rename", parse_rename_cmd);
    map.insert("/share", |_| SlashCommand::Share);
    map.insert("/session-info", |_| SlashCommand::SessionInfo);
}

fn add_context_commands(map: &mut HashMap<&'static str, fn(&[&str]) -> SlashCommand>) {
    map.insert("/context", |_| SlashCommand::Context);
    map.insert("/compact", parse_compact_cmd);
    map.insert("/compact-mode", |_| SlashCommand::CompactMode);
    map.insert("/rewind", |_| SlashCommand::Rewind);
    map.insert("/usage", |_| SlashCommand::Usage);
}

fn add_ui_commands(map: &mut HashMap<&'static str, fn(&[&str]) -> SlashCommand>) {
    map.insert("/theme", parse_theme_cmd);
    map.insert("/multiline", |_| SlashCommand::Multiline);
}

fn add_permission_commands(map: &mut HashMap<&'static str, fn(&[&str]) -> SlashCommand>) {
    map.insert("/always-approve", |_| SlashCommand::AlwaysApprove);
    map.insert("/plan", |_| SlashCommand::Plan);
    map.insert("/feedback", parse_feedback_cmd);
}

fn add_utility_commands(map: &mut HashMap<&'static str, fn(&[&str]) -> SlashCommand>) {
    map.insert("/btw", parse_btw_cmd);
    map.insert("/logout", |_| SlashCommand::Logout);
}

fn add_extension_commands(map: &mut HashMap<&'static str, fn(&[&str]) -> SlashCommand>) {
    map.insert("/hooks", |_| SlashCommand::Hooks);
    map.insert("/plugins", |_| SlashCommand::Plugins);
    map.insert("/skills", |_| SlashCommand::Skills);
    map.insert("/mcps", |_| SlashCommand::Mcps);
    map.insert("/extensions", |_| SlashCommand::Extensions);
    map.insert("/ext", |_| SlashCommand::Extensions);
}

fn add_shell_commands(map: &mut HashMap<&'static str, fn(&[&str]) -> SlashCommand>) {
    map.insert("/flush", |_| SlashCommand::Flush);
    map.insert("/memory", |_| SlashCommand::Memory);
    map.insert("/dream", |_| SlashCommand::Dream);
    map.insert("/imagine", parse_imagine_cmd);
    map.insert("/imagine-video", parse_imagine_video_cmd);
}

fn parse_cmd(input: &str) -> Option<SlashCommand> {
    let parts: Vec<&str> = input.split_whitespace().collect();
    let cmd = parts[0];
    let args = &parts[1..];

    static COMMANDS: std::sync::LazyLock<HashMap<&'static str, fn(&[&str]) -> SlashCommand>> =
        std::sync::LazyLock::new(build_command_map);

    COMMANDS.get(cmd).map(|handler| handler(args))
        .or_else(|| Some(SlashCommand::Unknown(cmd.to_string())))
}

fn parse_rename_cmd(args: &[&str]) -> SlashCommand {
    if args.is_empty() {
        SlashCommand::Help
    } else {
        SlashCommand::Rename(args.join(" ").trim().to_string())
    }
}

fn parse_compact_cmd(args: &[&str]) -> SlashCommand {
    SlashCommand::Compact(args.first().map(|s| s.to_string()))
}

fn parse_theme_cmd(args: &[&str]) -> SlashCommand {
    SlashCommand::Theme(args.first().map(|s| s.to_string()))
}

fn parse_feedback_cmd(args: &[&str]) -> SlashCommand {
    SlashCommand::Feedback(if args.is_empty() { None } else { Some(args.join(" ").trim().to_string()) })
}

fn parse_btw_cmd(args: &[&str]) -> SlashCommand {
    SlashCommand::Btw(args.join(" ").trim().to_string())
}

fn parse_imagine_cmd(args: &[&str]) -> SlashCommand {
    SlashCommand::Imagine(args.join(" ").trim().to_string())
}

fn parse_imagine_video_cmd(args: &[&str]) -> SlashCommand {
    SlashCommand::ImagineVideo(args.join(" ").trim().to_string())
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

// ─── Help Formatting ─────────────────────────────────────────────────────────

fn session_help() -> &'static str {
    r#"  Session
    /new, /n           Start new session
    /clear, /c         Clear conversation
    /tree, /t          Open session tree navigator
    /fork, /f          Fork at current position
    /home              Return to welcome screen
    /resume            Resume previous session
    /sessions          Browse past sessions
    /rename <title>    Rename current session
    /share             Share session
    /session-info      Show session info
"#
}

fn context_help() -> &'static str {
    r#"  Context
    /context           View context usage
    /compact [ctx]     Compact conversation history
    /compact-mode      Toggle denser UI layout
    /rewind            Rewind conversation
    /usage             Show token/credit usage
"#
}

fn config_help() -> &'static str {
    r#"  Config
    /model, /m <name>  Switch model
    /onboard, /o       Configure provider and API key
    /theme [name]      Switch theme
    /status            Show current provider and model
    /models            Show available models
"#
}

fn tools_help() -> &'static str {
    r#"  Tools
    /copy              Copy last response to clipboard
    /cost              Show cost statistics
    /always-approve    Toggle auto-approve mode
    /multiline         Toggle multiline input
"#
}

fn permission_help() -> &'static str {
    r#"  Permission
    /plan              View current session plan
    /feedback [text]   Send feedback
"#
}

fn utility_help() -> &'static str {
    r#"  Utility
    /btw <question>    Ask side question
    /logout            Sign out
"#
}

fn extensions_help() -> &'static str {
    r#"  Extensions
    /extensions, /ext  Open extensions modal
    /hooks             Open extensions (Hooks)
    /plugins           Open extensions (Plugins)
    /skills            Open extensions (Skills)
    /mcps              Open extensions (MCP Servers)
"#
}

fn shell_help() -> &'static str {
    r#"  Shell
    /flush             Flush memory to disk
    /memory            Search memory
    /dream              Memory consolidation
    /imagine <prompt>  Generate image
    /imagine-video <prompt> Generate video
"#
}

fn app_help() -> &'static str {
    r#"  App
    /quit, /q, /exit   Exit runie
    /help, /h, /?      Show this help
"#
}

fn keyboard_help() -> &'static str {
    r#"Keyboard shortcuts:
  Enter              Submit message
  Shift+Enter        New line
  Ctrl+C             Exit
  Ctrl+O             Onboard / copy last response
  Ctrl+B             Toggle sidebar
  Ctrl+K / Ctrl+P    Command palette
  ?                  Show help"#
}

pub fn format_help() -> String {
    format!(
        "Available commands:\n{}{}{}{}{}{}{}{}{}{}",
        session_help(),
        context_help(),
        config_help(),
        tools_help(),
        permission_help(),
        utility_help(),
        extensions_help(),
        shell_help(),
        app_help(),
        keyboard_help()
    )
}
