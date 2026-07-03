//! Shell completion generator for the runie CLI.
//!
//! This module is only compiled when the `completions` feature is enabled:
//!     cargo build -p runie-cli --features completions
//!
//! Usage:
//!     runie completion bash > /etc/bash_completion.d/runie
//!     runie completion zsh > ~/.zsh/completions/_runie
//!     runie completion fish > ~/.config/fish/completions/runie.fish

use anyhow::Result;
use clap::CommandFactory;
use clap_complete::shells::{Bash, Elvish, Fish, PowerShell, Zsh};

use crate::Cli;

/// Dispatch to the appropriate generator based on shell name.
pub fn run_completion(shell: &str) -> Result<()> {
    let mut cmd = Cli::command();

    match shell.to_lowercase().as_str() {
        "bash" => {
            clap_complete::generate(Bash, &mut cmd, "runie", &mut std::io::stdout());
        }
        "zsh" => {
            clap_complete::generate(Zsh, &mut cmd, "runie", &mut std::io::stdout());
        }
        "fish" => {
            clap_complete::generate(Fish, &mut cmd, "runie", &mut std::io::stdout());
        }
        "powershell" | "pwsh" => {
            clap_complete::generate(PowerShell, &mut cmd, "runie", &mut std::io::stdout());
        }
        "elvish" => {
            clap_complete::generate(Elvish, &mut cmd, "runie", &mut std::io::stdout());
        }
        _ => {
            anyhow::bail!(
                "Unknown shell: {}\nSupported shells: bash, zsh, fish, powershell, elvish",
                shell
            );
        }
    }

    Ok(())
}
