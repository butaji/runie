//! runie-print — Non-interactive CLI for single-turn LLM execution.

use anyhow::Result;
use runie_agent::{build_provider_with_warning, run_headless_turn, HeadlessOptions};
use runie_core::{
    config_reload,
    message::ChatMessage,
    provider::Provider,
};

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
    let config = config_reload::Config::load(Some(&config_reload::config_path()));
    let provider_name = config.provider.as_deref().unwrap_or("mock");
    let model = config.default_model().unwrap_or("echo");
    let provider =
        build_provider_with_warning(provider_name, model).map_err(|e| anyhow::anyhow!("{}", e))?;
    run_print_with(prompt, &provider).await?;
    println!();
    Ok(())
}

async fn run_print_with(prompt: &str, provider: &dyn Provider) -> Result<()> {
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

    let options = HeadlessOptions {
        execute_tools: true,
        max_tool_rounds: 5,
        on_chunk: Some(Box::new(|chunk: &str| {
            print!("{}", chunk);
            let _ = std::io::Write::flush(&mut std::io::stdout());
        })),
    };

    run_headless_turn(messages, provider, options).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn print_mode_streams_output() {
        let provider = runie_provider::MockProvider::default();
        let output = run_print_with("Hello", &provider).await;
        // run_print_with writes to stdout; we just verify it doesn't panic
        assert!(
            output.is_ok(),
            "print mode should not error on mock provider"
        );
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

        let config = config_reload::Config::load(Some(&path));
        assert_eq!(config.provider, Some("openai".to_string()));
        assert_eq!(config.default_model(), Some("gpt-4o"));
    }
}
