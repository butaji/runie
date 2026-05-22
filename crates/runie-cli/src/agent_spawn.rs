use std::sync::Arc;
use tokio::sync::mpsc;
use runie_agent::events::{AgentEvent, PermissionDecision};
use runie_agent::loop_engine::{run_agent_loop, AgentLoopConfig};
use runie_agent::pi::AgentTool;
use runie_agent::{SafetyHook, Hook};
use runie_tools::{create_default_toolkit, Workspace};
use runie_ai::Provider;
use runie_ai::providers::{OpenAiProvider, AnthropicProvider};

pub fn spawn_agent_task(
    messages: Vec<runie_agent::events::AgentMessage>,
    event_tx: mpsc::UnboundedSender<AgentEvent>,
    perm_rx: mpsc::UnboundedReceiver<PermissionDecision>,
    workspace: &std::path::PathBuf,
    model: &str,
    provider_name: &str,
    api_key: &Option<String>,
    base_url: &Option<String>,
    enable_thinking: bool,
    max_turns: usize,
    system_prompt: String,
) -> tokio::task::JoinHandle<()> {
    let ws = Workspace::new(workspace.clone());
    let registry = Arc::new(create_default_toolkit(ws));
    let tools = create_agent_tools(registry.clone());
    let safety_hook: Arc<dyn Hook> = Arc::new(SafetyHook);
    let hooks: Vec<Arc<dyn Hook>> = vec![safety_hook];

    let model_str = model.to_string();
    let provider_str = provider_name.to_string();
    let api_key_clone = api_key.clone();
    let base_url_clone = base_url.clone();

    tokio::spawn(async move {
        let provider = match create_provider_internal(&model_str, &provider_str, &api_key_clone, &base_url_clone) {
            Ok(p) => p,
            Err(e) => {
                event_tx.send(AgentEvent::Error { message: e }).ok();
                return;
            }
        };

        let config = AgentLoopConfig {
            system_prompt,
            model: model_str.clone(),
            thinking_level: if enable_thinking { "high" } else { "low" }.to_string(),
            max_turns,
        };

        match run_agent_loop(
            messages,
            config,
            provider.as_ref(),
            &tools,
            event_tx,
            perm_rx,
            Some(registry),
            hooks,
        ).await {
            Ok(_) => {},
            Err(e) => eprintln!("Agent error: {}", e),
        }
    })
}

fn create_provider_internal(
    model: &str,
    provider_name: &str,
    api_key: &Option<String>,
    base_url: &Option<String>,
) -> Result<Box<dyn Provider>, String> {
    match provider_name {
        "openai" => {
            let key = api_key.clone()
                .or_else(|| std::env::var("OPENAI_API_KEY").ok())
                .ok_or("OpenAI API key required. Set OPENAI_API_KEY env var or use --api-key")?;
            let mut provider = OpenAiProvider::new(key, model.to_string());
            if let Some(ref url) = base_url {
                provider = provider.with_base_url(url.clone());
            }
            Ok(Box::new(provider))
        }
        "anthropic" => {
            let key = api_key.clone()
                .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok())
                .ok_or("Anthropic API key required. Set ANTHROPIC_API_KEY env var or use --api-key")?;
            let mut provider = AnthropicProvider::new(key, model.to_string());
            if let Some(ref url) = base_url {
                provider = provider.with_base_url(url.clone());
            }
            Ok(Box::new(provider))
        }
        other => Err(format!("Unknown provider: {}. Use 'openai' or 'anthropic'", other)),
    }
}

pub fn create_agent_tools(registry: Arc<runie_tools::ToolRegistry>) -> Vec<AgentTool> {
    let handle = tokio::runtime::Handle::current();

    registry.list().into_iter().map(|tool| {
        let name = tool.name().to_string();
        let description = tool.description().to_string();
        let parameters = tool.schema().parameters;
        let registry_clone = registry.clone();
        let handle_clone = handle.clone();

        AgentTool::new(name.clone(), description, parameters).with_handler(
            Arc::new(move |args| {
                let registry = registry_clone.clone();
                let handle = handle_clone.clone();
                let name = name.clone();
                handle.block_on(async move {
                    match registry.get(&name) {
                        Some(t) => t.execute(args).await
                            .map(|o| o.content)
                            .map_err(|e| e.to_string()),
                        None => Err(format!("Tool not found: {}", name)),
                    }
                })
            }),
        )
    }).collect()
}