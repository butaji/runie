//! Actor-owned provider configuration and connection state.

use crate::task_owner::{mailbox_call, spawn_actor_worker, TaskOwner};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::{mpsc, watch};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub id: String,
    pub label: String,
    pub base_url: String,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub selected_model: Option<String>,
    #[serde(default)]
    pub connected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ProviderRegistryState {
    pub providers: Vec<ProviderConfig>,
    pub active_provider: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderEvent {
    Connected(ProviderConfig),
    Disconnected { provider_id: String },
    Updated(ProviderConfig),
    Selected { provider_id: String, model: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SupportedProvider {
    pub id: &'static str,
    pub label: &'static str,
    pub base_url: &'static str,
    pub models: &'static [&'static str],
}

const MINIMAX_MODELS: &[&str] = &["MiniMax-M2.7", "MiniMax-M2.7-highspeed", "MiniMax-M3"];

/// Provider choices mirrored from pi's built-in provider registry.
const SUPPORTED_PROVIDERS: &[SupportedProvider] = &[
    SupportedProvider {
        id: "amazon-bedrock",
        label: "Amazon Bedrock",
        base_url: "",
        models: &[],
    },
    SupportedProvider {
        id: "anthropic",
        label: "Anthropic",
        base_url: "https://api.anthropic.com",
        models: &[],
    },
    SupportedProvider {
        id: "azure-openai-responses",
        label: "Azure OpenAI",
        base_url: "",
        models: &[],
    },
    SupportedProvider {
        id: "baseten",
        label: "Baseten",
        base_url: "https://inference.baseten.co/v1",
        models: &[],
    },
    SupportedProvider {
        id: "cerebras",
        label: "Cerebras",
        base_url: "https://api.cerebras.ai/v1",
        models: &[],
    },
    SupportedProvider {
        id: "cloudflare-ai-gateway",
        label: "Cloudflare AI Gateway",
        base_url: "",
        models: &[],
    },
    SupportedProvider {
        id: "cloudflare-workers-ai",
        label: "Cloudflare Workers AI",
        base_url: "",
        models: &[],
    },
    SupportedProvider {
        id: "deepseek",
        label: "DeepSeek",
        base_url: "https://api.deepseek.com",
        models: &[],
    },
    SupportedProvider {
        id: "fireworks",
        label: "Fireworks AI",
        base_url: "https://api.fireworks.ai/inference/v1",
        models: &[],
    },
    SupportedProvider {
        id: "github-copilot",
        label: "GitHub Copilot",
        base_url: "",
        models: &[],
    },
    SupportedProvider {
        id: "google",
        label: "Google",
        base_url: "https://generativelanguage.googleapis.com",
        models: &[],
    },
    SupportedProvider {
        id: "google-vertex",
        label: "Google Vertex",
        base_url: "",
        models: &[],
    },
    SupportedProvider {
        id: "groq",
        label: "Groq",
        base_url: "https://api.groq.com/openai/v1",
        models: &[],
    },
    SupportedProvider {
        id: "huggingface",
        label: "Hugging Face",
        base_url: "https://router.huggingface.co/v1",
        models: &[],
    },
    SupportedProvider {
        id: "kimi-coding",
        label: "Kimi Coding",
        base_url: "https://api.kimi.com/coding/v1",
        models: &[],
    },
    SupportedProvider {
        id: "minimax",
        label: "MiniMax",
        base_url: "https://api.minimax.io/v1/text/chatcompletion_v2",
        models: MINIMAX_MODELS,
    },
    SupportedProvider {
        id: "minimax-cn",
        label: "MiniMax CN",
        base_url: "https://api.minimaxi.com/v1/text/chatcompletion_v2",
        models: MINIMAX_MODELS,
    },
    SupportedProvider {
        id: "mistral",
        label: "Mistral",
        base_url: "https://api.mistral.ai/v1",
        models: &[],
    },
    SupportedProvider {
        id: "moonshotai",
        label: "Moonshot AI",
        base_url: "https://api.moonshot.ai/v1",
        models: &[],
    },
    SupportedProvider {
        id: "moonshotai-cn",
        label: "Moonshot AI CN",
        base_url: "https://api.moonshot.cn/v1",
        models: &[],
    },
    SupportedProvider {
        id: "nvidia",
        label: "NVIDIA",
        base_url: "https://integrate.api.nvidia.com/v1",
        models: &[],
    },
    SupportedProvider {
        id: "openai",
        label: "OpenAI",
        base_url: "https://api.openai.com/v1",
        models: &[],
    },
    SupportedProvider {
        id: "openai-codex",
        label: "OpenAI Codex",
        base_url: "",
        models: &[],
    },
    SupportedProvider {
        id: "opencode",
        label: "OpenCode",
        base_url: "https://opencode.ai/zen/v1",
        models: &[],
    },
    SupportedProvider {
        id: "opencode-go",
        label: "OpenCode Go",
        base_url: "",
        models: &[],
    },
    SupportedProvider {
        id: "openrouter",
        label: "OpenRouter",
        base_url: "https://openrouter.ai/api/v1",
        models: &[],
    },
    SupportedProvider {
        id: "qwen-token-plan",
        label: "Qwen Token Plan",
        base_url: "",
        models: &[],
    },
    SupportedProvider {
        id: "qwen-token-plan-cn",
        label: "Qwen Token Plan CN",
        base_url: "",
        models: &[],
    },
    SupportedProvider {
        id: "qwen-token-plan-individual",
        label: "Qwen Token Plan Individual",
        base_url: "",
        models: &[],
    },
    SupportedProvider {
        id: "radius",
        label: "Radius",
        base_url: "",
        models: &[],
    },
    SupportedProvider {
        id: "together",
        label: "Together AI",
        base_url: "https://api.together.xyz/v1",
        models: &[],
    },
    SupportedProvider {
        id: "vercel-ai-gateway",
        label: "Vercel AI Gateway",
        base_url: "",
        models: &[],
    },
    SupportedProvider {
        id: "xai",
        label: "xAI",
        base_url: "https://api.x.ai/v1",
        models: &[],
    },
    SupportedProvider {
        id: "xiaomi",
        label: "Xiaomi",
        base_url: "",
        models: &[],
    },
    SupportedProvider {
        id: "xiaomi-token-plan-ams",
        label: "Xiaomi Token Plan AMS",
        base_url: "",
        models: &[],
    },
    SupportedProvider {
        id: "xiaomi-token-plan-cn",
        label: "Xiaomi Token Plan CN",
        base_url: "",
        models: &[],
    },
    SupportedProvider {
        id: "xiaomi-token-plan-sgp",
        label: "Xiaomi Token Plan SGP",
        base_url: "",
        models: &[],
    },
    SupportedProvider {
        id: "zai",
        label: "Z.AI",
        base_url: "https://api.z.ai/api/paas/v4",
        models: &[],
    },
    SupportedProvider {
        id: "zai-coding-cn",
        label: "Z.AI Coding CN",
        base_url: "",
        models: &[],
    },
];
pub const fn supported_providers() -> &'static [SupportedProvider] {
    SUPPORTED_PROVIDERS
}

enum ProviderCommand {
    Event(ProviderEvent, tokio::sync::mpsc::Sender<()>),
    Replace(ProviderRegistryState, tokio::sync::mpsc::Sender<()>),
}

/// SSOT actor for configured providers and their active model selection.
#[derive(Clone)]
pub struct ProviderRegistryActor {
    tx: mpsc::Sender<ProviderCommand>,
    snapshot: watch::Receiver<ProviderRegistryState>,
    _worker: Arc<TaskOwner>,
}

impl ProviderRegistryActor {
    pub fn new(initial: ProviderRegistryState) -> Self {
        let (snapshot_tx, snapshot) = watch::channel(initial);
        let (tx, worker) = spawn_actor_worker!(64, move |mut rx: mpsc::Receiver<
            ProviderCommand,
        >| async move {
            while let Some(command) = rx.recv().await {
                let (next, reply) = match command {
                    ProviderCommand::Event(event, reply) => (
                        reduce_provider_event(snapshot_tx.borrow().clone(), event),
                        reply,
                    ),
                    ProviderCommand::Replace(state, reply) => (state, reply),
                };
                let _ = snapshot_tx.send(next);
                let _ = reply.send(()).await;
            }
        });
        Self {
            tx,
            snapshot,
            _worker: worker,
        }
    }

    pub fn snapshot(&self) -> ProviderRegistryState {
        self.snapshot.borrow().clone()
    }

    pub async fn apply(&self, event: ProviderEvent) {
        mailbox_call!(self.tx, |reply| ProviderCommand::Event(event, reply), ());
    }

    pub async fn replace(&self, state: ProviderRegistryState) {
        mailbox_call!(self.tx, |reply| ProviderCommand::Replace(state, reply), ());
    }
}

pub fn reduce_provider_event(
    mut state: ProviderRegistryState,
    event: ProviderEvent,
) -> ProviderRegistryState {
    match event {
        ProviderEvent::Connected(mut provider) => {
            provider.connected = true;
            upsert(&mut state.providers, provider.clone());
            state.active_provider = Some(provider.id);
        }
        ProviderEvent::Disconnected { provider_id } => {
            if let Some(provider) = state.providers.iter_mut().find(|p| p.id == provider_id) {
                provider.connected = false;
            }
            if state.active_provider.as_deref() == Some(provider_id.as_str()) {
                state.active_provider = None;
            }
        }
        ProviderEvent::Updated(provider) => upsert(&mut state.providers, provider),
        ProviderEvent::Selected { provider_id, model } => {
            if let Some(provider) = state.providers.iter_mut().find(|p| p.id == provider_id) {
                provider.selected_model = Some(model);
                state.active_provider = Some(provider_id);
            }
        }
    }
    state
}

pub async fn load_provider_config(path: impl AsRef<Path>) -> Result<ProviderRegistryState, String> {
    let bytes = tokio::fs::read(path.as_ref())
        .await
        .map_err(|error| error.to_string())?;
    serde_json::from_slice(&bytes).map_err(|error| error.to_string())
}

pub async fn save_provider_config(
    path: impl AsRef<Path>,
    state: &ProviderRegistryState,
) -> Result<(), String> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|error| error.to_string())?;
    }
    let bytes = serde_json::to_vec_pretty(state).map_err(|error| error.to_string())?;
    tokio::fs::write(path, bytes)
        .await
        .map_err(|error| error.to_string())
}

