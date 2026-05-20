use anvil::core::executor::run_headless;
use anvil::router::ModelDatabase;
use anvil::tui;

use clap::{Parser, Subcommand};
use anyhow::Result;

#[derive(Parser)]
#[command(
    name = "anvil",
    version = "0.1.0",
    about = "Terminal-native coding harness for AI agent swarms"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Run in offline mode (use bundled models.dev snapshot)
    #[arg(long)]
    offline: bool,

    /// Force a specific model (e.g. anthropic/claude-sonnet-4)
    #[arg(long)]
    model: Option<String>,

    /// Print result instead of running TUI (headless/scriptable mode)
    #[arg(long)]
    print: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Interactive TUI mode
    Run {
        /// Task to execute (headless if provided)
        task: Option<String>,

        /// Force specific model
        #[arg(long)]
        model: Option<String>,
    },
    /// Show model selector
    Models,
    /// Show cost breakdown
    Cost,
    /// Show agent swarm status
    Agents,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command.as_ref().unwrap_or(&Commands::Run { task: None, model: None }) {
        Commands::Run { task, model } => {
            let model = model.clone().or(cli.model);
            if cli.print.is_some() || task.is_some() {
                let intent = cli.print.as_ref()
                    .or(task.as_ref())
                    .map(|s| s.as_str())
                    .unwrap_or("");
                eprintln!("[anvil] headless mode: \"{}\"", intent);
                if let Some(m) = &model {
                    eprintln!("[anvil] model: {}", m);
                }
                if cli.offline {
                    eprintln!("[anvil] offline mode");
                }
                if let Err(e) = run_headless(intent) {
                    eprintln!("[anvil] headless error: {}", e);
                }
                Ok(())
            } else {
                if cli.offline {
                    eprintln!("[anvil] offline mode — using bundled models.dev snapshot");
                }
                tui::run()
            }
        }
        Commands::Models => {
            let db = ModelDatabase::new();
            println!("Available models:");
            for (id, model) in &db.models {
                let cost = if model.input_cost > 0.0 {
                    format!("${:.2}/Mtok", model.input_cost)
                } else {
                    "free".to_string()
                };
                let ctx = if model.context_length >= 1_000_000 {
                    format!("{}M ctx", model.context_length / 1_000_000)
                } else {
                    format!("{}K ctx", model.context_length / 1000)
                };
                println!("  {}  {}  {}", id, cost, ctx);
            }
            Ok(())
        }
        Commands::Cost => {
            let db = ModelDatabase::new();
            println!("Session total: ${:.2}", db.total_spent());
            for (id, status) in &db.statuses {
                if status.spent > 0.0 {
                    println!("  {}: ${:.2}", id, status.spent);
                }
            }
            Ok(())
        }
        Commands::Agents => {
            println!("4 agents active (mock)");
            println!("  • general   Suggest design improvements");
            println!("  • explore   Check Section component");
            Ok(())
        }
    }
}
