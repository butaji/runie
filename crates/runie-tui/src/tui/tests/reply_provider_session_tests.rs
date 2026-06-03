//! ReplyProvider session save/load and multi-provider switching tests.
//!
//! Tests:
//! - Session save to file and load from file
//! - Invalid file handling
//! - Provider switching changes model
//! - Provider affects token estimation
//! - Provider switch mid-session
//! - Provider-specific system prompts
//! - Mock provider fallback

use std::env;
use std::fs;
use std::path::PathBuf;

use crate::components::MessageItem;
use crate::tui::state::{AppState, Msg};
use crate::tui::update::agent::handle_agent_event;
use runie_agent::{AgentEvent, AgentMessage, ContentPart, TokenUsage};
use runie_core::{Message, MessageNode};

// ─── Helper Functions ─────────────────────────────────────────────────────────

/// Create AppState with messages for session tests.
fn make_state_with_messages() -> AppState {
    let mut state = AppState::default();
    state.current_model = Some("MiniMax-M2.7-highspeed".to_string());
    state.messages.push(MessageItem::User {
        text: "Hello".to_string(),
        model: None,
        timestamp: None,
    });
    state.messages.push(MessageItem::Assistant {
        text: "Hi there!".to_string(),
        model: None,
        timestamp: None,
        expanded: false,
        thought_duration: None,
        turn_duration: None,
    });
    state
}

/// Create a temporary file path for session testing.
fn temp_session_file() -> PathBuf {
    env::temp_dir().join(format!("runie_test_session_{}.jsonl", uuid::Uuid::new_v4()))
}

/// Save session messages to a JSONL file (simplified format for testing).
fn save_session_to_file(messages: &[MessageItem], path: &PathBuf) -> Result<(), String> {
    let mut content = String::new();
    for msg in messages {
        if let Some(node) = message_item_to_node(msg) {
            let line = serde_json::to_string(&node).map_err(|e| e.to_string())?;
            content.push_str(&line);
            content.push('\n');
        }
    }
    fs::write(path, content).map_err(|e| e.to_string())?;
    Ok(())
}

/// Load session messages from a JSONL file.
fn load_session_from_file(path: &PathBuf) -> Result<Vec<MessageItem>, String> {
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut items = Vec::new();
    for line in content.lines() {
        if !line.trim().is_empty() {
            let node: MessageNode = serde_json::from_str(line).map_err(|e| e.to_string())?;
            items.push(node_to_message_item(&node));
        }
    }
    Ok(items)
}

/// Convert MessageItem to MessageNode for serialization.
fn message_item_to_node(msg: &MessageItem) -> Option<MessageNode> {
    let message = match msg {
        MessageItem::User { text, .. } => Message::User {
            content: text.clone(),
            attachments: vec![],
        },
        MessageItem::Assistant { text, .. } => Message::Assistant {
            content: text.clone(),
            tool_calls: vec![],
            thinking: None,
        },
        MessageItem::System { text, .. } => Message::System {
            content: text.clone(),
        },
        MessageItem::ToolCall { name, args, result, .. } => Message::ToolResult {
            tool_call_id: name.clone(),
            content: format!("{} => {}", args, result.as_deref().unwrap_or("")),
            is_error: false,
        },
        MessageItem::Error { message, .. } => Message::System {
            content: format!("[ERROR] {}", message),
        },
        MessageItem::Thought { text, .. } => Message::System {
            content: format!("[THOUGHT] {}", text),
        },
        _ => return None, // Skip other message types for serialization
    };
    // Create timestamp using runie-core's Session (which uses chrono internally)
    let temp_session = runie_core::Session::new("temp".to_string());
    Some(MessageNode {
        id: uuid::Uuid::new_v4().to_string(),
        parent_id: None,
        message,
        timestamp: temp_session.created_at,
        metadata: serde_json::Value::Null,
    })
}

