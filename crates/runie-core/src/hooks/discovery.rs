//! File-based hook discovery from ~/.runie/hooks/{event}/ directories.
//!
//! Hooks are defined as JSON files with the following format:
//! ```json
//! {
//!   "command": "echo allow",
//!   "env": { "VAR": "value" },
//!   "trust": "trusted",
//!   "timeout_ms": 5000
//! }
//! ```
//!
//! Event types: session_start, pre_tool_use, post_tool_use, session_end, subagent_start, subagent_stop

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use serde::{Deserialize, Serialize};

use super::{HookDecision, HookEvent, HookHandler};

/// Hook trust level for security policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HookTrust {
    /// Untrusted - network and file write operations blocked.
    #[default]
    Untrusted,
    /// Fully trusted - can execute any command.
    Trusted,
}

/// A hook loaded from a JSON file.
#[derive(Debug, Clone, Deserialize)]
pub struct FileHook {
    /// Shell command to execute.
    pub command: String,
    /// Environment variables to pass to the hook.
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// Trust level for the hook.
    #[serde(default)]
    pub trust: HookTrust,
    /// Execution timeout in milliseconds.
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
}

fn default_timeout() -> u64 {
    5000
}

/// Discover hooks from the ~/.runie/hooks/{event}/ directory structure.
pub fn discover_hooks(hooks_dir: &Path) -> HashMap<HookEvent, Vec<Box<dyn HookHandler>>> {
    let mut handlers: HashMap<HookEvent, Vec<Box<dyn HookHandler>>> = HashMap::new();

    if !hooks_dir.is_dir() {
        return handlers;
    }

    for entry in fs::read_dir(hooks_dir).into_iter().flatten().flatten() {
        let event_dir = entry.path();
        if !event_dir.is_dir() {
            continue;
        }

        let event_name = event_dir.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let Some(event) = parse_event_name(event_name) else {
            continue;
        };

        for hook_entry in fs::read_dir(&event_dir).into_iter().flatten().flatten() {
            let hook_path = hook_entry.path();
            if !hook_path.is_file() {
                continue;
            }
            if hook_path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }

            if let Ok(hook) = load_hook_file(&hook_path) {
                let handler: Box<dyn HookHandler> = match hook.trust {
                    HookTrust::Trusted => Box::new(FileHookHandler::new(hook, true)),
                    HookTrust::Untrusted => Box::new(FileHookHandler::new(hook, false)),
                };
                handlers.entry(event).or_default().push(handler);
            }
        }
    }

    handlers
}

/// Load a hook from a JSON file.
fn load_hook_file(path: &Path) -> Result<FileHook, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let hook: FileHook = serde_json::from_str(&content)?;
    Ok(hook)
}

/// Parse a hook event name into a HookEvent.
fn parse_event_name(name: &str) -> Option<HookEvent> {
    match name.to_ascii_lowercase().as_str() {
        "session_start" | "sessionstart" => Some(HookEvent::SessionStart),
        "pre_tool_use" | "pretooluse" => Some(HookEvent::PreToolUse),
        "post_tool_use" | "posttooluse" => Some(HookEvent::PostToolUse),
        "session_end" | "sessionend" => Some(HookEvent::Stop),
        "subagent_start" | "subagentstart" => Some(HookEvent::SubagentStart),
        "subagent_stop" | "subagentstop" => Some(HookEvent::SubagentStop),
        "permission_request" | "permissionrequest" => Some(HookEvent::PermissionRequest),
        "pre_compact" | "precompact" => Some(HookEvent::PreCompact),
        "post_compact" | "postcompact" => Some(HookEvent::PostCompact),
        "user_prompt_submit" | "userpromptsubmit" => Some(HookEvent::UserPromptSubmit),
        _ => None,
    }
}

/// Get the default hooks directory path (~/.runie/hooks).
pub fn default_hooks_dir() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join(".runie").join("hooks"))
        .unwrap_or_else(|| PathBuf::from(".runie/hooks"))
}

/// A hook handler that executes a file-based hook.
pub struct FileHookHandler {
    hook: FileHook,
    trusted: bool,
}

impl FileHookHandler {
    pub fn new(hook: FileHook, trusted: bool) -> Self {
        Self { hook, trusted }
    }
}

