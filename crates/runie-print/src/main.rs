//! runie-print — Non-interactive CLI for single-turn LLM execution.

use anyhow::Result;
use runie_agent::parser::parse_tool_calls;
use runie_agent::{build_provider, Tool};
use runie_core::{
    config_reload,
    provider::{Message, Provider, ResponseChunk},
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
    let config = config_reload::Config::load_from(&config_reload::config_path());
    let provider_name = config.provider.as_deref().unwrap_or("mock");
    let model = config.default_model().unwrap_or("echo");
    let provider = build_provider(provider_name, model);
    let mut stdout = String::new();
    run_print_with(prompt, provider, |chunk| {
        stdout.push_str(&chunk);
        print!("{}", chunk);
        let _ = std::io::Write::flush(&mut std::io::stdout());
    })
    .await?;
    println!();
    Ok(())
}

async fn run_print_with<P, F>(prompt: &str, provider: P, mut on_chunk: F) -> Result<()>
where
    P: Provider,
    F: FnMut(String) + Send,
{
    let system = runie_core::prompts::build_system_prompt(
        runie_core::prompts::DEFAULT_PROMPT,
        "read_file, list_dir, write_file, edit_file, bash, grep, find",
        false,
        "",
    );

    let mut messages = vec![
        Message::System { content: system },
        Message::User {
            content: prompt.to_string(),
        },
    ];

    for _ in 0..5 {
        let mut response_text = String::new();
        provider
            .generate(messages.clone(), |chunk: ResponseChunk| {
                response_text.push_str(&chunk.content);
                on_chunk(chunk.content);
            })
            .await?;

        let tools = parse_tool_calls(&response_text);
        if tools.is_empty() {
            break;
        }

        on_chunk("\n".to_string());
        messages.push(Message::Assistant {
            content: response_text,
        });

        for tool in &tools {
            let result = execute_tool(tool);
            messages.push(Message::ToolResult {
                content: format!("{} result:\n{}", tool.name(), result.output),
            });
        }
    }

    Ok(())
}

fn execute_tool(tool: &Tool) -> runie_agent::ToolResult {
    match tool {
        Tool::EditFile { .. } => {
            // In print mode, execute edit directly without preview
            tool.execute()
        }
        _ => tool.execute(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn print_mode_streams_output() {
        let provider = runie_provider::MockProvider::default();
        let mut output = String::new();
        run_print_with("Hello", provider, |chunk| {
            output.push_str(&chunk);
        })
        .await
        .unwrap();
        assert!(!output.is_empty(), "Output should contain streamed chunks");
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

        let config = config_reload::Config::load_from(&path);
        assert_eq!(config.provider, Some("openai".to_string()));
        assert_eq!(config.default_model(), Some("gpt-4o"));
    }
}