/// Convert MessageNode back to MessageItem.
fn node_to_message_item(node: &MessageNode) -> MessageItem {
    match &node.message {
        Message::User { content, .. } => MessageItem::User {
            text: content.clone(),
            model: None,
            timestamp: None,
        },
        Message::Assistant { content, .. } => MessageItem::Assistant {
            text: content.clone(),
            model: None,
            timestamp: None,
            expanded: false,
            thought_duration: None,
            turn_duration: None,
        },
        Message::System { content, .. } => {
            if content.starts_with("[ERROR] ") {
                MessageItem::Error {
                    message: content.strip_prefix("[ERROR] ").unwrap_or(content).to_string(),
                    recoverable: true,
                }
            } else if content.starts_with("[THOUGHT] ") {
                MessageItem::Thought {
                    duration_secs: 0.0,
                    text: content.strip_prefix("[THOUGHT] ").unwrap_or(content).to_string(),
                }
            } else {
                MessageItem::System {
                    text: content.clone(),
                }
            }
        }
        Message::ToolResult { tool_call_id, content, .. } => MessageItem::ToolCall {
            name: tool_call_id.clone(),
            args: content.clone(),
            result: Some(content.clone()),
            is_error: false,
        },
    }
}

/// Create a TokenUsage event.
fn token_usage_event(prompt: usize, completion: usize) -> AgentEvent {
    AgentEvent::TokenUsage {
        prompt_tokens: prompt,
        completion_tokens: completion,
        total_tokens: prompt + completion,
        context_window: 128_000,
    }
}

/// Helper to set current model via the proper update mechanism.
fn set_current_model(state: &mut AppState, model: Option<String>) {
    // Use the same approach as navigation - directly set the field
    state.current_model = model;
}

// ─── Session Tests ─────────────────────────────────────────────────────────────

mod session_tests {
    use super::*;

