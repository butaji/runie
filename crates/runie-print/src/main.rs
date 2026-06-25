//! runie-print — Non-interactive CLI for single-turn LLM execution.

use anyhow::Result;
use runie_agent::{run_headless_cli, HeadlessCliOptions};
use runie_core::permissions::build_sink;
use runie_core::message::ChatMessage;

#[tokio::main]
async fn main() {
    let prompt = match std::env::args().nth(1) {
        Some(p) => p,
        None => {
            eprintln!("Usage: runie-print <prompt>");
            std::process::exit(1);
        }
    };

    if let Err(e) = run_print(&prompt).await {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

async fn run_print(prompt: &str) -> Result<()> {
    let system = runie_core::prompts::build_system_prompt(
        runie_core::prompts::DEFAULT_PROMPT,
        runie_core::prompts::DEFAULT_TOOLS,
        false,
        "",
    );
    let messages = vec![
        ChatMessage::system(system),
        ChatMessage::user(prompt.to_string()),
    ];
    let sink = build_sink(false);
    let opts = HeadlessCliOptions {
        execute_tools: true,
        max_tool_rounds: 5,
        on_chunk: Some(Box::new(|chunk: &str| {
            print!("{}", chunk);
            let _ = std::io::Write::flush(&mut std::io::stdout());
        })),
    };
    run_headless_cli(None, None, messages, sink, opts).await?;
    println!();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use runie_core::config;

    #[tokio::test]
    async fn print_mode_streams_output() {
        // run_print writes to stdout; we just verify it doesn't panic.
        let output = run_print("Hello").await;
        assert!(output.is_ok(), "print mode should not error");
    }

    #[tokio::test]
    async fn print_mode_respects_config_provider() {
        // Layer 1: verify that Config::load_from parses provider correctly
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(
            &path,
            r#"
provider = "openai"
model = "gpt-4o"
"#,
        )
        .unwrap();

        let config = config::Config::load(Some(&path));
        assert_eq!(config.provider, Some("openai".to_string()));
        assert_eq!(config.default_model(), Some("gpt-4o"));
    }
}
