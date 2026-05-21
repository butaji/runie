use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::Command;

#[derive(Parser)]
#[command(name = "cargo-pantry")]
#[command(about = "Widget development environment for Anvil TUI")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    
    /// Path to pantry directory
    #[arg(short, long, default_value = "pantry")]
    dir: PathBuf,
}

#[derive(Subcommand)]
enum Commands {
    /// Open interactive pantry browser
    Open,
    /// Dump all widget renders to stdout (for CI)
    Dump {
        /// Output format
        #[arg(short, long, default_value = "ansi")]
        format: String,
    },
    /// List all available widgets
    List,
    /// Initialize a new pantry directory
    Init {
        /// Target directory
        #[arg(default_value = "pantry")]
        target: PathBuf,
    },
    /// Run widget tests in headless mode
    Test,
}

fn main() {
    let cli = Cli::parse();
    
    match cli.command {
        Some(Commands::Open) | None => handle_open(&cli),
        Some(Commands::Dump { format }) => handle_dump(&cli, &format),
        Some(Commands::List) => handle_list(),
        Some(Commands::Init { target }) => handle_init(&target),
        Some(Commands::Test) => handle_test(&cli),
    }
}

fn handle_open(cli: &Cli) {
    println!("Starting Anvil Pantry...");
    println!("Directory: {}", cli.dir.display());
    println!("\nTo run the pantry:");
    println!("  cd {} && cargo run", cli.dir.display());
}

fn handle_dump(cli: &Cli, format: &str) {
    println!("Dumping widgets in {} format...", format);
    println!("Directory: {}", cli.dir.display());
}

fn handle_list() {
    println!("Available widgets:");
    println!("  TopBar::Default");
    println!("  MessageList::With Messages");
    println!("  InputBar::Default");
    println!("  Overlay::Skills");
    println!("  StatusBar::Chat Mode");
}

fn handle_init(target: &PathBuf) {
    println!("Initializing pantry at: {}", target.display());
    std::fs::create_dir_all(target).expect("Failed to create directory");
    create_pantry_toml(target);
    println!("Created pantry.toml");
    println!("\nNext steps:");
    println!("  1. cd {}", target.display());
    println!("  2. Create src/main.rs with your ingredients");
    println!("  3. cargo run");
}

fn create_pantry_toml(target: &PathBuf) {
    let pantry_toml = r##"[config]
name = "Anvil Pantry"
description = "Widget development environment"
version = "0.1.0"

[pantry.dark]
accent = "#5ccfe6"
panel_bg = "#252525"
cursor_bg = "#2a2a2a"
border = "#404040"
border_dim = "#333333"
text = "#e0e0e0"
text_dim = "#808080"
doc_accent = "#5ccfe6"
doc_text = "#e0e0e0"
doc_type = "#c792ea"
indicator = "#5ccfe6"
dark = true
"##;
    std::fs::write(target.join("pantry.toml"), pantry_toml)
        .expect("Failed to write pantry.toml");
}

fn handle_test(cli: &Cli) {
    println!("Running pantry tests...");
    println!("Directory: {}", cli.dir.display());
    let output = Command::new("cargo").args(["test"]).current_dir(&cli.dir).output();
    match output {
        Ok(output) => {
            if output.status.success() {
                println!("All tests passed!");
            } else {
                eprintln!("Tests failed:");
                eprintln!("{}", String::from_utf8_lossy(&output.stderr));
            }
        }
        Err(e) => {
            eprintln!("Failed to run tests: {}", e);
            std::process::exit(1);
        }
    }
}
        Some(Commands::Dump { format }) => {
            println!("Dumping widgets in {} format...", format);
            println!("Directory: {}", cli.dir.display());
        }
        Some(Commands::List) => {
            println!("Available widgets:");
            println!("  TopBar::Default");
            println!("  MessageList::With Messages");
            println!("  InputBar::Default");
            println!("  Overlay::Skills");
            println!("  StatusBar::Chat Mode");
        }
        Some(Commands::Init { target }) => {
            println!("Initializing pantry at: {}", target.display());
            
            std::fs::create_dir_all(&target).expect("Failed to create directory");
            
            // Create default pantry.toml
            let pantry_toml = r##"[config]
name = "Anvil Pantry"
description = "Widget development environment"
version = "0.1.0"

[pantry.dark]
accent = "#5ccfe6"
panel_bg = "#252525"
cursor_bg = "#2a2a2a"
border = "#404040"
border_dim = "#333333"
text = "#e0e0e0"
text_dim = "#808080"
doc_accent = "#5ccfe6"
doc_text = "#e0e0e0"
doc_type = "#c792ea"
indicator = "#5ccfe6"
dark = true
"##;
            
            std::fs::write(target.join("pantry.toml"), pantry_toml)
                .expect("Failed to write pantry.toml");
            
            println!("Created pantry.toml");
            println!("\nNext steps:");
            println!("  1. cd {}", target.display());
            println!("  2. Create src/main.rs with your ingredients");
            println!("  3. cargo run");
        }
        Some(Commands::Test) => {
            println!("Running pantry tests...");
            println!("Directory: {}", cli.dir.display());
            
            // Run cargo test in pantry directory
            let output = Command::new("cargo")
                .args(["test"])
                .current_dir(&cli.dir)
                .output();
            
            match output {
                Ok(output) => {
                    if output.status.success() {
                        println!("All tests passed!");
                    } else {
                        eprintln!("Tests failed:");
                        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
                    }
                }
                Err(e) => {
                    eprintln!("Failed to run tests: {}", e);
                }
            }
        }
    }
}