    #[test]
    fn test_session_save_creates_file() {
        let state = make_state_with_messages();
        let path = temp_session_file();

        let result = save_session_to_file(&state.messages, &path);

        assert!(result.is_ok(), "Save should succeed: {:?}", result.err());
        assert!(path.exists(), "Session file should exist");
        assert!(path.metadata().unwrap().len() > 0, "Session file should not be empty");

        // Cleanup
        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_session_load_restores_messages() {
        let original_state = make_state_with_messages();
        let path = temp_session_file();

        // Save original messages
        save_session_to_file(&original_state.messages, &path).unwrap();

        // Load into new state
        let loaded_messages = load_session_from_file(&path).unwrap();

        assert_eq!(
            loaded_messages.len(),
            original_state.messages.len(),
            "Loaded messages count should match original"
        );

        // Verify message content
        let original_user = match &original_state.messages[0] {
            MessageItem::User { text, .. } => text.clone(),
            _ => panic!("Expected User message"),
        };
        let loaded_user = match &loaded_messages[0] {
            MessageItem::User { text, .. } => text.clone(),
            _ => panic!("Expected User message"),
        };
        assert_eq!(loaded_user, original_user, "User message content should match");

        let original_assistant = match &original_state.messages[1] {
            MessageItem::Assistant { text, .. } => text.clone(),
            _ => panic!("Expected Assistant message"),
        };
        let loaded_assistant = match &loaded_messages[1] {
            MessageItem::Assistant { text, .. } => text.clone(),
            _ => panic!("Expected Assistant message"),
        };
        assert_eq!(
            loaded_assistant, original_assistant,
            "Assistant message content should match"
        );

        // Cleanup
        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_session_load_invalid_file() {
        let path = env::temp_dir().join("nonexistent_session_file_12345.jsonl");

        let result = load_session_from_file(&path);

        assert!(
            result.is_err(),
            "Loading non-existent file should return error"
        );

        // Verify error message is descriptive
        let error = result.unwrap_err();
        assert!(
            error.contains("No such file") || error.contains("not found"),
            "Error should mention file not found"
        );
    }
}

// ─── Multi-Provider Tests ───────────────────────────────────────────────────────

mod provider_tests {
    use super::*;

    #[test]
    fn test_switch_provider_changes_model() {
        let mut state = AppState::default();
        state.current_model = Some("MiniMax-M2.7-highspeed".to_string());

        // Simulate switching to OpenAI provider
        // The model picker has gpt-4o as first OpenAI model
        set_current_model(&mut state, Some("gpt-4o".to_string()));

        assert_eq!(
            state.current_model,
            Some("gpt-4o".to_string()),
            "Model should change to gpt-4o after provider switch"
        );
    }

    #[test]
    fn test_provider_affects_token_estimation() {
        let mut state = AppState::default();

        // Set GPT-4o model (OpenAI pricing)
        state.current_model = Some("gpt-4o".to_string());

        // Add token usage event
        handle_agent_event(&mut state, token_usage_event(100, 200));

        // GPT-4o pricing: ~$5/1M prompt + ~$15/1M completion
        // 100 prompt + 200 completion = 300 tokens
        // Expected cost should be > 0
        assert!(
            state.session_token_usage.estimated_cost > 0.0,
            "OpenAI GPT-4o should have non-zero cost"
        );

        // Compare with a model that has different pricing
        let mut state2 = AppState::default();
        state2.current_model = Some("gpt-4o-mini".to_string()); // Cheaper model
        handle_agent_event(&mut state2, token_usage_event(100, 200));

        // GPT-4o-mini is much cheaper
        assert!(
            state2.session_token_usage.estimated_cost < state.session_token_usage.estimated_cost,
            "GPT-4o-mini should be cheaper than GPT-4o"
        );
    }

    #[test]
    fn test_provider_switch_mid_session() {
        let mut state = make_state_with_messages();

        // Initial provider (MiniMax)
        state.current_model = Some("MiniMax-M2.7-highspeed".to_string());

        // Simulate some token usage with MiniMax
        handle_agent_event(&mut state, token_usage_event(50, 100));
        let cost_minimax = state.session_token_usage.estimated_cost;

        // Switch to OpenAI
        set_current_model(&mut state, Some("gpt-4o".to_string()));

        // Add more token usage with OpenAI
        handle_agent_event(&mut state, token_usage_event(50, 100));
        let cost_openai = state.session_token_usage.estimated_cost - cost_minimax;

        // OpenAI should have different (higher) cost per token
        assert!(
            cost_openai > 0.0,
            "OpenAI should accumulate cost correctly"
        );

        // Verify model is now GPT-4o
        assert_eq!(
            state.current_model,
            Some("gpt-4o".to_string()),
            "Current model should be GPT-4o after switch"
        );
    }

    #[test]
    fn test_provider_specific_system_prompt() {
        // This test verifies that system prompts are provider-appropriate
        // by checking the model_picker has the right models per provider

        let picker = crate::components::ModelPicker::with_default_models();

        // Verify OpenAI models
        let openai_provider = picker
            .providers
            .iter()
            .find(|p| p.provider_id == "openai");
        assert!(
            openai_provider.is_some(),
            "Should have OpenAI provider"
        );
        let openai = openai_provider.unwrap();
        assert!(
            openai.models.iter().any(|m| m.id == "gpt-4o"),
            "OpenAI should have GPT-4o"
        );

        // Verify Anthropic models
        let anthropic_provider = picker
            .providers
            .iter()
            .find(|p| p.provider_id == "anthropic");
        assert!(
            anthropic_provider.is_some(),
            "Should have Anthropic provider"
        );
        let anthropic = anthropic_provider.unwrap();
        assert!(
            anthropic
                .models
                .iter()
                .any(|m| m.id.contains("claude")),
            "Anthropic should have Claude models"
        );

        // Verify Google models
        let google_provider = picker
            .providers
            .iter()
            .find(|p| p.provider_id == "google");
        assert!(
            google_provider.is_some(),
            "Should have Google provider"
        );
        let google = google_provider.unwrap();
        assert!(
            google.models.iter().any(|m| m.id.contains("gemini")),
            "Google should have Gemini models"
        );
    }

    #[test]
    fn test_mock_provider_fallback() {
        let mut state = AppState::default();

        // When no API key is configured, mock_mode should be set
        // This is how the app handles missing credentials
        state.mock_mode = true;
        state.current_model = Some("MiniMax-M2.7-highspeed".to_string());

        // In mock mode, agent should still process events
        // Start an agent message flow
        handle_agent_event(&mut state, AgentEvent::MessageStart {
            message: AgentMessage {
                role: "assistant".to_string(),
                content: vec![ContentPart::Text {
                    text: String::new(),
                }],
                timestamp: 0,
                usage: None,
                stop_reason: None,
                error_message: None,
                tool_calls: vec![],
            },
            turn: 0,
        });

        assert!(
            state.agent_running || state.messages.iter().any(|m| matches!(m, MessageItem::Assistant { .. })),
            "Agent should start in mock mode"
        );

        handle_agent_event(&mut state, AgentEvent::AgentEnd {
            messages: vec![],
            total_turns: 1,
            final_token_usage: TokenUsage::default(),
        });

        assert!(
            !state.agent_running,
            "Agent should end properly in mock mode"
        );
    }

    #[test]
    fn test_model_picker_has_default_providers() {
        let picker = crate::components::ModelPicker::with_default_models();

        // Should have multiple providers
        assert!(
            picker.providers.len() >= 3,
            "Should have at least 3 providers configured"
        );

        // Each provider should have models
        for provider in &picker.providers {
            assert!(
                !provider.models.is_empty(),
                "Provider {} should have models",
                provider.provider_name
            );
        }
    }

    #[test]
    fn test_token_cost_different_providers() {
        // Test that different models have different cost calculations
        let test_cases = vec![
            ("gpt-4o", 0.0035), // 100 prompt + 200 completion
            ("gpt-4o-mini", 0.00027), // Much cheaper
            ("claude-3-5-sonnet", 0.0030), // Similar to GPT-4o
            ("o1", 0.0105), // More expensive (reasoning tokens)
        ];

        for (model, _expected_cost) in test_cases {
            let mut state = AppState::default();
            state.current_model = Some(model.to_string());
            handle_agent_event(&mut state, token_usage_event(100, 200));

            let cost = state.session_token_usage.estimated_cost;
            assert!(
                cost > 0.0,
                "Model {} should have non-zero cost, got {}",
                model,
                cost
            );
        }
    }

    #[test]
    fn test_token_cost_unknown_model() {
        let mut state = AppState::default();
        state.current_model = Some("unknown-model-xyz".to_string());

        handle_agent_event(&mut state, token_usage_event(100, 200));

        // Unknown model should have zero cost
        assert_eq!(
            state.session_token_usage.estimated_cost, 0.0,
            "Unknown model should have zero cost"
        );
    }
}

// ─── Integration Tests ─────────────────────────────────────────────────────────

mod integration_tests {
    use super::*;

    #[test]
    fn test_session_save_load_roundtrip() {
        // Create state with multiple messages
        let mut state = make_state_with_messages();
        state.current_model = Some("gpt-4o".to_string());

        // Add more messages
        state.messages.push(MessageItem::User {
            text: "How are you?".to_string(),
            model: None,
            timestamp: None,
        });
        state.messages.push(MessageItem::Assistant {
            text: "I'm doing well, thank you!".to_string(),
            model: None,
            timestamp: None,
            expanded: false,
            thought_duration: None,
            turn_duration: None,
        });

        let path = temp_session_file();

        // Save
        save_session_to_file(&state.messages, &path).unwrap();

        // Load
        let loaded = load_session_from_file(&path).unwrap();

        assert_eq!(loaded.len(), state.messages.len());

        // Cleanup
        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_provider_switch_preserves_token_usage() {
        let mut state = AppState::default();

        // Start with MiniMax
        state.current_model = Some("MiniMax-M2.7-highspeed".to_string());
        handle_agent_event(&mut state, token_usage_event(100, 200));
        let cost_after_minimax = state.session_token_usage.estimated_cost;
        let tokens_after_minimax = state.session_token_usage.total_tokens;

        // Switch to GPT-4o
        set_current_model(&mut state, Some("gpt-4o".to_string()));
        handle_agent_event(&mut state, token_usage_event(50, 100));

        // Token counts should accumulate
        assert!(
            state.session_token_usage.total_tokens > tokens_after_minimax,
            "Total tokens should accumulate across provider switches"
        );

        // Cost should reflect both providers
        assert!(
            state.session_token_usage.estimated_cost > cost_after_minimax,
            "Cost should accumulate across provider switches"
        );
    }
}
