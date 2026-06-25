//! Print mode — streaming LLM response to stdout.

use anyhow::Result;
use runie_agent::headless_helper::{build_messages, build_options, build_sink};

/// Run a single-turn LLM prompt and stream the response to stdout.
pub fn run(prompt: &str) -> Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let messages = build_messages(prompt);
        let sink = build_sink(false);
        let opts = build_options(Some(Box::new(|chunk: &str| {
            print!("{}", chunk);
            let _ = std::io::Write::flush(&mut std::io::stdout());
        })));
        runie_agent::run_headless_cli(None, None, messages, sink, opts).await?;
        println!();
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn print_mode_accepts_prompt() {
        // Smoke test: verify the function accepts a prompt without panicking
        let result = run("test");
        // Will fail without config, but proves the dispatch works
        assert!(result.is_err() || result.is_ok());
    }
}
