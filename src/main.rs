use anvil::core::executor::run_headless;
use anvil::router::ModelDatabase;
use anvil::tui;

use clap::{Parser, Subcommand};
use anyhow::Result;

/// Returns the anvil agent packs directory (~/.anvil/agents)
fn agent_packs_dir() -> std::path::PathBuf {
    dirs::home_dir()
        .map(|h| h.join(".anvil/agents"))
        .unwrap_or_else(|| std::path::PathBuf::from(".anvil/agents"))
}

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
    /// Manage agent packs
    Agent {
        #[command(subcommand)]
        action: AgentAction,
    },
}

#[derive(Subcommand)]
enum AgentAction {
    /// Install an agent pack from a git URL
    Install {
        /// Git URL of the agent pack (e.g. https://github.com/alice/backend-agent)
        url: String,
        /// Optional name override for the agent pack
        #[arg(short, long)]
        name: Option<String>,
    },
    /// List all installed agent packs
    List,
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
        Commands::Agent { action } => match action {
            AgentAction::Install { url, name } => {
                let packs_dir = agent_packs_dir();
                std::fs::create_dir_all(&packs_dir)
                    .map_err(|e| anyhow::anyhow!("Failed to create agents dir: {}", e))?;

                // Determine agent name: from --name arg, or derive from URL
                let agent_name = if let Some(n) = &name {
                    n.clone()
                } else {
                    url.trim_end_matches('/').rsplit('/').next().unwrap_or("agent").to_string()
                };

                let target_dir = packs_dir.join(&agent_name);
                if target_dir.exists() {
                    println!("[anvil] Agent '{}' already installed at {:?}", agent_name, target_dir);
                    println!("[anvil] Use --name to install with a different name, or remove it first.");
                    return Ok(());
                }

                println!("[anvil] Installing agent pack '{}' from {}", agent_name, url);
                println!("[anvil] Destination: {:?}", target_dir);

                // Clone into a temp dir, then move to target
                let temp_dir = std::env::temp_dir().join(format!("anvil-install-{}", agent_name));
                let _ = std::fs::remove_dir_all(&temp_dir);

                let output = std::process::Command::new("git")
                    .args(["clone", url.as_str(), temp_dir.to_str().unwrap()])
                    .output()?;

                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    return Err(anyhow::anyhow!("Git clone failed: {}", stderr));
                }

                // Validate anvil.js exists
                let anvil_js = temp_dir.join("anvil.js");
                if !anvil_js.exists() {
                    return Err(anyhow::anyhow!(
                        "Cloned repo does not contain anvil.js at the root. Is this an anvil agent pack?"
                    ));
                }

                // Move to final location
                std::fs::rename(&temp_dir, &target_dir)
                    .map_err(|e| anyhow::anyhow!("Failed to move clone to {:?}: {}", target_dir, e))?;

                // Inspect the agent pack
                let content = std::fs::read_to_string(&anvil_js).unwrap_or_default();
                let has_route = content.contains("export function route")
                    || content.contains("export async function route");
                let has_plan = content.contains("export async function plan")
                    || content.contains("export function plan");
                let has_validate = content.contains("export async function validate")
                    || content.contains("export function validate");

                println!("[anvil] ✓ Agent '{}' installed successfully", agent_name);
                println!();
                println!("  route():     {}", if has_route { "✓" } else { "✗" });
                println!("  plan():      {}", if has_plan { "✓" } else { "✗" });
                println!("  validate():  {}", if has_validate { "✓" } else { "✗" });

                // Check for prompts/ and tests/ dirs
                let prompts_dir = target_dir.join("prompts");
                let tests_dir = target_dir.join("tests");
                if prompts_dir.exists() {
                    println!("  prompts/:    ✓");
                }
                if tests_dir.exists() {
                    println!("  tests/:      ✓");
                }

                Ok(())
            }
            AgentAction::List => {
                let packs_dir = agent_packs_dir();
                if !packs_dir.exists() {
                    println!("No agent packs installed.");
                    println!("Run 'anvil agent install <git-url>' to add one.");
                    return Ok(());
                }

                let entries: Vec<_> = std::fs::read_dir(&packs_dir)
                    .map_err(|e| anyhow::anyhow!("Failed to read agents dir: {}", e))?
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().join("anvil.js").exists())
                    .collect();

                if entries.is_empty() {
                    println!("No agent packs installed.");
                    println!("Run 'anvil agent install <git-url>' to add one.");
                    return Ok(());
                }

                println!("Installed agent packs ({}):\n", entries.len());
                for entry in entries {
                    let path = entry.path();
                    let name = path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("?");
                    let anvil_js = path.join("anvil.js");
                    let (has_route, has_plan, has_validate) = if let Ok(content) = std::fs::read_to_string(&anvil_js) {
                        (
                            content.contains("export function route") || content.contains("export async function route"),
                            content.contains("export async function plan") || content.contains("export function plan"),
                            content.contains("export async function validate") || content.contains("export function validate"),
                        )
                    } else {
                        (false, false, false)
                    };

                    let funcs: Vec<&str> = [
                        (has_route, "route"),
                        (has_plan, "plan"),
                        (has_validate, "validate"),
                    ].iter().filter(|(on, _)| *on).map(|(_, n)| *n).collect();

                    let funcs_str = if funcs.is_empty() {
                        "(no functions)".to_string()
                    } else {
                        funcs.join(", ")
                    };

                    println!("  {}  [{:?}]  {}", name, path, funcs_str);
                }
                Ok(())
            }
        },
    }
}