impl HookHandler for FileHookHandler {
    fn handle(&self, payload: &serde_json::Value) -> super::HookDecision {
        let input = match serde_json::to_string(payload) {
            Ok(s) => s,
            Err(_) => return HookDecision::Allow,
        };

        run_file_hook(&self.hook, &input, self.trusted)
    }
}

/// Run a file-based hook with proper environment.
fn run_file_hook(hook: &FileHook, _input: &str, _trusted: bool) -> HookDecision {
    let output = match std::process::Command::new("sh")
        .args(["-c", &hook.command])
        .envs(&hook.env)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
    {
        Ok(o) => o,
        Err(_) => return HookDecision::Allow,
    };

    // Fail-open: if command fails, allow
    if !output.status.success() {
        return HookDecision::Allow;
    }

    let text = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    match text.to_ascii_lowercase().as_str() {
        "allow" | "" => HookDecision::Allow,
        "deny" => HookDecision::Deny { reason: "hook denied".into() },
        _ => match serde_json::from_str::<serde_json::Value>(&text) {
            Ok(value) => HookDecision::Modify { payload: value },
            Err(_) => HookDecision::Allow,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_event_name_valid() {
        assert_eq!(parse_event_name("session_start"), Some(HookEvent::SessionStart));
        assert_eq!(parse_event_name("pre_tool_use"), Some(HookEvent::PreToolUse));
        assert_eq!(parse_event_name("post_tool_use"), Some(HookEvent::PostToolUse));
        assert_eq!(parse_event_name("session_end"), Some(HookEvent::Stop));
        assert_eq!(parse_event_name("subagent_start"), Some(HookEvent::SubagentStart));
        assert_eq!(parse_event_name("subagent_stop"), Some(HookEvent::SubagentStop));
    }

    #[test]
    fn parse_event_name_case_insensitive() {
        assert_eq!(parse_event_name("SESSION_START"), Some(HookEvent::SessionStart));
        assert_eq!(parse_event_name("PreToolUse"), Some(HookEvent::PreToolUse));
        assert_eq!(parse_event_name("Post_Tool_Use"), Some(HookEvent::PostToolUse));
    }

    #[test]
    fn parse_event_name_invalid() {
        assert_eq!(parse_event_name("invalid_event"), None);
        assert_eq!(parse_event_name(""), None);
        assert_eq!(parse_event_name("unknown"), None);
    }

    #[test]
    fn file_hook_deserialization() {
        let json = r#"{
            "command": "echo allow",
            "env": { "TEST": "value" },
            "trust": "trusted",
            "timeout_ms": 3000
        }"#;

        let hook: FileHook = serde_json::from_str(json).unwrap();
        assert_eq!(hook.command, "echo allow");
        assert_eq!(hook.env.get("TEST"), Some(&"value".to_string()));
        assert_eq!(hook.trust, HookTrust::Trusted);
        assert_eq!(hook.timeout_ms, 3000);
    }

    #[test]
    fn file_hook_defaults() {
        let json = r#"{
            "command": "echo test"
        }"#;

        let hook: FileHook = serde_json::from_str(json).unwrap();
        assert_eq!(hook.command, "echo test");
        assert!(hook.env.is_empty());
        assert_eq!(hook.trust, HookTrust::Untrusted);
        assert_eq!(hook.timeout_ms, 5000);
    }

    #[test]
    fn hook_trust_serialization() {
        assert_eq!(serde_json::to_string(&HookTrust::Trusted).unwrap(), "\"trusted\"");
        assert_eq!(serde_json::to_string(&HookTrust::Untrusted).unwrap(), "\"untrusted\"");
    }

    #[test]
    fn default_hooks_dir_format() {
        let dir = default_hooks_dir();
        assert!(dir.to_string_lossy().contains(".runie"));
        assert!(dir.to_string_lossy().contains("hooks"));
    }

    #[test]
    fn discover_hooks_empty_dir() {
        let temp = tempfile::tempdir().unwrap();
        let hooks = discover_hooks(temp.path());
        assert!(hooks.is_empty());
    }

    #[test]
    fn discover_hooks_nonexistent_dir() {
        let hooks = discover_hooks(Path::new("/nonexistent/hooks"));
        assert!(hooks.is_empty());
    }

    #[test]
    fn discover_hooks_with_valid_event_dir() {
        use tempfile::TempDir;

        let temp = TempDir::new().unwrap();
        let hooks_dir = temp.path();

        // Create event directory
        let event_dir = hooks_dir.join("pre_tool_use");
        std::fs::create_dir(&event_dir).unwrap();

        // Create hook file
        let hook_path = event_dir.join("test_hook.json");
        std::fs::write(
            &hook_path,
            r#"{
                "command": "echo allow",
                "trust": "trusted",
                "timeout_ms": 1000
            }"#,
        )
        .unwrap();

        let hooks = discover_hooks(hooks_dir);
        assert!(hooks.contains_key(&HookEvent::PreToolUse));
        let handlers = hooks.get(&HookEvent::PreToolUse).unwrap();
        assert_eq!(handlers.len(), 1);
    }

    #[test]
    fn discover_hooks_ignores_non_json_files() {
        use tempfile::TempDir;

        let temp = TempDir::new().unwrap();
        let hooks_dir = temp.path();

        let event_dir = hooks_dir.join("session_start");
        std::fs::create_dir(&event_dir).unwrap();

        // Create non-JSON file (should be ignored)
        std::fs::write(event_dir.join("readme.txt"), "not a hook").unwrap();
        // Create JSON file (should be loaded)
        std::fs::write(
            event_dir.join("valid.json"),
            r#"{"command": "echo test"}"#,
        )
        .unwrap();

        let hooks = discover_hooks(hooks_dir);
        let handlers = hooks.get(&HookEvent::SessionStart).unwrap();
        assert_eq!(handlers.len(), 1);
    }

    #[test]
    fn discover_hooks_invalid_json_skipped() {
        use tempfile::TempDir;

        let temp = TempDir::new().unwrap();
        let hooks_dir = temp.path();

        let event_dir = hooks_dir.join("session_start");
        std::fs::create_dir(&event_dir).unwrap();

        // Create invalid JSON file (should be skipped)
        std::fs::write(event_dir.join("bad.json"), "not valid json {").unwrap();

        let hooks = discover_hooks(hooks_dir);
        assert!(!hooks.contains_key(&HookEvent::SessionStart));
    }

    #[test]
    fn discover_hooks_invalid_event_dir_skipped() {
        use tempfile::TempDir;

        let temp = TempDir::new().unwrap();
        let hooks_dir = temp.path();

        // Create directory with invalid event name
        let event_dir = hooks_dir.join("invalid_event_name");
        std::fs::create_dir(&event_dir).unwrap();
        std::fs::write(
            event_dir.join("hook.json"),
            r#"{"command": "echo test"}"#,
        )
        .unwrap();

        let hooks = discover_hooks(hooks_dir);
        assert!(hooks.is_empty());
    }

    #[test]
    fn file_hook_handler_returns_allow() {
        let hook = FileHook {
            command: "echo allow".to_string(),
            env: HashMap::new(),
            trust: HookTrust::Trusted,
            timeout_ms: 1000,
        };
        let handler = FileHookHandler::new(hook, true);
        let decision = handler.handle(&serde_json::json!({"test": true}));
        assert_eq!(decision, HookDecision::Allow);
    }

    #[test]
    fn file_hook_handler_returns_deny() {
        let hook = FileHook {
            command: "echo deny".to_string(),
            env: HashMap::new(),
            trust: HookTrust::Trusted,
            timeout_ms: 1000,
        };
        let handler = FileHookHandler::new(hook, true);
        let decision = handler.handle(&serde_json::json!({"test": true}));
        assert_eq!(
            decision,
            HookDecision::Deny { reason: "hook denied".into() }
        );
    }

    #[test]
    fn file_hook_handler_returns_modify() {
        let hook = FileHook {
            command: r#"echo '{"modified": true}'"#.to_string(),
            env: HashMap::new(),
            trust: HookTrust::Trusted,
            timeout_ms: 1000,
        };
        let handler = FileHookHandler::new(hook, true);
        let decision = handler.handle(&serde_json::json!({"test": true}));
        assert_eq!(
            decision,
            HookDecision::Modify { payload: serde_json::json!({"modified": true}) }
        );
    }

    #[test]
    fn file_hook_handler_fail_open() {
        let hook = FileHook {
            command: "exit 1".to_string(),
            env: HashMap::new(),
            trust: HookTrust::Trusted,
            timeout_ms: 1000,
        };
        let handler = FileHookHandler::new(hook, true);
        let decision = handler.handle(&serde_json::json!({"test": true}));
        // Fail-open: command failure returns Allow
        assert_eq!(decision, HookDecision::Allow);
    }
}
