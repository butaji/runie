use std::path::PathBuf;

use runie_ai::providers::MockProvider;
use runie_ai::Provider;

use crate::provider_factory::create_provider;
use crate::settings::{CliSettings, Settings};

#[derive(Clone, Default, PartialEq, clap::ValueEnum)]
pub enum OutputFormat {
    #[default]
    Plain,
    Json,
    StreamingJson,
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Plain => write!(f, "plain"),
            OutputFormat::Json => write!(f, "json"),
            OutputFormat::StreamingJson => write!(f, "streaming-json"),
        }
    }
}

pub fn is_headless(cli: &crate::Cli) -> bool {
    cli.single.is_some()
        || cli.model.is_some()
        || cli.session_id.is_some()
        || cli.resume.is_some()
        || cli.continue_
        || cli.cwd.is_some()
        || cli.output_format != OutputFormat::Plain
        || cli.always_approve
        || cli.no_alt_screen
        || cli.no_auto_update
}

pub fn build_headless_settings(cli: &crate::Cli, _settings: &mut Settings) -> CliSettings {
    if let Some(ref model) = cli.model {
        let _ = model;
    }
    CliSettings {
        model: cli.model.clone(),
        ..Default::default()
    }
}

/// Run in headless mode with output format support
pub async fn run_headless(
    cli: &crate::Cli,
    headless_settings: &CliSettings,
) -> Result<(), Box<dyn std::error::Error>> {
    let prompt = cli.single.clone().unwrap_or_default();
    let workspace = cli.cwd.clone().unwrap_or_else(|| cli.workspace.clone());
    let output_format = &cli.output_format;

    match output_format {
        OutputFormat::Plain => {
            run_headless_plain(&prompt, &workspace, cli.mock, headless_settings).await?;
        }
        OutputFormat::Json => {
            run_headless_json(&prompt, &workspace, cli.mock, headless_settings).await?;
        }
        OutputFormat::StreamingJson => {
            run_headless_streaming_json(&prompt, &workspace, cli.mock, headless_settings).await?;
        }
    }

    Ok(())
}

async fn run_headless_plain(
    prompt: &str,
    _workspace: &PathBuf,
    mock: bool,
    _settings: &CliSettings,
) -> Result<(), Box<dyn std::error::Error>> {
    if prompt.is_empty() {
        return Ok(());
    }

    println!("❯ {}", prompt);
    println!();

    let mut settings = Settings::load();

    if mock {
        let mock_provider = MockProvider::new();
        let messages = vec![runie_core::Message::User {
            content: prompt.to_string(),
            attachments: vec![],
        }];
        match mock_provider.chat_simple(messages).await {
            Ok(response) => println!("{}", response),
            Err(e) => eprintln!("Mock error: {}", e),
        }
    } else {
        let provider = match create_provider(mock, &settings) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Error: {}", e);
                return Ok(());
            }
        };
        println!("Model: {} ({})", settings.model, settings.provider);
        println!("Processing...");

        let messages = vec![runie_core::Message::User {
            content: prompt.to_string(),
            attachments: vec![],
        }];

        match provider.chat_simple(messages).await {
            Ok(response) => println!("{}", response),
            Err(e) => eprintln!("Error: {}", e),
        }
    }
    Ok(())
}

async fn run_headless_json(
    prompt: &str,
    workspace: &PathBuf,
    mock: bool,
    _settings: &CliSettings,
) -> Result<(), Box<dyn std::error::Error>> {
    #[derive(serde::Serialize)]
    struct JsonOutput {
        prompt: String,
        workspace: String,
        response: Option<String>,
        error: Option<String>,
    }

    if prompt.is_empty() {
        let output = JsonOutput {
            prompt: String::new(),
            workspace: workspace.to_string_lossy().to_string(),
            response: None,
            error: Some("No prompt provided".to_string()),
        };
        println!("{}", serde_json::to_string(&output)?);
        return Ok(());
    }

    let mut settings = Settings::load();
    let mut response_content: Option<String> = None;
    let mut error_content: Option<String> = None;

    if mock {
        let mock_provider = MockProvider::new();
        let messages = vec![runie_core::Message::User {
            content: prompt.to_string(),
            attachments: vec![],
        }];
        match mock_provider.chat_simple(messages).await {
            Ok(response) => response_content = Some(response),
            Err(e) => error_content = Some(e.to_string()),
        }
    } else {
        let provider = match create_provider(mock, &settings) {
            Ok(p) => p,
            Err(e) => {
                let output = JsonOutput {
                    prompt: prompt.to_string(),
                    workspace: workspace.to_string_lossy().to_string(),
                    response: None,
                    error: Some(e.to_string()),
                };
                println!("{}", serde_json::to_string(&output)?);
                return Ok(());
            }
        };

        let messages = vec![runie_core::Message::User {
            content: prompt.to_string(),
            attachments: vec![],
        }];

        match provider.chat_simple(messages).await {
            Ok(response) => response_content = Some(response),
            Err(e) => error_content = Some(e.to_string()),
        }
    }

    let output = JsonOutput {
        prompt: prompt.to_string(),
        workspace: workspace.to_string_lossy().to_string(),
        response: response_content,
        error: error_content,
    };

    println!("{}", serde_json::to_string(&output)?);
    Ok(())
}