fn upsert(providers: &mut Vec<ProviderConfig>, provider: ProviderConfig) {
    if let Some(existing) = providers.iter_mut().find(|item| item.id == provider.id) {
        *existing = provider;
    } else {
        providers.push(provider);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn provider() -> ProviderConfig {
        ProviderConfig {
            id: "minimax".into(),
            label: "MiniMax".into(),
            base_url: "https://api.minimax.io".into(),
            api_key: Some("secret".into()),
            selected_model: None,
            connected: false,
        }
    }

    #[tokio::test]
    async fn config_round_trip_preserves_provider_state() {
        let directory =
            std::env::temp_dir().join(format!("runie-provider-registry-{}", std::process::id()));
        let path = directory.join("providers.json");
        let state = reduce_provider_event(
            ProviderRegistryState::default(),
            ProviderEvent::Connected(provider()),
        );
        save_provider_config(&path, &state)
            .await
            .expect("save config");
        assert_eq!(
            load_provider_config(&path).await.expect("load config"),
            state
        );
        tokio::fs::remove_dir_all(directory)
            .await
            .expect("remove test config");
    }

    #[test]
    fn event_sequence_connect_update_select_disconnect_is_actor_reducible() {
        let state = reduce_provider_event(
            ProviderRegistryState::default(),
            ProviderEvent::Connected(provider()),
        );
        assert_eq!(state.active_provider.as_deref(), Some("minimax"));
        assert!(state.providers[0].connected);
        let state = reduce_provider_event(
            state,
            ProviderEvent::Selected {
                provider_id: "minimax".into(),
                model: "MiniMax-M2.5".into(),
            },
        );
        assert_eq!(
            state.providers[0].selected_model.as_deref(),
            Some("MiniMax-M2.5")
        );
        let mut updated = provider();
        updated.label = "MiniMax Updated".into();
        let state = reduce_provider_event(state, ProviderEvent::Updated(updated));
        assert_eq!(state.providers[0].label, "MiniMax Updated");
        let state = reduce_provider_event(
            state,
            ProviderEvent::Disconnected {
                provider_id: "minimax".into(),
            },
        );
        assert!(!state.providers[0].connected);
        assert!(state.active_provider.is_none());
    }

    #[test]
    fn supported_catalog_contains_pi_providers_and_minimax_models() {
        let providers = supported_providers();
        assert_eq!(providers.len(), 39);
        let minimax = providers
            .iter()
            .find(|provider| provider.id == "minimax")
            .unwrap();
        assert_eq!(
            minimax.models,
            ["MiniMax-M2.7", "MiniMax-M2.7-highspeed", "MiniMax-M3"]
        );
        assert!(providers
            .iter()
            .any(|provider| provider.id == "openai-codex"));
        assert!(providers.iter().any(|provider| provider.id == "minimax-cn"));
    }
}
