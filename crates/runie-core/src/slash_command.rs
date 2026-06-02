/// Slash commands for TUI - shared between runie-cli and runie-tui

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
        (&["/onboard", "/o"], |_| SlashCommand::Onboard),
        (&["/quit", "/q", "/exit"], |_| SlashCommand::Quit),
        (&["/help", "/h", "/?"], |_| SlashCommand::Help),
        (&["/cost"], |_| SlashCommand::Cost),
        (&["/status"], |_| SlashCommand::Status),
        (&["/models"], |_| SlashCommand::Models),
        // Session
        (&["/home"], |_| SlashCommand::Home),
        (&["/resume"], |_| SlashCommand::Resume),
        (&["/sessions"], |_| SlashCommand::Sessions),
        (&["/rename"], parse_rename_cmd),
        (&["/share"], |_| SlashCommand::Share),
        (&["/session-info"], |_| SlashCommand::SessionInfo),
        // Context
        (&["/context"], |_| SlashCommand::Context),
        (&["/compact"], parse_compact_cmd),
        (&["/compact-mode"], |_| SlashCommand::CompactMode),
        (&["/rewind"], |_| SlashCommand::Rewind),
        (&["/usage"], |_| SlashCommand::Usage),
        // UI
        (&["/theme"], parse_theme_cmd),
        (&["/multiline"], |_| SlashCommand::Multiline),
        // Permission
        (&["/always-approve"], |_| SlashCommand::AlwaysApprove),
        (&["/plan"], |_| SlashCommand::Plan),
        (&["/feedback"], parse_feedback_cmd),
        // Utility
        (&["/btw"], parse_btw_cmd),
        (&["/logout"], |_| SlashCommand::Logout),
        // Extensions
        (&["/hooks"], |_| SlashCommand::Hooks),
        (&["/plugins"], |_| SlashCommand::Plugins),
        (&["/skills"], |_| SlashCommand::Skills),
        (&["/mcps"], |_| SlashCommand::Mcps),
        (&["/extensions", "/ext"], |_| SlashCommand::Extensions),
        // Shell
        (&["/flush"], |_| SlashCommand::Flush),
        (&["/memory"], |_| SlashCommand::Memory),
        (&["/dream"], |_| SlashCommand::Dream),
        (&["/imagine"], parse_imagine_cmd),
        (&["/imagine-video"], parse_imagine_video_cmd),
    ];

    for (aliases, handler) in COMMANDS {
        if aliases.iter().any(|&a| a == cmd) {
            return Some(handler(args));
        }
    }

    Some(SlashCommand::Unknown(cmd.to_string()))
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

pub fn format_help() -> String {
    r#"Available commands:
  Session
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
  Context
    /context           View context usage
    /compact [ctx]     Compact conversation history
    /compact-mode      Toggle denser UI layout
    /rewind            Rewind conversation
    /usage             Show token/credit usage
  Config
    /model, /m <name>  Switch model
    /onboard, /o       Configure provider and API key
    /theme [name]      Switch theme
    /status            Show current provider and model
    /models            Show available models
  Tools
    /copy              Copy last response to clipboard
    /cost              Show cost statistics
    /always-approve    Toggle auto-approve mode
    /multiline         Toggle multiline input
  Permission
    /plan              View current session plan
    /feedback [text]   Send feedback
  Utility
    /btw <question>    Ask side question
    /logout            Sign out
  Extensions
    /extensions, /ext  Open extensions modal
    /hooks             Open extensions (Hooks)
    /plugins           Open extensions (Plugins)
    /skills            Open extensions (Skills)
    /mcps              Open extensions (MCP Servers)
  Shell
    /flush             Flush memory to disk
    /memory            Search memory
    /dream              Memory consolidation
    /imagine <prompt>  Generate image
    /imagine-video <prompt> Generate video
  App
    /quit, /q, /exit   Exit runie
    /help, /h, /?      Show this help

Keyboard shortcuts:
  Enter              Submit message
  Shift+Enter        New line
  Ctrl+C             Exit
  Ctrl+O             Onboard / copy last response
  Ctrl+B             Toggle sidebar
  Ctrl+K / Ctrl+P    Command palette
  ?                  Show help"#
        .to_string()
}