async fn run_headless_streaming_json(
    prompt: &str,
    workspace: &PathBuf,
    mock: bool,
    _settings: &CliSettings,
) -> Result<(), Box<dyn std::error::Error>> {
    #[derive(serde::Serialize)]
    struct StreamEvent {
        event: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<serde_json::Value>,
    }

    // Emit start event
    println!(
        "{}",
        serde_json::to_string(&StreamEvent {
            event: "start".to_string(),
            data: Some(serde_json::json!({
                "prompt": prompt,
                "workspace": workspace.to_string_lossy()
            })),
        })?
    );

    if prompt.is_empty() {
        println!(
            "{}",
            serde_json::to_string(&StreamEvent {
                event: "error".to_string(),
                data: Some(serde_json::json!({"message": "No prompt provided"})),
            })?
        );
        return Ok(());
    }

    let mut settings = Settings::load();

    if mock {
        let mock_provider = MockProvider::new();
        let messages = vec![runie_core::Message::User {
            content: prompt.to_string(),
            attachments: vec![],
        }];
        match mock_provider.chat_simple(messages).await {
            Ok(response) => {
                println!(
                    "{}",
                    serde_json::to_string(&StreamEvent {
                        event: "response".to_string(),
                        data: Some(serde_json::json!({"content": response})),
                    })?
                );
            }
            Err(e) => {
                println!(
                    "{}",
                    serde_json::to_string(&StreamEvent {
                        event: "error".to_string(),
                        data: Some(serde_json::json!({"message": e.to_string()})),
                    })?
                );
            }
        }
    } else {
        let provider = match create_provider(mock, &settings) {
            Ok(p) => p,
            Err(e) => {
                println!(
                    "{}",
                    serde_json::to_string(&StreamEvent {
                        event: "error".to_string(),
                        data: Some(serde_json::json!({"message": e.to_string()})),
                    })?
                );
                return Ok(());
            }
        };

        let messages = vec![runie_core::Message::User {
            content: prompt.to_string(),
            attachments: vec![],
        }];

        match provider.chat_simple(messages).await {
            Ok(response) => {
                println!(
                    "{}",
                    serde_json::to_string(&StreamEvent {
                        event: "response".to_string(),
                        data: Some(serde_json::json!({"content": response})),
                    })?
                );
            }
            Err(e) => {
                println!(
                    "{}",
                    serde_json::to_string(&StreamEvent {
                        event: "error".to_string(),
                        data: Some(serde_json::json!({"message": e.to_string()})),
                    })?
                );
            }
        }
    }

    // Emit end event
    println!(
        "{}",
        serde_json::to_string(&StreamEvent {
            event: "end".to_string(),
            data: None,
        })?
    );

    Ok(())
}

/// CLI: One-shot execution without TUI.
pub async fn run_cli_one_shot(
    prompt: &str,
    _workspace: &PathBuf,
    mock: bool,
    settings: &Settings,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("❯ {}", prompt);
    println!();

    if mock {
        let mock_provider = MockProvider::new();
        let messages = vec![runie_core::Message::User {
            content: prompt.to_string(),
            attachments: vec![],
        }];
        match mock_provider.chat_simple(messages).await {
            Ok(response) => println!("{}", response),
            Err(e) => eprintln!("Mock error: {}", e),
        }
    } else {
        let provider = match create_provider(mock, settings) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Error: {}", e);
                return Ok(());
            }
        };
        println!("Model: {} ({})", settings.model, settings.provider);
        println!("Processing...");

        let messages = vec![runie_core::Message::User {
            content: prompt.to_string(),
            attachments: vec![],
        }];

        match provider.chat_simple(messages).await {
            Ok(response) => println!("{}", response),
            Err(e) => eprintln!("Error: {}", e),
        }
    }
    Ok(())
}
