use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use shell_words;
use std::panic::AssertUnwindSafe;

use super::{HarnessSkill, TurnStartCtx, TurnStartResult};

/// Configuration for the startup context skill.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StartupContextConfig {
    #[serde(default = "super::default_true")]
    pub enabled: bool,
    #[serde(default = "default_max_output")]
    pub max_output_bytes: usize,
    #[serde(default)]
    pub commands: Vec<String>,
}

impl Default for StartupContextConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_output_bytes: 2048,
            commands: vec!["pwd".into(), "ls".into(), "git branch --show-current".into()],
        }
    }
}
fn default_max_output() -> usize {
    2048
}

/// Startup context injector skill.
pub struct StartupContextSkill {
    config: StartupContextConfig,
    cache: RwLock<Option<String>>,
}

impl StartupContextSkill {
    pub fn new(config: StartupContextConfig) -> Self {
        Self { config, cache: RwLock::new(None) }
    }

    fn run_cmd(cmd: &str) -> String {
        let parts = match shell_words::split(cmd) {
            Ok(p) => p,
            Err(_) => return String::new(),
        };
        if parts.is_empty() {
            return String::new();
        }
        match std::process::Command::new(&parts[0])
            .args(&parts[1..])
            .output()
        {
            Ok(o) => String::from_utf8_lossy(&o.stdout).trim().to_owned(),
            Err(_) => String::new(),
        }
    }

    fn discover(&self) -> String {
        let lines: Vec<String> = std::iter::once("=== Workspace Context ===".into())
            .chain(self.config.commands.iter().filter_map(|c| {
                let o = Self::run_cmd(c);
                if o.is_empty() {
                    None
                } else {
                    Some(format!("$ {}\n{}", c, o))
                }
            }))
            .collect();
        let ctx = lines.join("\n");
        if ctx.len() > self.config.max_output_bytes {
            ctx[..self.config.max_output_bytes].to_string()
        } else {
            ctx
        }
    }

    pub fn get_context(&self) -> String {
        if let Some(c) = self.cache.read().as_ref() {
            return c.clone();
        }
        // Use block_in_place with catch_unwind for compatibility with single-threaded
        // runtimes used in tests. If block_in_place panics (not multi-threaded), fall
        // back to synchronous execution.
        let ctx = std::panic::catch_unwind(AssertUnwindSafe(|| {
            tokio::task::block_in_place(|| self.discover())
        }))
        .unwrap_or_else(|_| self.discover());
        // parking_lot RwLock guards don't panic, so we can use them directly.
        *self.cache.write() = Some(ctx.clone());
        ctx
    }

    pub fn clear_cache(&self) {
        *self.cache.write() = None;
    }
}

impl HarnessSkill for StartupContextSkill {
    fn name(&self) -> &str {
        "startup_context"
    }
    fn on_turn_start(&self, ctx: &TurnStartCtx) -> TurnStartResult {
        if !self.config.enabled {
            return TurnStartResult::Continue;
        }
        if ctx.system_prompt.contains("=== Workspace Context ===") {
            return TurnStartResult::Continue;
        }
        let ctx_str = self.get_context();
        if ctx_str.is_empty() {
            return TurnStartResult::Continue;
        }
        TurnStartResult::SkipWithMessage(format!("{}\n\n{}", ctx_str, ctx.message))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_cmd_simple() {
        let result = StartupContextSkill::run_cmd("echo hello");
        assert_eq!(result, "hello");
    }

    #[test]
    fn run_cmd_quoted_args() {
        // Test that shell_words handles quoted arguments with spaces
        let result = StartupContextSkill::run_cmd("echo 'hello world'");
        assert_eq!(result, "hello world");
    }

    #[test]
    fn run_cmd_double_quoted_args() {
        let result = StartupContextSkill::run_cmd(r#"echo "hello world""#);
        assert_eq!(result, "hello world");
    }

    #[test]
    fn run_cmd_empty() {
        let result = StartupContextSkill::run_cmd("");
        assert_eq!(result, "");
    }

    #[test]
    fn run_cmd_complex_args() {
        let result = StartupContextSkill::run_cmd("printf '%s %s' hello 'world test'");
        assert_eq!(result, "hello world test");
    }

    #[test]
    fn run_cmd_with_escaped_chars() {
        let result = StartupContextSkill::run_cmd(r"echo hello\nworld");
        assert!(result.contains("hello"));
    }
}
